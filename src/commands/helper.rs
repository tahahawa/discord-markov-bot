use serenity;
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;


pub fn get_guild_id_from_chan(chan: serenity::model::Channel) -> serenity::model::GuildId {

    match chan {
        serenity::model::Channel::Guild(guild_channel) => guild_channel.read().unwrap().guild_id,
        _ => serenity::model::GuildId(0),
    }

}

pub fn download_all_messages(
    guild: &serenity::model::Guild,
    pool: &r2d2::Pool<SqliteConnectionManager>,
) {
    for chan in guild.channels().unwrap() {

        let mut _messages = Vec::new();
        let channel_id = (chan.0).0;

        println!("{:?}", chan.1.name);
        println!("");
        println!("");

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
            let try = chan.0.messages(|g| {
                g.after(serenity::model::MessageId(id as u64)).limit(100)
            });

            match try {
                Err(_) => println!("error getting messages"),
                _ => _messages = try.unwrap(),
            }
        }

        while !_messages.is_empty() {
            let _ = chan.0.broadcast_typing();
            println!("storing {} messages from {}", _messages.len(), chan.1.name );
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
                let try = chan.0.messages(|g| {
                    g.after(serenity::model::MessageId(id2 as u64)).limit(100)
                });

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

fn biggest_id_exists_in_db(biggest_id: u64, pool: &r2d2::Pool<SqliteConnectionManager>) -> bool {
    let conn = pool.get().unwrap();

    let biggest_id_row: Result<String, _> =
        conn.query_row("SELECT * FROM messages where id = ?",
                       &[&(biggest_id.to_string())],
                       |row| match row.get(0) {
                           None::<String> => 0.to_string(),
                           _ => row.get(0),
                       });

    match biggest_id_row {
        Result::Ok(_) => true,
        Result::Err(_) => false,
    }
}

fn get_latest_id_for_channel(channel_id: u64, pool: &r2d2::Pool<SqliteConnectionManager>) -> u64 {

    let conn = pool.get().unwrap();

    let row: Result<String, _> = conn.query_row("SELECT MAX(id) FROM messages where channel_id = ?",
                                                &[&channel_id.to_string()],
                                                |row| match row.get(0) {
                                                    None::<String> => 0.to_string(),
                                                    _ => row.get(0),
                                                });

    row.unwrap().parse::<u64>().unwrap()
}

pub fn insert_into_db(
    pool: &r2d2::Pool<SqliteConnectionManager>,
    message_id: &String,
    channel_id: &String,
    message_author: &String,
    message_content: &String,
    message_timestamp: &String,
) {

    let conn = pool.get().unwrap();

    let _ = conn.execute(
        "INSERT or REPLACE INTO messages (id, channel_id, author, content, timestamp) \
                                          VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            message_id,
            (channel_id),
            (message_author),
            message_content,
            message_timestamp,
        ],
    );

}
