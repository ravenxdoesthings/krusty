pub mod model;
pub mod schema;
use diesel::OptionalExtension;
use diesel::dsl::insert_into;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, r2d2::ConnectionManager};
use r2d2::Pool;

use crate::persistence::provider::postgres::model::FilterSet as DbFilterSet;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct Store {
    pool: DbPool,
}

use crate::filters::FilterSet;

impl Store {
    pub fn new(pg_url: &str) -> Result<Self, anyhow::Error> {
        let manager = ConnectionManager::<PgConnection>::new(pg_url);
        let pool = Pool::builder().build(manager)?;
        Ok(Self { pool })
    }
}

impl crate::persistence::Store for Store {
    fn get_channel_filter_set(&self, id: u64) -> Result<FilterSet, anyhow::Error> {
        use crate::persistence::provider::postgres::schema::filter_sets::dsl::*;

        tracing::debug!(
            channel_id = id,
            provider = "postgres",
            "getting filter set for channel"
        );

        let result: DbFilterSet = filter_sets
            .filter(channel_id.eq(id as i64))
            .first(&mut self.pool.get()?)?;
        let channel_filters: Vec<String> = serde_json::from_value(result.filters)?;

        Ok(FilterSet {
            guild_id: result.guild_id as u64,
            channel_id: result.channel_id as u64,
            filters: channel_filters,
        })
    }

    fn list_filter_sets(&self) -> Result<Vec<FilterSet>, anyhow::Error> {
        use crate::persistence::provider::postgres::schema::filter_sets::dsl::*;

        tracing::trace!(provider = "postgres", "listing all filter sets");

        let result: Vec<DbFilterSet> = filter_sets.load(&mut self.pool.get()?)?;

        let mut filter_sets_vec = Vec::new();
        for fs in result {
            let channel_filters: Vec<String> = serde_json::from_value(fs.filters)?;
            filter_sets_vec.push(FilterSet {
                guild_id: fs.guild_id as u64,
                channel_id: fs.channel_id as u64,
                filters: channel_filters,
            });
        }

        Ok(filter_sets_vec)
    }

    fn set_filter_set(&self, filter_set: FilterSet) -> Result<(), anyhow::Error> {
        use crate::persistence::provider::postgres::schema::filter_sets::dsl::*;

        tracing::trace!(?filter_set, provider = "postgres", "setting filter set");

        let new_db_filter_set = DbFilterSet {
            channel_id: filter_set.channel_id as i64,
            guild_id: filter_set.guild_id as i64,
            filters: serde_json::to_value(&filter_set.filters)?,
            created_at: Some(chrono::Utc::now().naive_utc()),
            updated_at: Some(chrono::Utc::now().naive_utc()),
        };

        insert_into(filter_sets)
            .values(&new_db_filter_set)
            .on_conflict(channel_id)
            .do_update()
            .set((
                filters.eq(new_db_filter_set.filters.clone()),
                updated_at.eq(new_db_filter_set.updated_at),
            ))
            .execute(&mut self.pool.get()?)?;

        Ok(())
    }

