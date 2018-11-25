#[macro_use]
extern crate diesel;

#[macro_use]
extern crate serenity;

#[macro_use]
extern crate log;

extern crate pretty_env_logger;

extern crate bigdecimal;
extern crate num;

extern crate markov;
extern crate serde_yaml;
extern crate typemap;

extern crate chrono;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use typemap::Key;

use serenity::framework::standard::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

use chrono::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use models::*;

pub mod commands;
pub mod models;
pub mod schema;

pub type Pool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>;

pub struct Sqlpool;

impl Key for Sqlpool {
    type Value = Pool;
}

command!(ping(_ctx, msg, _args){
    if let Err(why) = msg.channel_id.say("Pong!") {
        warn!("Error sending message: {:?}", why);
    }
});

command!(stats(_ctx, msg, _args){
        let cache = serenity::CACHE.read();
        let mut guild_names: Vec<String> = Vec::new();

        for (id, _) in cache.clone().guilds {
            guild_names.push(id.to_partial_guild().unwrap().name);
        }

        info!("guilds: {:?}; channels: {}; users: {}", 
        guild_names,
        cache.channels.len(),
        cache.users.len());


        if let Err(why) = msg.channel_id.say(
            format!("guilds: {:?}; channels: {}; users: {}", 
            guild_names,
            cache.channels.len(),
            cache.users.len())){
                info!("Error sending message: {:?}", why);
                };
});

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Version {} of markovbot", env!("CARGO_PKG_VERSION"));
        info!("{} is connected!", ready.user.name);
    }

    fn resume(&self, _: Context, resume: ResumedEvent) {
        // Log at the DEBUG level.
        //
        // In this example, this will not show up in the logs because DEBUG is
        // below INFO, which is the set debug level.
        debug!("Resumed; trace: {:?}", resume.trace);
    }

    fn guild_create(&self, _ctx: Context, guild: Guild, _: bool) {
        commands::helper::download_all_messages(&guild, &_ctx);
    }

    fn message(&self, _ctx: Context, message: Message) {
        let mut message_vec = Vec::new();

        let val = InsertableMessage {
            id: message.id.0 as i64,
            channel_id: message.channel_id.0 as i64,
            author: message.author.id.0 as i64,
            content: message.content,
            timestamp: message.timestamp,
        };

        message_vec.push(val);

        commands::helper::insert_into_db(&_ctx, &message_vec);

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
    pretty_env_logger::init();

    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&fstr).unwrap();

    let dbname = config["db"].clone();

    let manager = diesel::r2d2::ConnectionManager::<PgConnection>::new(dbname.to_string());

    let pool = diesel::r2d2::Pool::builder()
        .max_size(120)
        .build(manager)
        .unwrap_or_else(|_| panic!("Error connecting to {}", dbname.to_string()));

    let conn = pool.get().unwrap();

    use schema::messages;

    let def_vals = models::InsertableMessage {
        id: 0,
        channel_id: 0,
        author: 0,
        content: "0".to_owned(),
        timestamp: DateTime::parse_from_rfc3339("2014-11-28T21:00:09+09:00").unwrap(),
    };

    let _ = diesel::insert_into(messages::table)
        .values(&def_vals)
        .on_conflict_do_nothing()
        .execute(&conn)
        .expect("Error inserting default values");

    info!("pre-init done");

    let mut client = Client::new(&config["token"], Handler).unwrap();
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
            .on_dispatch_error(|_ctx, msg, error| {
                if let DispatchError::RateLimited(seconds) = error {
                    info!("Hivemind cooldown for {} more seconds", seconds);
                    let _ = msg
                        .channel_id
                        .say(&format!("Try this again in {} seconds.", seconds));
                }
            })
            .before(|_, msg, command_name| {
                info!(
                    "Got command '{}' by user '{}'",
                    command_name, msg.author.name
                );
                true
            })
            .command("ping", |c| c.cmd(ping))
            .command("stats", |c| c.cmd(stats))
            .command("hivemind", |c| {
                c
                    // .use_quotes(false)
                    .min_args(0)
                    .guild_only(true)
                    .bucket("hivemind")
                    .cmd(commands::hivemind::hivemind)
            })
            .command("impersonate", |c| {
                c
                    // .use_quotes(true)
                    .min_args(1)
                    .guild_only(true)
                    .cmd(commands::impersonate::impersonate)
            })
            .simple_bucket("hivemind", 300),
    );

    {
        let mut data = client.data.lock();
        data.insert::<Sqlpool>(pool.clone());
    }

    if let Err(why) = client.start_autosharded() {
        println!("Client error: {:?}", why);
    }
}
