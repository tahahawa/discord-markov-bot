use super::schema::messages;

#[derive(Queryable)]
#[derive(Insertable)]
#[table_name = "messages"]

pub struct InsertableMessage {
    pub id: String,
    pub channel_id: String,
    pub author: String,
    pub content: String,
    pub timestamp: String,
}
