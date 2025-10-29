use uuid::Uuid;

use crate::{filters, zkb};

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub queue_id: Option<String>,
    pub redis_url: Option<String>,
    pub filters: Vec<zkb::ChannelConfig>,

    pub experimental: Option<filters::Config>,
}

impl Config {
    pub fn load(path: String) -> Self {
        let content = std::fs::read_to_string(path).expect("Failed to read config file");
        serde_yaml::from_str(&content).expect("Failed to parse config file")
    }

    pub fn queue_id(&self) -> String {
        let id = self
            .queue_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        format!("krusty-{id}")
    }

    pub fn redis_url(&self) -> String {
        self.redis_url
            .clone()
            .unwrap_or_else(|| "redis://localhost:6379".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let test_queue_id = "test-queue-id";
        let content = format!(
            r#"
---
queue_id: "{test_queue_id}"
filters:
    - channel_ids:
        - 1000
      filters:
        include_npc: true
        corps:
          includes:
            - 2
        alliances:
          includes:
            - 3
"#
        );
        let config = serde_yaml::from_str::<Config>(content.as_str()).unwrap();
        assert_eq!(config.queue_id.unwrap(), test_queue_id.to_string());
        assert_eq!(config.filters.len(), 1);

        let filter = config.filters[0].clone();
        assert!(filter.filters.characters.is_none());
        assert_eq!(filter.filters.corps.clone().unwrap().includes.len(), 1);
        assert_eq!(filter.filters.corps.unwrap().excludes.len(), 0);
        assert_eq!(filter.filters.alliances.clone().unwrap().includes.len(), 1);
        assert_eq!(filter.filters.alliances.unwrap().excludes.len(), 0);
    }
}
