extern crate serenity;
extern crate serde_yaml;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;
extern crate markov;
extern crate typemap;
extern crate regex;

mod commands;

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;
use typemap::Key;

use r2d2_sqlite::SqliteConnectionManager;

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
        .on("ping", commands::meta::ping)
        .command("hivemind", |c| c
        .use_quotes(true)
        .min_args(1)
        .guild_only(true)
        .exec(commands::hivemind::hivemind))
        .command("impersonate", |c| c
        .use_quotes(true)
        .min_args(1)
        .guild_only(true)
        .exec(commands::impersonate::impersonate))
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

                               commands::helper::download_all_messages(&guild, &sql_pool);
                           });

    client.on_message(|_ctx, message| {
        let mut data = _ctx.data.lock().unwrap();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        commands::helper::insert_into_db(&sql_pool,
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