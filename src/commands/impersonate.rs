use serenity::client::*;
use serenity::model::*;
use regex::Regex;
use markov::Chain;

use commands;

use Sqlpool;

pub fn impersonate(_context: &mut Context,
               message: &Message,
               _args: Vec<String>)
               -> Result<(), String> {
    let chan = _context.channel_id
        .unwrap()
        .get()
        .unwrap();

    let re = Regex::new(r"(<@!?\d*>)").unwrap();

    let guild_id = commands::helper::get_guild_id_from_chan(chan);
    let mut user = None;
    let mut offset = 0;
    let mut count = 0;
    'outer: while user.is_none() {
        let members = guild_id.members(Some(1000), Some(offset)).unwrap();

        if count == 10 || members.is_empty() {
            break 'outer;
        } else {
            offset = members[members.len()-1]
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
                if iter_test.parse::<usize>().is_ok() {
                    iter_test.parse::<usize>().unwrap()
                }
                else {
                    1
                }
            } else {
                1
            };

            for line in chain.str_iter_for(iter) {
                
                let _ = message.channel_id.say(&re.replace_all(&line, "@mention").into_owned());
                //println!("{}", line);
            }

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
            let _ = message.channel_id.say(&re.replace_all(&chain.generate_str(), "@mention").into_owned());
        } else {
            let _ = message.reply("They haven't said anything");
        }
    } else {
        let _ = message.reply("No user found");
    }
    Ok(())

}
