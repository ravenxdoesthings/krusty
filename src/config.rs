use crate::zkb::Filters;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub channels: Vec<u64>,
    pub filters: Filters,
}

impl Config {
    pub fn load(path: String) -> Self {
        let content = std::fs::read_to_string(path).expect("Failed to read config file");
        serde_json::from_str(&content).expect("Failed to parse config file")
    }
}
