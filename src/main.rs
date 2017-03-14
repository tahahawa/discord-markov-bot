#[macro_use]
extern crate serenity;
extern crate serde_yaml;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;
extern crate markov;
extern crate typemap;

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;
use typemap::Key;

use r2d2_sqlite::SqliteConnectionManager;

use markov::Chain;

use serenity::client::Client;

pub type SqlitePool = r2d2::Pool<SqliteConnectionManager>;

pub struct Sqlpool;
impl Key for Sqlpool {
    type Value = SqlitePool;
}


fn main() {
    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&mut fstr).unwrap();

    let dbname = config.get("db").unwrap().clone();

    let r2d2_config = r2d2::Config::default();
    let manager = SqliteConnectionManager::new(&dbname);

    let pool = r2d2::Pool::new(r2d2_config, manager).unwrap();
    let conn = pool.get().unwrap();

    conn.execute("CREATE TABLE IF NOT EXISTS messages (
                  id                        TEXT PRIMARY KEY,
                  channel_id        TEXT NOT NULL,
                  author              TEXT NOT NULL,
                  content             TEXT NOT NULL,
                  timestamp       TEXT NOT NULL)",
                 &[])
        .unwrap();

    conn.execute("INSERT or REPLACE INTO messages (id, channel_id, author, content, timestamp) \
                                          VALUES (0, 0, 0, 0, 0)",
                 &[])
        .unwrap();

    println!("pre-init done");

    let mut client = Client::login_bot(&config.get("token").expect("token"));
    client.with_framework(|f| {
                              f
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .on("ping", ping).on("impersonate", impersonate)
                          });

    {
        let mut data = client.data.lock().unwrap();
        data.insert::<Sqlpool>(pool);
    }


    client.on_ready(|_ctx, ready| {
        println!("{} is connected!", ready.user.name);
        println!("{:?}", ready.guilds);
        let mut data = _ctx.data.lock().unwrap();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap();

        //download_all_messages(ready, sql_pool );
    });

    client.on_guild_create(|_ctx, guild| {
                               let mut data = _ctx.data.lock().unwrap();
                               let sql_pool = data.get_mut::<Sqlpool>().unwrap();

                               download_all_messages(guild, sql_pool);
                           });

    client.on_message(|_ctx, message| {
        let mut data = _ctx.data.lock().unwrap();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap();

        insert_into_db(sql_pool,
                       message.id.0.to_string(),
                       message.channel_id.0.to_string(),
                       message.author
                           .id
                           .0
                           .to_string(),
                       message.content,
                       message.timestamp);

        println!("added message on_message: {}", message.id.0.to_string());
    });

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }

}

command!(ping(_context, message) {
    let _ = message.reply("Pong!");
});

command!(impersonate(_context, message) {
    if message.mentions.len() > 0 {
        let ref user = message.mentions[0];
        let mut chain: Chain<String> = Chain::new();

        let mut data = _context.data.lock().unwrap();
        let mut pool = data.get_mut::<Sqlpool>().unwrap();
        let conn = pool.get().unwrap();

        let mut stmt = conn.prepare("SELECT * FROM messages where author = :id and content not like '%~impersonate%' and content not like '%~ping%' " ).unwrap();
        let mut rows = stmt.query_map_named(&[ (":id", &(user.id.0.to_string())) ],  |row| row.get(3)).unwrap();

        for content in rows {
            let dbstr: String = content.unwrap();
            let chainstr: &str = &*dbstr;
            chain.feed_str(chainstr);
            }

        let _ = message.reply(&chain.generate_str());

    }
    else {
        let _ = message.reply("No mention found");
    }
});


fn download_all_messages(guild: serenity::model::Guild,
                         ref pool: &r2d2::Pool<SqliteConnectionManager>) {
    for chan in guild.get_channels().unwrap() {

        let mut _messages = Vec::new();
        let channel_id = (chan.0).0;

        println!("{:?}", chan.1.name);
        println!("");
        println!("");

        if chan.1.bitrate != None {
            continue;
        }

        let conn = pool.get().unwrap();

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
            let try = chan.0.get_messages(|g| g.after(0).limit(100));
            match try {
                Err(try) => println!("error getting messages"),
                _ => _messages = try.unwrap(),
            }
        } else {
            let try =
                chan.0.get_messages(|g| g.after(serenity::model::MessageId(id as u64)).limit(100));

            match try {
                Err(try) => println!("error getting messages"),
                _ => _messages = try.unwrap(),
            }
        }

        while !_messages.is_empty() && _messages.len() > 0 {
            let message_vec = _messages.to_vec();
            for message in message_vec {
                
                insert_into_db(pool,
                               message.id.0.to_string(),
                               message.channel_id.0.to_string(),
                               message.author
                                   .id
                                   .0
                                   .to_string(),
                               message.content,
                               message.timestamp);

                //println!("{:?}", message);
            }

            let id2 = get_latest_id_for_channel(channel_id, pool);

            if id2 == 0 {
                //println!("no message ID");
                let try = chan.0.get_messages(|g| g.after(0).limit(100));
                match try {
                    Err(try) => println!("error getting messages"),
                    _ => _messages = try.unwrap(),
                }
            } else if id2 >= biggest_id {
                break;
            } else {
                let try = chan.0.get_messages(|g| {
                                                  g.after(serenity::model::MessageId(id2 as u64))
                                                      .limit(100)
                                              });

                match try {
                    Err(try) => println!("error getting messages"),
                    _ => _messages = try.unwrap(),
                }

                //println!("id2: {:?}", id2);
                //println!("{:?}", _messages);
            }
        }
    }
    println!("Downloaded all messages for {:?}", guild);
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
        Result::Ok(biggest_id_row) => true,
        Result::Err(biggest_id_row) => false,
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

fn insert_into_db(pool: &r2d2::Pool<SqliteConnectionManager>,
                  message_id: String,
                  channel_id: String,
                  message_author: String,
                  message_content: String,
                  message_timestamp: String) {

    let conn = pool.get().unwrap();

    let _ = conn.execute("INSERT or REPLACE INTO messages (id, channel_id, author, content, timestamp) \
                                          VALUES (?1, ?2, ?3, ?4, ?5)",
                         &[&message_id,
                           &(channel_id),
                           &(message_author),
                           &message_content,
                           &message_timestamp]);

}