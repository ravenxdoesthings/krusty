use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::filters::FilterSet;

type FilterSetMap = Arc<RwLock<HashMap<u64, FilterSet>>>;

#[derive(Clone, Debug)]
pub struct Store {
    filter_sets: FilterSetMap,
}

impl Store {
    pub fn new() -> Self {
        Self {
            filter_sets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::persistence::Store for Store {
    fn get_channel_filter_set(&self, channel_id: u64) -> Result<FilterSet, anyhow::Error> {
        tracing::debug!(channel_id, "getting filter set for channel");
        if let Ok(filters_sets) = self.filter_sets.read() {
            match filters_sets.get(&channel_id).cloned() {
                Some(filter_set) => Ok(filter_set),
                None => Err(anyhow::anyhow!(
                    "filter set not found for channel {}",
                    channel_id
                )),
            }
        } else {
            Err(anyhow::anyhow!("failed to acquire read lock"))
        }
    }

    fn list_filter_sets(&self) -> Result<Vec<FilterSet>, anyhow::Error> {
        tracing::trace!("listing all filter sets");
        if let Ok(filter_sets) = self.filter_sets.read() {
            Ok(filter_sets.values().cloned().collect())
        } else {
            Err(anyhow::anyhow!("failed to acquire read lock"))
        }
    }

    fn set_filter_set(&self, filter_set: FilterSet) -> Result<(), anyhow::Error> {
        tracing::trace!(?filter_set, "setting filter set");

        if let Ok(mut filters) = self.filter_sets.write() {
            filters.insert(filter_set.channel_id, filter_set);

            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to acquire write lock"))
        }
    }

    fn add_filter_to_set(
        &self,
        guild_id: u64,
        channel_id: u64,
        filter: &str,
    ) -> Result<(), anyhow::Error> {
        tracing::debug!(guild_id = guild_id, channel_id = channel_id, filters = ?filter, "adding filter to set");
        if let Ok(mut filters_sets) = self.filter_sets.write() {
            let filter_set = filters_sets.entry(channel_id).or_insert_with(|| FilterSet {
                channel_id,
                guild_id,
                filters: Vec::new(),
            });
            filter_set.filters.push(filter.to_string());
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to acquire write lock"))
        }
    }

    fn remove_filter_from_set(&self, channel_id: u64, filter: &str) -> Result<(), anyhow::Error> {
        tracing::debug!(channel_id, filter = filter, "removing filter from set");
        if let Ok(mut filters_sets) = self.filter_sets.write() {
            if let Some(filter_set) = filters_sets.get_mut(&channel_id) {
                filter_set.filters.retain(|f| f != filter);
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "filter set not found for channel {}",
                    channel_id
                ))
            }
        } else {
            Err(anyhow::anyhow!("failed to acquire write lock"))
        }
    }

    fn clear_filter_set(&self, channel_id: u64) -> Result<(), anyhow::Error> {
        tracing::debug!(channel_id, "clearing filter set");
        if let Ok(mut filters_sets) = self.filter_sets.write() {
            filters_sets.remove(&channel_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to acquire write lock"))
        }
    }

    fn add_analytics_data(&self, km: &crate::zkb::Zkb) -> Result<(), anyhow::Error> {
        tracing::debug!(?km, "adding analytics data to memory (no-op)");

        // No-op for Redis store
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::persistence::Store;

    use super::*;

    #[test]
    fn test_store() {
        let store = super::Store::new();

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
        store
            .add_filter_to_set(1, 20, &"filter3".to_string())
            .unwrap();
        let filter_set = store.get_channel_filter_set(20).unwrap();
        assert_eq!(
            filter_set.filters,
            vec![
                "filter1".to_string(),
                "filter2".to_string(),
                "filter3".to_string()
            ]
        );

        // Test listing filter sets
        let all_filter_sets = store.list_filter_sets().unwrap();
        assert_eq!(all_filter_sets.len(), 1);
        assert_eq!(
            all_filter_sets[0],
            FilterSet {
                guild_id: 1,
                channel_id: 20,
                filters: vec![
                    "filter1".to_string(),
                    "filter2".to_string(),
                    "filter3".to_string()
                ]
            }
        );
    }
}
