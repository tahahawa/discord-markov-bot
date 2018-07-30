use diesel::prelude::*;
use diesel::r2d2::*;
use serenity::model::prelude::*;

pub fn download_all_messages(guild: &Guild, pool: &Pool<ConnectionManager<SqliteConnection>>) {
    for chan in guild.channels().unwrap() {
        let mut _messages = Vec::new();
        let channel_id = (chan.0).0;

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

        let biggest_id = biggest_id.unwrap().0;
        //println!("biggest ID: {}", biggest_id);

        if biggest_id_exists_in_db(biggest_id, pool) {
            continue;
        }

        let id = get_latest_id_for_channel(channel_id, pool);

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
            for message in message_vec {
                insert_into_db(
                    pool,
                    &message.id.0.to_string(),
                    &message.channel_id.0.to_string(),
                    &message.author.id.0.to_string(),
                    &message.content,
                    &message.timestamp.to_string(),
                );

                //println!("{:?}", message);
            }

            let id2 = get_latest_id_for_channel(channel_id, pool);

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

fn biggest_id_exists_in_db(
    biggest_id: u64,
    pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> bool {
    let conn = pool.get().unwrap();

    use schema::messages;
    use schema::messages::dsl::*;

    let biggest_id_db_vec = messages::table
        .order(id.desc())
        .select(id)
        .limit(1)
        .filter(id.eq(biggest_id.to_string()))
        .load::<Option<String>>(&conn)
        .expect("Error loading biggest id");

    if biggest_id_db_vec.is_empty() {
        return false;
    } else {
        return true;
    }
}

fn get_latest_id_for_channel(
    chan_id: u64,
    pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> u64 {
    let conn = pool.get().unwrap();

    use schema::messages;
    use schema::messages::dsl::*;

    let mut chan_id_vec = messages::table
        .order(id.desc())
        .select(id)
        .limit(1)
        .filter(channel_id.eq(chan_id.to_string()))
        .load::<Option<String>>(&conn)
        .unwrap_or(vec![Some("0".to_owned())]);

    if chan_id_vec.is_empty() {
        return 0;
    }

    let latest_chan_id = chan_id_vec
        .pop()
        .unwrap()
        .unwrap()
        .parse::<u64>()
        .unwrap_or(0);

    latest_chan_id
}

pub fn insert_into_db(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    message_id: &String,
    chan_id: &String,
    message_author: &String,
    message_content: &String,
    message_timestamp: &String,
) {
    use diesel;
    use models::*;
    use schema::messages;

    let conn = pool.get().unwrap();

    let vals = InsertableMessage {
        id: message_id.to_string(),
        channel_id: chan_id.to_string(),
        author: message_author.to_string(),
        content: message_content.to_string(),
        timestamp: message_timestamp.to_string(),
    };

    let _ = diesel::replace_into(messages::table)
        .values(&vals)
        .execute(&conn)
        .expect("Error inserting values");

    // let _ = conn.execute(
    //     "INSERT or REPLACE INTO messages (id, channel_id, author, content, timestamp) \
    //      VALUES (?1, ?2, ?3, ?4, ?5)",
    //     &[
    //         message_id,
    //         (channel_id),
    //         (message_author),
    //         message_content,
    //         message_timestamp,
    //     ],
    // );
}
