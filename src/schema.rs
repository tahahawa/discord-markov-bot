table! {
    messages (id) {
        id -> Nullable<Text>,
        channel_id -> Text,
        author -> Text,
        content -> Text,
        timestamp -> Text,
    }
}
