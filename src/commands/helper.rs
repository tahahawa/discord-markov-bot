use chrono::*;
use diesel;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use crate::models::*;
use crate::schema::messages;
use serenity::client::Context;
use serenity::model::prelude::*;

use crate::Sqlpool;

pub fn download_all_messages(guild: &Guild, _ctx: &Context) {
    let channels = guild.channels(&_ctx.http).expect("Channels not found");

    for chan in channels {
        let mut _messages = Vec::new();
        let channel_id = (chan.0).0 as i64;

        info!("{:?}", chan.1.name);

        if chan.1.bitrate != None {
            continue;
        }

        let biggest_id = chan.1.last_message_id;

        if biggest_id == None {
            info!("skipped, no latest message exists");
            continue;
        }

        let biggest_id = biggest_id.expect("Biggest ID = None").0 as i64;
        //println!("biggest ID: {}", biggest_id);

        if biggest_id_exists_in_db(biggest_id, _ctx) {
            continue;
        }

        let id = get_latest_id_for_channel(channel_id, _ctx);

        if id == 0 {
            //println!("no message ID");
            let r#try = chan.0.messages(&_ctx.http, |g| g.after(0).limit(100));
            match r#try {
                Err(_) => warn!("error getting messages"),
                _ => _messages = r#try.unwrap(),
            }
        } else {
            let r#try = chan
                .0
                .messages(&_ctx.http, |g| g.after(MessageId(id as u64)).limit(100));

            match r#try {
                Err(_) => warn!("error getting messages"),
                _ => _messages = r#try.unwrap(),
            }
        }

        while !_messages.is_empty() {
            let _ = chan.0.broadcast_typing(&_ctx.http);
            info!(
                "storing {} messages from #{} on {}",
                _messages.len(),
                chan.1.name,
                guild.name
            );
            let message_vec = _messages.to_vec();
            let mut transformed_message_vec = Vec::new();
            for message in message_vec {
                let vals = InsertableMessage {
                    id: message.id.0 as i64,
                    channel_id: message.channel_id.0 as i64,
                    author: message.author.id.0 as i64,
                    content: message.content,
                    timestamp: message.timestamp,
                };

                transformed_message_vec.push(vals);
                //println!("{:?}", message);
            }

            insert_into_db(_ctx, &transformed_message_vec);

            let id2 = get_latest_id_for_channel(channel_id, _ctx);

            if id2 == 0 {
                //println!("no message ID");
                let r#try = chan.0.messages(&_ctx.http,|g| g.after(0).limit(100));
                match r#try {
                    Err(_) => warn!("error getting messages"),
                    _ => _messages = r#try.unwrap(),
                }
            } else if id2 >= biggest_id {
                break;
            } else {
                let r#try = chan
                    .0
                    .messages(&_ctx.http,|g| g.after(MessageId(id2 as u64)).limit(100));

                match r#try {
                    Err(_) => warn!("error getting messages"),
                    _ => _messages = r#try.unwrap(),
                }

                //println!("id2: {:?}", id2);
                //println!("{:?}", _messages);
            }
        }
    }
    info!("Downloaded all messages for {:?}", guild.name);
}

fn biggest_id_exists_in_db(biggest_id: i64, _ctx: &Context) -> bool {
    let conn;

    {
        let mut data = _ctx.data.write();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        conn = sql_pool.get().unwrap();
    }

    use crate::schema::messages;
    use crate::schema::messages::dsl::*;

    let biggest_id_db_vec = messages::table
        .order(id.desc())
        .select(id)
        .limit(1)
        .filter(id.eq(biggest_id as i64))
        .load::<Option<i64>>(&conn)
        .expect("Error loading biggest id");

    !biggest_id_db_vec.is_empty()
}

fn get_latest_id_for_channel(chan_id: i64, _ctx: &Context) -> i64 {
    let conn;

    {
        let mut data = _ctx.data.write();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        conn = sql_pool.get().unwrap();
    }

    use crate::schema::messages;
    use crate::schema::messages::dsl::{channel_id, id};

    let mut chan_id_vec = messages::table
        .order(id.desc())
        .select(id)
        .limit(1)
        .filter(channel_id.eq(chan_id as i64))
        .load::<Option<i64>>(&conn)
        .unwrap_or_default();

    if chan_id_vec.is_empty() {
        return 0;
    }

    chan_id_vec.pop().unwrap().unwrap_or(0)
}

pub fn insert_into_db(_ctx: &Context, message_vec: &[InsertableMessage<FixedOffset>]) {
    let conn;

    {
        let mut data = _ctx.data.write();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        conn = sql_pool.get().unwrap();
    }

    let _ = diesel::insert_into(messages::table)
        .values(message_vec)
        .on_conflict(messages::id)
        .do_update()
        .set(messages::content.eq(excluded(messages::content)))
        .execute(&conn)
        .expect("Error inserting values");
}
