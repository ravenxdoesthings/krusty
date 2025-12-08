pub mod cache;
// pub mod filters;

pub mod memory;

pub trait StoreTrait: Send + Sync {
    fn get_filter_set(&self, channel_id: u64) -> Result<Vec<String>, anyhow::Error>;

    fn list_filter_sets(&self) -> Result<Vec<(u64, Vec<String>)>, anyhow::Error>;

    fn set_filter_set(&self, channel_id: u64, filter_set: Vec<String>)
    -> Result<(), anyhow::Error>;

    fn add_filter_to_set(&self, channel_id: u64, filter: String) -> Result<(), anyhow::Error>;
}
