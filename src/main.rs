#[macro_use]
extern crate serenity;
extern crate serde_yaml;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;
extern crate markov;
extern crate typemap;
extern crate regex;

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;
use typemap::Key;

use r2d2_sqlite::SqliteConnectionManager;

use markov::Chain;

use regex::Regex;

use serenity::client::{Client, Context};
use serenity::model::Message;

pub type SqlitePool = r2d2::Pool<SqliteConnectionManager>;

pub struct Sqlpool;

impl Key for Sqlpool {
    type Value = SqlitePool;
}


fn main() {
    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&fstr).unwrap();

    let dbname = config["db"].clone();

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

    let mut client = Client::login_bot(&config["token"]);
    client.with_framework(|f| {
        f
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .on("ping", ping)
        .on("hivemind", hivemind)
        .command("impersonate", |c| c
        .use_quotes(true)
        .min_args(1)
        .guild_only(true)
        .exec(impersonate))
    });

    {
        let mut data = client.data.lock().unwrap();
        data.insert::<Sqlpool>(pool);
    }


    client.on_ready(|_ctx, ready| {
        println!("{} is connected!", ready.user.name);
        println!("{:?}", ready.guilds);
        //let mut data = _ctx.data.lock().unwrap();
        //let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        //download_all_messages(ready, sql_pool );
    });

    client.on_guild_create(|_ctx, guild| {
                               let mut data = _ctx.data.lock().unwrap();
                               let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

                               download_all_messages(&guild, &sql_pool);
                           });

    client.on_message(|_ctx, message| {
        let mut data = _ctx.data.lock().unwrap();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        insert_into_db(&sql_pool,
                       &message.id.0.to_string(),
                       &message.channel_id.0.to_string(),
                       &message.author
                            .id
                            .0
                            .to_string(),
                       &message.content,
                       &message.timestamp);

        //println!("added message on_message: {}", message.id.0.to_string());
    });

    /*client.on_message_update(|_ctx, message| {
        let mut data = _ctx.data.lock().unwrap();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap();

        insert_into_db(sql_pool,
                       message.id.0.to_string(),
                       message.channel_id.0.to_string(),
                       message.author
                           .unwrap()
                           .id
                           .0
                           .to_string(),
                       message.content.unwrap(),
                       message.timestamp.unwrap());

        //println!("added message on_message_update: {}", message.id.0.to_string());
    });*/


    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded() {
        println!("Client error: {:?}", why);
    }

}

command!(ping(_context, message) {
    let _ = message.reply("Pong!");
});

fn get_guild_id_from_chan(chan: serenity::model::Channel) -> serenity::model::GuildId {

    match chan {
        serenity::model::Channel::Guild(guild_channel) => guild_channel.read().unwrap().guild_id,
        _ => serenity::model::GuildId(0),
    }

}

