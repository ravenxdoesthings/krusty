#[derive(Clone)]
pub struct Cache {
    client: redis::Client,
    url: String,
}

/**
 * Keys used in Redict for filter persistence
 * 
 * FILTER_SET_LIST_KEY: This is the key of a set that contains all filter sets
 * FILTER_SET_TEMPLATE: This is a template for the key of a specific filter set, where {} is replaced by the channel ID
 * 
 * In order to store filter sets by channels we'll have to normalize from their current structure which
 * has multiple channels associated to a filter set.
 */

const FILTER_SET_LIST_KEY: &str = "filter_sets_list";
const FILTER_SET_TEMPLATE: &str = "filter_set:{}";

