// @generated automatically by Diesel CLI.

diesel::table! {
    analytics_killmails (killmail_id, killmail_hash) {
        killmail_id -> Int8,
        killmail_hash -> Text,
        fitted_value -> Nullable<Float8>,
        destroyed_value -> Nullable<Float8>,
        dropped_value -> Nullable<Float8>,
        total_value -> Nullable<Float8>,
        attacker_count -> Nullable<Int4>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    filter_sets (channel_id) {
        channel_id -> Int8,
        guild_id -> Int8,
        filters -> Jsonb,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(analytics_killmails, filter_sets,);