fn impersonate(_context: &mut Context,
               message: &Message,
               _args: Vec<String>)
               -> Result<(), String> {
    let chan = _context.channel_id
        .unwrap()
        .get()
        .unwrap();

    let re = Regex::new(r"(<@!?\d*>)").unwrap();

    let guild_id = get_guild_id_from_chan(chan);
    let mut user = None;
    let mut offset = 0;
    let mut count = 0;
    'outer: while user.is_none() {
        let members = guild_id.get_members(Some(1000), Some(offset)).unwrap();

        if count == 10 || members.is_empty() {
            break 'outer;
        } else {
            offset = members[0]
                .user
                .read()
                .unwrap()
                .clone()
                .id
                .0;
        }

        for m in members {
            if m.display_name().to_lowercase() == _args[0].to_lowercase() ||
               m.distinct().to_lowercase() == _args[0].to_lowercase() {
                user = Some(m.user
                                .read()
                                .unwrap()
                                .clone());
                break 'outer;
            }
        }

        count += 1;
    }

    let mut data = _context.data.lock().unwrap();
    let pool = data.get_mut::<Sqlpool>().unwrap().clone();
    let conn = pool.get().unwrap();

    if user.is_some() && _args.len() > 1 {
        let user = user.unwrap();
        let mut chain: Chain<String> = Chain::new();

        let mut stmt = conn.prepare("SELECT * FROM messages where author = :id and content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' " ).unwrap();
        let rows = stmt.query_map_named(&[(":id", &(user.id.0.to_string()))], |row| row.get(3))
            .unwrap();

        let mut messages = Vec::<String>::new();
        for content in rows {
            messages.push(content.unwrap());
        }

        if !messages.is_empty() {

            for m in messages {
                chain.feed_str(&m);
            }

            let re_iter = Regex::new(r"\D").unwrap();
            let iter_test = re_iter.replace_all(&_args[1], "");

            let iter: usize = if !iter_test.is_empty() {
                iter_test.parse::<usize>().unwrap()
            } else {
                1
            };

            let mut msg = String::new();

            for line in chain.str_iter_for(iter) {
                msg = msg + "\n" + &line;
                //println!("{}", line);
            }

            let _ = message.reply(&re.replace_all(&msg, "@mention").into_owned());
        } else {
            let _ = message.reply("They haven't said anything");
        }

    } else if user.is_some() {
        let user = user.unwrap();
        let mut chain: Chain<String> = Chain::new();

        let mut stmt = conn.prepare("SELECT * FROM messages where author = :id and content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' " ).unwrap();
        let rows = stmt.query_map_named(&[(":id", &(user.id.0.to_string()))], |row| row.get(3))
            .unwrap();

        let mut messages = Vec::<String>::new();
        for content in rows {
            messages.push(content.unwrap());
        }

        if !messages.is_empty() {
            for m in messages {
                chain.feed_str(&m);
            }
            let _ = message.reply(&re.replace_all(&chain.generate_str(), "@mention").into_owned());
        } else {
            let _ = message.reply("They haven't said anything");
        }
    } else {
        let _ = message.reply("No user found");
    }
    Ok(())

}


fn download_all_messages(guild: &serenity::model::Guild,
                         pool: &r2d2::Pool<SqliteConnectionManager>) {
    for chan in guild.get_channels().unwrap() {

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
            let try = chan.0.get_messages(|g| g.after(0).limit(100));
            match try {
                Err(_) => println!("error getting messages"),
                _ => _messages = try.unwrap(),
            }
        } else {
            let try =
                chan.0.get_messages(|g| g.after(serenity::model::MessageId(id as u64)).limit(100));

            match try {
                Err(_) => println!("error getting messages"),
                _ => _messages = try.unwrap(),
            }
        }

        while !_messages.is_empty() && !_messages.is_empty() {
            let message_vec = _messages.to_vec();
            for message in message_vec {

                insert_into_db(pool,
                               &message.id.0.to_string(),
                               &message.channel_id.0.to_string(),
                               &message.author
                                    .id
                                    .0
                                    .to_string(),
                               &message.content,
                               &message.timestamp);

                //println!("{:?}", message);
            }

            let id2 = get_latest_id_for_channel(channel_id, pool);

            if id2 == 0 {
                //println!("no message ID");
                let try = chan.0.get_messages(|g| g.after(0).limit(100));
                match try {
                    Err(_) => println!("error getting messages"),
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

fn insert_into_db(pool: &r2d2::Pool<SqliteConnectionManager>,
                  message_id: &String,
                  channel_id: &String,
                  message_author: &String,
                  message_content: &String,
                  message_timestamp: &String) {

    let conn = pool.get().unwrap();

    let _ = conn.execute("INSERT or REPLACE INTO messages (id, channel_id, author, content, timestamp) \
                                          VALUES (?1, ?2, ?3, ?4, ?5)",
                         &[message_id,
                           (channel_id),
                           (message_author),
                           message_content,
                           message_timestamp]);

}

command!(hivemind(_context, message) {
    let re = Regex::new(r"(<@!?\d*>)").unwrap();

        let mut chain: Chain<String> = Chain::new();

        let mut data = _context.data.lock().unwrap();
        let pool = data.get_mut::<Sqlpool>().unwrap().clone();
        let conn = pool.get().unwrap();

        let mut stmt = conn.prepare("SELECT * FROM messages where content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' " ).unwrap();
        let rows = stmt.query_map_named(&[], |row| row.get(3))
            .unwrap();

        let mut messages = Vec::<String>::new();
        for content in rows {
            messages.push(content.unwrap());
        }

        if messages.len() > 0 {
            for m in messages {
                chain.feed_str(&m);
            }
            let _ = message.reply(&re.replace_all(&chain.generate_str(), "@mention").into_owned());
        } else {
            let _ = message.reply("They haven't said anything");
        }
});