    fn add_filter_to_set(
        &self,
        guild_id: u64,
        channel_id: u64,
        filter: &str,
    ) -> Result<(), anyhow::Error> {
        tracing::debug!(
            guild_id = guild_id,
            channel_id = channel_id,
            filters = ?filter,
            provider="postgres",
            "adding filter to set"
        );
        use crate::persistence::provider::postgres::schema::filter_sets::dsl;

        let mut existing_filter_set: Option<DbFilterSet> = dsl::filter_sets
            .filter(dsl::channel_id.eq(channel_id as i64))
            .first(&mut self.pool.get()?)
            .optional()?;

        match existing_filter_set {
            Some(ref mut fs) => {
                let mut filters_vec: Vec<String> = serde_json::from_value(fs.filters.clone())?;
                if !filters_vec.contains(&filter.to_string()) {
                    filters_vec.push(filter.to_string());
                    fs.filters = serde_json::to_value(&filters_vec)?;
                    fs.updated_at = Some(chrono::Utc::now().naive_utc());
                }
            }
            None => {
                existing_filter_set = Some(DbFilterSet {
                    channel_id: channel_id as i64,
                    guild_id: guild_id as i64,
                    filters: serde_json::to_value(vec![filter.to_string()])?,
                    created_at: Some(chrono::Utc::now().naive_utc()),
                    updated_at: Some(chrono::Utc::now().naive_utc()),
                });
            }
        }

        let final_filter_set = existing_filter_set.as_ref().unwrap();

        insert_into(dsl::filter_sets)
            .values(final_filter_set)
            .on_conflict(dsl::channel_id)
            .do_update()
            .set((
                dsl::filters.eq(final_filter_set.filters.clone()),
                dsl::updated_at.eq(final_filter_set.updated_at),
            ))
            .execute(&mut self.pool.get()?)?;

        Ok(())
    }

    fn remove_filter_from_set(&self, channel_id: u64, filter: &str) -> Result<(), anyhow::Error> {
        tracing::debug!(
            channel_id,
            filter = filter,
            provider="postgres",
            "removing filter from set"
        );

        use crate::persistence::provider::postgres::schema::filter_sets::dsl;
        let mut existing_filter_set: DbFilterSet = dsl::filter_sets
            .filter(dsl::channel_id.eq(channel_id as i64))
            .first(&mut self.pool.get()?)?;

        let mut filters_vec: Vec<String> = serde_json::from_value(existing_filter_set.filters)?;
        filters_vec.retain(|f| f != filter);
        existing_filter_set.filters = serde_json::to_value(&filters_vec)?;
        existing_filter_set.updated_at = Some(chrono::Utc::now().naive_utc());

        insert_into(dsl::filter_sets)
            .values(&existing_filter_set)
            .on_conflict(dsl::channel_id)
            .do_update()
            .set((
                dsl::filters.eq(existing_filter_set.filters.clone()),
                dsl::updated_at.eq(existing_filter_set.updated_at),
            ))
            .execute(&mut self.pool.get()?)?;

        Ok(())
    }

    fn clear_filter_set(&self, channel_id: u64) -> Result<(), anyhow::Error> {
        tracing::debug!(channel_id, provider="postgres", "clearing filter set");

        use crate::persistence::provider::postgres::schema::filter_sets::dsl;

        diesel::delete(dsl::filter_sets.filter(dsl::channel_id.eq(channel_id as i64)))
            .execute(&mut self.pool.get()?)?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "postgres-tests")]
mod tests {
    use super::*;
    use crate::persistence::Store as StoreTrait;

    // Note: These tests require a running Postgres instance
    // Run with: cargo test --features postgres-tests -- --ignored
    #[test]
    #[ignore]
    fn test_postgres_store() {
        let store = Store::new("postgres://postgres:postgres@127.0.0.1/postgres").expect("Failed to connect to Postgres");
        // Clean up any existing test data
        let _ = store.clear_filter_set(20);

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
        store.add_filter_to_set(1, 20, "filter3").unwrap();
        let filter_set = store.get_channel_filter_set(20).unwrap();
        assert_eq!(
            filter_set.filters,
            vec![
                "filter1".to_string(),
                "filter2".to_string(),
                "filter3".to_string()
            ]
        );

        // Test removing a filter from a set
        store.remove_filter_from_set(20, "filter2").unwrap();
        let filter_set = store.get_channel_filter_set(20).unwrap();
        assert_eq!(
            filter_set.filters,
            vec!["filter1".to_string(), "filter3".to_string()]
        );

        // Test listing filter sets
        let all_filter_sets = store.list_filter_sets().unwrap();
        assert!(all_filter_sets.iter().any(|fs| fs.channel_id == 20));

        // Test clearing filter set
        store.clear_filter_set(20).unwrap();
        assert!(store.get_channel_filter_set(20).is_err());
    }
}
