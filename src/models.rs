use super::schema::messages;
use chrono::*;

#[derive(Queryable, Insertable, Debug)]
#[table_name = "messages"]

pub struct InsertableMessage<Tz: TimeZone> {
    pub id: i64,
    pub channel_id: i64,
    pub author: i64,
    pub content: String,
    pub timestamp: DateTime<Tz>,
}
