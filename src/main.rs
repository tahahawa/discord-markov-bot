#[macro_use]
extern crate serenity;
extern crate serde_yaml;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;
use r2d2_sqlite::SqliteConnectionManager;

//use serenity::model::GuildId;
use serenity::client::Client;

//use rusqlite::Connection;

fn main() {
    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&mut fstr).unwrap();

    let dbname = config.get("db").unwrap().clone();

    let r2d2_config = r2d2::Config::default();
    let manager = SqliteConnectionManager::new(&dbname);

    let pool = r2d2::Pool::new(r2d2_config, manager).unwrap();
    //let conn = Connection::open(dbname.clone()).unwrap();
    let conn = pool.get().unwrap();


    conn.execute("CREATE TABLE IF NOT EXISTS messages (
                  id                        INTEGER PRIMARY KEY,
                  author               INTEGER NOT NULL,
                  content             TEXT NOT NULL,
                  timestamp        TEXT NOT NULL)",
                 &[])
        .unwrap();

    conn.execute("INSERT or REPLACE INTO messages (id, author, content, timestamp) \
                                          VALUES (0, 0, 0, 0)",
                 &[])
        .unwrap();

    println!("pre-init done");

    let mut client = Client::login_bot(&config.get("token").expect("token"));
    client.with_framework(|f| {
                              f
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .on("ping", ping)
                          });



    client.on_ready(move |_ctx, ready| {
                        println!("{} is connected!", ready.user.name);
                        println!("{:?}", ready.guilds);

                        download_all_messages(ready, &pool);
                    });

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }

}

command!(ping(_context, message) {
    let _ = message.reply("Pong!");
});

fn download_all_messages(ready: serenity::model::Ready,
                         ref pool: &r2d2::Pool<SqliteConnectionManager>) {
    for guild in ready.guilds {
        for chan in guild.id().get_channels().unwrap() {
            let mut _messages;

            let on_ready_pool = pool.clone();
            let on_ready_conn = on_ready_pool.get().unwrap();

            let row: Result<i64, _> =
                on_ready_conn.query_row("SELECT MAX(id) FROM messages", &[], |row| row.get(0));

            let id = row.unwrap();
            println!("{}", id);

            if id == 0 {
                println!("no message ID");
                _messages = chan.0.get_messages(|g| g.after(0).limit(100)).unwrap();
            } else {
                _messages = chan.0
                    .get_messages(|g| g.after(serenity::model::MessageId(id as u64)).limit(100))
                    .unwrap();
            }

            while !_messages.is_empty() && _messages.len() > 0 {
                let message_vec = _messages.to_vec();
                for message in message_vec {

                    let on_ready_loop_pool = pool.clone();
                    let on_ready_loop_conn = on_ready_loop_pool.get().unwrap();

                    let _ = on_ready_loop_conn.execute("INSERT or REPLACE INTO messages (id, author, content, timestamp) \
                                          VALUES (?1, ?2, ?3, ?4)",
                                          &[&(message.id.0 as i64),
                                            &(message.author.id.0 as i64),
                                            &message.content,
                                            &message.timestamp]);

                    //println!("{:?}", message);
                }
                let row2: Result<i64, _> =
                    on_ready_conn.query_row("SELECT MAX(id) FROM messages", &[], |row| row.get(0));

                let id2 = row2.unwrap();
                println!("{}", id2);

                if id2 == 0 {
                    println!("no message ID");
                    _messages = chan.0.get_messages(|g| g.after(0).limit(100)).unwrap();
                } else if _messages.len() < 100 {
                    break;
                } else {
                    _messages = chan.0
                        .get_messages(|g| {
                                          g.after(serenity::model::MessageId(id2 as u64)).limit(100)
                                      })
                        .unwrap();
                }
            }
        }
    }
}