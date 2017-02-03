extern crate serde_yaml;
extern crate discord;

use std::fs::File;
use std::io::Read;
use std::collections::BTreeMap;

use discord::{Discord, ChannelRef, State};
use discord::model::{Event, ChannelType};


fn main() {
    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&mut fstr).unwrap();

    let discord = Discord::from_bot_token(&config.get("token").expect("expected token"))
        .expect("login failed");

    // Establish the websocket connection
    let (mut connection, ready) = discord.connect().expect("connect failed");
    let mut state = State::new(ready);
    print!("{:?}", state);
    let channel_count: usize = state.servers()
        .iter()
        .map(|srv| {
            srv.channels
                .iter()
                .filter(|chan| chan.kind == ChannelType::Text)
                .count()
        })
        .fold(0, |v, s| v + s);
    println!("[Ready] {} logging {} servers with {} text channels",
             state.user().username,
             state.servers().len(),
             channel_count);


    loop {
        // Receive an event and update the state with it
        let event = match connection.recv_event() {
            Ok(event) => event,
            Err(discord::Error::Closed(code, body)) => {
                println!("[Error] Connection closed with status {:?}: {}", code, body);
                break;
            }
            Err(err) => {
                println!("[Warning] Receive error: {:?}", err);
                continue;
            }
        };
        state.update(&event);

        match connection.recv_event() {
            Ok(Event::MessageCreate(message)) => {
                println!("{} says: {}", message.author.name, message.content);
                if message.content == "!test" {
                    let _ = discord.send_message(message.channel_id,
                                                 "This is a reply to the test.",
                                                 "",
                                                 false);
                } else if message.content == "!quit" {
                    println!("Quitting.");
                    break;
                }
            }
            Ok(_) => {}
            Err(discord::Error::Closed(code, body)) => {
                println!("Gateway closed on us with code {:?}: {}", code, body);
                break;
            }
            Err(err) => println!("Receive error: {:?}", err),
        }
    }

}
