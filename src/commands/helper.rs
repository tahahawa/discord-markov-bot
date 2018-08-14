use diesel;
use diesel::prelude::*;
use models::*;
use schema::messages;
use serenity::model::prelude::*;
use serenity::client::Context;
use diesel::pg::upsert::excluded;
use chrono::*;

use Sqlpool;



pub fn download_all_messages(guild: &Guild, _ctx: &Context) {
    let channels = guild.channels().expect("Channels not found");

    for chan in channels {
        let mut _messages = Vec::new();
        let channel_id = (chan.0).0 as i64;

        println!("{:?}", chan.1.name);
        println!();
        println!();

        if chan.1.bitrate != None {
            continue;
        }

        let biggest_id = chan.1.last_message_id;

        if biggest_id == None {
            println!("skipped, no latest message exists");
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
            let try = chan.0.messages(|g| g.after(0).limit(100));
            match try {
                Err(_) => println!("error getting messages"),
                _ => _messages = try.unwrap(),
            }
        } else {
            let try = chan
                .0
                .messages(|g| g.after(MessageId(id as u64)).limit(100));

            match try {
                Err(_) => println!("error getting messages"),
                _ => _messages = try.unwrap(),
            }
        }

        while !_messages.is_empty() {
            let _ = chan.0.broadcast_typing();
            println!(
                "storing {} messages from #{} on {}",
                _messages.len(),
                chan.1.name,
                guild.name
            );
            let message_vec = _messages.to_vec();
            let mut transformed_message_vec = Vec::new();
            for message in message_vec {
                let vals = InsertableMessage{
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
                let try = chan.0.messages(|g| g.after(0).limit(100));
                match try {
                    Err(_) => println!("error getting messages"),
                    _ => _messages = try.unwrap(),
                }
            } else if id2 >= biggest_id {
                break;
            } else {
                let try = chan
                    .0
                    .messages(|g| g.after(MessageId(id2 as u64)).limit(100));

                match try {
                    Err(_) => println!("error getting messages"),
                    _ => _messages = try.unwrap(),
                }

                //println!("id2: {:?}", id2);
                //println!("{:?}", _messages);
            }
        }
    }
    println!("Downloaded all messages for {:?}", guild.name);
}

fn biggest_id_exists_in_db(biggest_id: i64, _ctx: &Context) -> bool {
    let conn;

    {
    let mut data = _ctx.data.lock();
    let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();
    
    conn = sql_pool.get().unwrap();
    }

    
    use schema::messages;
    use schema::messages::dsl::*;

    let biggest_id_db_vec = messages::table
        .order(id.desc())
        .select(id)
        .limit(1)
        .filter(id.eq(biggest_id as i64))
        .load::<Option<i64>>(&conn)
        .expect("Error loading biggest id");

    if biggest_id_db_vec.is_empty() {
        return false;
    } else {
        return true;
    }
}

fn get_latest_id_for_channel(chan_id: i64, _ctx: &Context) -> i64 {
    let conn;
    
    {
    let mut data = _ctx.data.lock();
    let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();
    
    conn = sql_pool.get().unwrap();
    }

    use schema::messages;
    use schema::messages::dsl::*;

    let mut chan_id_vec = messages::table
        .order(id.desc())
        .select(id)
        .limit(1)
        .filter(channel_id.eq(chan_id as i64))
        .load::<Option<i64>>(&conn)
        .unwrap_or(vec![Some(0)]);

    if chan_id_vec.is_empty() {
        return 0;
    }

    let latest_chan_id = chan_id_vec
        .pop()
        .unwrap()
        .unwrap_or(0);

    latest_chan_id
}

pub fn insert_into_db(_ctx: &Context, message_vec: &Vec<InsertableMessage<FixedOffset>>) {
    let conn;
    
    {
    let mut data = _ctx.data.lock();
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
