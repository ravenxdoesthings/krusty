use diesel::prelude::*;
use crate::persistence::provider::postgres::schema;

#[derive(Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = schema::filter_sets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FilterSet {
    pub channel_id: i64,
    pub guild_id: i64,
    pub filters: serde_json::Value,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}