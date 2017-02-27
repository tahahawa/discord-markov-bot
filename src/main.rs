#[macro_use]
extern crate serenity;
extern crate serde_yaml;
extern crate rusqlite;

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;


use serenity::client::Client;

use rusqlite::Connection;

fn main() {
    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&mut fstr).unwrap();


    let conn = Connection::open(config.get("db").unwrap()).unwrap();

    conn.execute("CREATE TABLE IF NOT EXISTS messages (
                  id                        INTEGER PRIMARY KEY,
                  author               INTEGER NOT NULL,
                  content             TEXT NOT NULL,
                  timestamp        TEXT NOT NULL)",
                 &[])
        .unwrap();


    let mut client = Client::login_bot(&config.get("token").expect("token"));
    client.with_framework(|f| {
        f
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .on("ping", ping)
    });

    // start listening for events by starting a single shard
    let _ = client.start();

    client.on_ready(|_ctx, ready| {
        println!("{} is connected!", ready.user.name);
        println!("{}", ready.);

        for chan in ready.channels {
            let _messages = channel.get_messages(|g| g.after(id).limit(100));

            while (!_messages.is_empty() && _messages.len() > 0) {

                for message in _messages {
                    conn.execute("INSERT INTO messages (id, author, content, timestamp)
                  VALUES (?1, ?2, ?3, ?4)",
                                 &[&message.id,
                                   &message.author,
                                   &message.content,
                                   &message.timestamp])
                        .unwrap();
                }
            }
        }
    });

}

command!(ping(_context, message) {
    let _ = message.reply("Pong!");
});
