use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

type FilterSetMap = Arc<RwLock<HashMap<u64, Vec<String>>>>;

#[derive(Clone, Debug)]
pub struct Store {
    filters_sets: FilterSetMap,
}

impl Store {
    pub fn new() -> Self {
        Self {
            filters_sets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl super::StoreTrait for Store {
    fn get_filter_set(&self, channel_id: u64) -> Result<Vec<String>, anyhow::Error> {
        if let Ok(filters_sets) = self.filters_sets.read() {
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

    fn list_filter_sets(&self) -> Result<Vec<(u64, Vec<String>)>, anyhow::Error> {
        if let Ok(filters_sets) = self.filters_sets.read() {
            Ok(filters_sets.iter().map(|(k, v)| (*k, v.clone())).collect())
        } else {
            Err(anyhow::anyhow!("failed to acquire read lock"))
        }
    }

    fn set_filter_set(
        &self,
        channel_id: u64,
        filter_set: Vec<String>,
    ) -> Result<(), anyhow::Error> {
        tracing::trace!(channel_id, ?filter_set, "setting filter set");

        if let Ok(mut filters_sets) = self.filters_sets.write() {
            filters_sets.insert(channel_id, filter_set);
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to acquire write lock"))
        }
    }

    fn add_filter_to_set(&self, channel_id: u64, filter: String) -> Result<(), anyhow::Error> {
        tracing::debug!(channel_id, filter = filter.as_str(), "adding filter to set");
        if let Ok(mut filters_sets) = self.filters_sets.write() {
            let filter_set = filters_sets.entry(channel_id).or_insert_with(Vec::new);
            filter_set.push(filter);
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to acquire write lock"))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::persistence::StoreTrait;

    use super::*;

    #[test]
    fn test_store() {
        let store = Store::new();

        // Test setting and getting filter sets
        store
            .set_filter_set(1, vec!["filter1".to_string(), "filter2".to_string()])
            .unwrap();
        let filter_set = store.get_filter_set(1).unwrap();
        assert_eq!(
            filter_set,
            vec!["filter1".to_string(), "filter2".to_string()]
        );

        // Test adding a filter to a set
        store.add_filter_to_set(1, "filter3".to_string()).unwrap();
        let filter_set = store.get_filter_set(1).unwrap();
        assert_eq!(
            filter_set,
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
            (
                1,
                vec![
                    "filter1".to_string(),
                    "filter2".to_string(),
                    "filter3".to_string()
                ]
            )
        );
    }
}
