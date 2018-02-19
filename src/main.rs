extern crate r2d2;
extern crate r2d2_sqlite;
extern crate serde_yaml;
#[macro_use]
extern crate serenity;
// extern crate rusqlite;
extern crate markov;
extern crate regex;
extern crate typemap;

mod commands;

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;
use typemap::Key;

use r2d2_sqlite::SqliteConnectionManager;

use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::*;

pub type SqlitePool = r2d2::Pool<SqliteConnectionManager>;

pub struct Sqlpool;

impl Key for Sqlpool {
    type Value = SqlitePool;
}

command!(ping(_ctx, msg, _args){
    if let Err(why) = msg.channel_id.say("Pong!") {
        println!("Error sending message: {:?}", why);
    }
});

command!(stats(_ctx, msg, _args){
        let cache = serenity::CACHE.read();
        if let Err(why) = msg.channel_id.say(
            format!("guilds: {:?}; channels: {}; users: {}", 
            cache.guilds,
            cache.channels.len(),
            cache.users.len())){
                println!("Error sending message: {:?}", why);
                };
});

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        // println!("{:?}", ready.guilds);
        //let mut data = _ctx.data.lock().unwrap();
        //let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        //download_all_messages(ready, sql_pool );
        let cache = serenity::CACHE.read();
        println!(
            "guilds: {:?}; channels: {}; users: {}",
            cache.guilds,
            cache.channels.len(),
            cache.users.len()
        );
    }

    //noinspection Annotator
    fn guild_create(&self, _ctx: Context, guild: Guild, _: bool) {
        let mut data = _ctx.data.lock();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        commands::helper::download_all_messages(&guild, &sql_pool);
    }

    fn message(&self, _ctx: Context, message: Message) {
        let mut data = _ctx.data.lock();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        commands::helper::insert_into_db(
            &sql_pool,
            &message.id.0.to_string(),
            &message.channel_id.0.to_string(),
            &message.author.id.0.to_string(),
            &message.content,
            &message.timestamp.to_string(),
        );

        //println!("added message on_message: {}", message.id.0.to_string());
    }

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
}

fn main() {
    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&fstr).unwrap();

    let dbname = config["db"].clone();

    let manager = r2d2_sqlite::SqliteConnectionManager::file(&dbname);

    let pool = r2d2::Pool::builder().max_size(10).build(manager).unwrap();
    let conn = pool.get().unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
                  id                        TEXT PRIMARY KEY,
                  channel_id        TEXT NOT NULL,
                  author              TEXT NOT NULL,
                  content             TEXT NOT NULL,
                  timestamp       TEXT NOT NULL)",
        &[],
    ).unwrap();

    conn.execute(
        "INSERT or REPLACE INTO messages (id, channel_id, author, content, timestamp) \
         VALUES (0, 0, 0, 0, 0)",
        &[],
    ).unwrap();

    println!("pre-init done");

    let mut client = Client::new(&config["token"], Handler).unwrap();
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
            .on_dispatch_error(|_ctx, msg, error| {
                if let DispatchError::RateLimited(seconds) = error {
                    let _ = msg.channel_id.say(&format!("Try this again in {} seconds.", seconds));
                }
            })
            .before(|_, msg, command_name| {
                println!("Got command '{}' by user '{}'",
                         command_name,
                         msg.author.name);
                true
            })
            .command("ping", |c| c.cmd(ping))
            .command("stats", |c| c
                .owners_only(true)
                .cmd(stats))
            .command("hivemind", |c| c
                // .use_quotes(false)
                .min_args(0)
                .guild_only(true)
                .bucket("hivemind")
                .cmd(commands::hivemind::hivemind))
            .command("impersonate", |c| c
                // .use_quotes(true)
                .min_args(1)
                .guild_only(true)
                .cmd(commands::impersonate::impersonate))
            .simple_bucket("hivemind", 300),
    );

    {
        let mut data = client.data.lock();
        data.insert::<Sqlpool>(pool.clone());
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded() {
        println!("Client error: {:?}", why);
    }
}
