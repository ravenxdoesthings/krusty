use redis::{Client, Commands, Connection};
use std::sync::{Arc, Mutex};

use crate::filters::FilterSet;

const FILTER_SET_PREFIX: &str = "krusty:filter_set:channel:";
const FILTER_SET_INDEX_KEY: &str = "krusty:filter_set:index";

#[derive(Clone)]
pub struct Store {
    #[allow(dead_code)] // Kept for potential reconnection logic
    client: Arc<Client>,
    connection: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn new(redis_url: &str) -> Result<Self, anyhow::Error> {
        let client = Client::open(redis_url)?;
        let connection = client.get_connection()?;
        Ok(Self {
            client: Arc::new(client),
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn get_key(channel_id: u64) -> String {
        format!("{}{}", FILTER_SET_PREFIX, channel_id)
    }
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("client", &"<redis::Client>")
            .finish()
    }
}

impl crate::persistence::Store for Store {
    fn get_channel_filter_set(&self, channel_id: u64) -> Result<FilterSet, anyhow::Error> {
        tracing::debug!(channel_id, "getting filter set for channel from redis");

        let mut conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to acquire connection lock"))?;

        let key = Self::get_key(channel_id);
        let data: Option<String> = conn.get(&key)?;

        match data {
            Some(json) => {
                let filter_set: FilterSet = simd_json::from_slice(&mut json.into_bytes())?;
                Ok(filter_set)
            }
            None => Err(anyhow::anyhow!(
                "filter set not found for channel {}",
                channel_id
            )),
        }
    }

    fn get_guild_filter_set(&self, guild_id: u64) -> Result<FilterSet, anyhow::Error> {
        tracing::debug!(guild_id, "getting filter set for guild from redis");

        // For guild filter sets, we need to iterate through all filter sets and find the one with the matching guild_id
        // This is less efficient than the memory implementation, but maintains the same interface
        let all_sets = self.list_filter_sets()?;

        all_sets
            .into_iter()
            .find(|fs| fs.guild_id == guild_id)
            .ok_or_else(|| anyhow::anyhow!("filter set not found for guild {}", guild_id))
    }

    fn list_filter_sets(&self) -> Result<Vec<FilterSet>, anyhow::Error> {
        tracing::trace!("listing all filter sets from redis");

        let mut conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to acquire connection lock"))?;

        // Get all channel IDs from the index
        let channel_ids: Vec<u64> = conn.smembers(FILTER_SET_INDEX_KEY)?;

        let mut filter_sets = Vec::new();
        for channel_id in channel_ids {
            let key = Self::get_key(channel_id);
            let data: Option<String> = conn.get(&key)?;
            if let Some(json) = data {
                let filter_set: FilterSet = simd_json::from_slice(&mut json.into_bytes())?;
                filter_sets.push(filter_set);
            }
        }

        Ok(filter_sets)
    }

    fn set_filter_set(&self, filter_set: FilterSet) -> Result<(), anyhow::Error> {
        tracing::trace!(?filter_set, "setting filter set in redis");

        let mut conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to acquire connection lock"))?;

        let key = Self::get_key(filter_set.channel_id);
        let json = simd_json::to_string(&filter_set)?;

        // Store the filter set
        let _: () = conn.set(&key, &json)?;

        // Add to the index set
        let _: usize = conn.sadd(FILTER_SET_INDEX_KEY, filter_set.channel_id)?;

        Ok(())
    }

    fn add_filter_to_set(
        &self,
        guild_id: u64,
        channel_id: u64,
        filter: &str,
    ) -> Result<(), anyhow::Error> {
        tracing::debug!(
            guild_id = guild_id,
            channel_id = channel_id,
            filters = ?filter,
            "adding filter to set in redis"
        );

        let mut conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to acquire connection lock"))?;

        let key = Self::get_key(channel_id);
        let data: Option<String> = conn.get(&key)?;

        let mut filter_set = match data {
            Some(json) => simd_json::from_slice(&mut json.into_bytes())?,
            None => FilterSet {
                channel_id,
                guild_id,
                filters: Vec::new(),
            },
        };

        filter_set.filters.push(filter.to_string());

        let json = simd_json::to_string(&filter_set)?;
        let _: () = conn.set(&key, &json)?;

        // Add to the index set (in case it's a new filter set)
        let _: usize = conn.sadd(FILTER_SET_INDEX_KEY, channel_id)?;

        Ok(())
    }

    fn remove_filter_from_set(&self, channel_id: u64, filter: &str) -> Result<(), anyhow::Error> {
        tracing::debug!(
            channel_id,
            filter = filter,
            "removing filter from set in redis"
        );

        let mut conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to acquire connection lock"))?;

        let key = Self::get_key(channel_id);
        let data: Option<String> = conn.get(&key)?;

        match data {
            Some(json) => {
                let mut filter_set: FilterSet = simd_json::from_slice(&mut json.into_bytes())?;
                filter_set.filters.retain(|f| f != filter);

                let json = simd_json::to_string(&filter_set)?;
                let _: () = conn.set(&key, &json)?;

                Ok(())
            }
            None => Err(anyhow::anyhow!(
                "filter set not found for channel {}",
                channel_id
            )),
        }
    }

    fn clear_filter_set(&self, channel_id: u64) -> Result<(), anyhow::Error> {
        tracing::debug!(channel_id, "clearing filter set from redis");

        let mut conn = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to acquire connection lock"))?;

        let key = Self::get_key(channel_id);

        // Remove the filter set
        let _: usize = conn.del(&key)?;

        // Remove from the index set
        let _: usize = conn.srem(FILTER_SET_INDEX_KEY, channel_id)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::Store as StoreTrait;

    // Note: These tests require a running Redis instance
    // Run with: cargo test --features redis-tests -- --ignored
    #[test]
    #[ignore]
    fn test_redis_store() {
        let store = Store::new("redis://127.0.0.1:6379").expect("Failed to connect to Redis");

        // Clean up any existing test data
        let _ = store.clear_filter_set(20);

        // Test setting and getting filter sets
        store
            .set_filter_set(FilterSet {
                guild_id: 1,
                channel_id: 20,
                filters: vec!["filter1".to_string(), "filter2".to_string()],
            })
            .unwrap();

        let filter_set = store.get_channel_filter_set(20).unwrap();
        assert_eq!(
            filter_set.filters,
            vec!["filter1".to_string(), "filter2".to_string()]
        );

        // Test adding a filter to a set
        store.add_filter_to_set(1, 20, "filter3").unwrap();
        let filter_set = store.get_channel_filter_set(20).unwrap();
        assert_eq!(
            filter_set.filters,
            vec![
                "filter1".to_string(),
                "filter2".to_string(),
                "filter3".to_string()
            ]
        );

        // Test removing a filter from a set
        store.remove_filter_from_set(20, "filter2").unwrap();
        let filter_set = store.get_channel_filter_set(20).unwrap();
        assert_eq!(
            filter_set.filters,
            vec!["filter1".to_string(), "filter3".to_string()]
        );

        // Test listing filter sets
        let all_filter_sets = store.list_filter_sets().unwrap();
        assert!(all_filter_sets.iter().any(|fs| fs.channel_id == 20));

        // Test clearing filter set
        store.clear_filter_set(20).unwrap();
        assert!(store.get_channel_filter_set(20).is_err());
    }
}
