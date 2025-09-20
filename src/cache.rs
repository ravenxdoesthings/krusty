use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub struct Entry {
    pub expires_at: Option<std::time::Instant>,
}

impl Entry {
    pub fn new(ttl: Option<std::time::Duration>) -> Self {
        if let Some(d) = ttl {
            Self {
                expires_at: Some(std::time::Instant::now() + d),
            }
        } else {
            Self { expires_at: None }
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            std::time::Instant::now() >= expiry
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub entries: Arc<RwLock<HashMap<String, Entry>>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_expiration_task(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            self.expire();
        }
    }

    pub fn get(&self, key: &str) -> Option<Entry> {
        tracing::debug!(key = key, "redict: looking up key");
        let entries = self.entries.read().unwrap();
        entries.get(key).cloned()
    }

    pub fn set(&self, key: String, value: Entry) {
        tracing::debug!(key = key, ttl = ?value.expires_at, "redict: saving value");
        let mut entries = self.entries.write().unwrap();
        entries.insert(key, value);
    }

    pub fn _persist(&self, _connection: ()) -> Result<(), anyhow::Error> {
        // Implement persistence logic here
        Ok(())
    }

    pub fn expire(&self) {
        let mut entries = self.entries.write().unwrap();
        let before = entries.len();
        entries.retain(|_, entry| !entry.is_expired());
        let expired = before - entries.len();
        if expired > 0 {
            tracing::debug!(expired = expired, "expired cache items");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_set_and_get() {
        let cache = Cache::new();
        let entry = Entry::new(None);
        cache.set("foo".to_string(), entry.clone());

        let retrieved = cache.get("foo");
        assert!(retrieved.is_some());
        let retrieved_entry = retrieved.unwrap();
        assert_eq!(retrieved_entry.expires_at, None);
    }

    #[test]
    fn test_cache_get_nonexistent() {
        let cache = Cache::new();
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn test_cache_expire_removes_expired_entries() {
        let cache = Cache::new();
        // Entry that expires immediately
        let entry_expired = Entry::new(Some(Duration::from_millis(1)));
        cache.set("expired".to_string(), entry_expired);

        // Entry that does not expire
        let entry_valid = Entry::new(None);
        cache.set("valid".to_string(), entry_valid.clone());

        // Wait for the expired entry to expire
        std::thread::sleep(Duration::from_millis(10));
        cache.expire();

        // Expired entry should be gone
        assert!(cache.get("expired").is_none());
        // Valid entry should remain
        let valid = cache.get("valid");
        assert!(valid.is_some());
    }

    #[test]
    fn test_entry_is_expired() {
        let entry = Entry::new(Some(Duration::from_millis(1)));
        std::thread::sleep(Duration::from_millis(10));
        assert!(entry.is_expired());

        let entry_no_expiry = Entry::new(None);
        assert!(!entry_no_expiry.is_expired());
    }
}
