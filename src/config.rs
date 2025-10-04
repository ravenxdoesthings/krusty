use uuid::Uuid;

use crate::zkb::Filters;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub channels: Vec<u64>,
    pub queue_id: Option<String>,
    pub filters: Filters,
}

impl Config {
    pub fn load(path: String) -> Self {
        let content = std::fs::read_to_string(path).expect("Failed to read config file");
        serde_json::from_str(&content).expect("Failed to parse config file")
    }

    pub fn queue_id(&self) -> String {
        let id = self
            .queue_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        format!("krusty-{id}")
    }
}
