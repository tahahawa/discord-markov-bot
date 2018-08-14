table! {
    messages (id) {
        id -> Nullable<Int8>,
        channel_id -> Int8,
        author -> Int8,
        content -> Text,
        timestamp -> Timestamptz,
    }
}
