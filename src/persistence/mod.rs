use crate::filters::FilterSet;

pub mod cache;
pub mod provider;

pub trait Store: Send + Sync {
    fn get_channel_filter_set(&self, channel_id: u64) -> Result<FilterSet, anyhow::Error>;

    fn list_filter_sets(&self) -> Result<Vec<FilterSet>, anyhow::Error>;

    fn set_filter_set(&self, filter_set: FilterSet) -> Result<(), anyhow::Error>;

    // need guild_id as we might have to create a new FilterSet
    fn add_filter_to_set(
        &self,
        guild_id: u64,
        channel_id: u64,
        new_filter: &str,
    ) -> Result<(), anyhow::Error>;

    fn remove_filter_from_set(&self, channel_id: u64, filter: &str) -> Result<(), anyhow::Error>;

    fn clear_filter_set(&self, channel_id: u64) -> Result<(), anyhow::Error>;
}
