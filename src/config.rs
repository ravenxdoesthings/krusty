use uuid::Uuid;

use crate::zkb::ChannelConfig;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub queue_id: Option<String>,
    pub redis_url: Option<String>,
    pub filters: Vec<ChannelConfig>,
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
