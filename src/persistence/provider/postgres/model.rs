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

#[derive(Queryable, Insertable, Clone)]
#[diesel(table_name = schema::analytics_killmails)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AnalyticsKillmails {
    pub killmail_id: i64,
    pub killmail_hash: String,
    pub fitted_value: Option<f64>,
    pub destroyed_value: Option<f64>,
    pub dropped_value: Option<f64>,
    pub total_value: Option<f64>,
    pub attacker_count: Option<i32>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}