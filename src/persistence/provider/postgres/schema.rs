// @generated automatically by Diesel CLI.

diesel::table! {
    filter_sets (channel_id) {
        channel_id -> Int8,
        guild_id -> Int8,
        filters -> Jsonb,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}
