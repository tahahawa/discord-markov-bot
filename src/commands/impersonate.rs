use serenity::client::*;
use serenity::model::*;
use serenity::framework::standard::Args;
use serenity::framework::standard::CommandError;
use regex::Regex;
use markov::Chain;

// use commands;

use Sqlpool;

pub fn impersonate(_context: &mut Context, message: &Message, args: Args) -> Result<(), CommandError> {

    let _args: Vec<String> = args.multiple_quoted().unwrap();
    // let chan = message.channel_id.get().unwrap();

    let _ = message.channel_id.broadcast_typing();;

    let re = Regex::new(r"(<@!?\d*>)").unwrap();

    let guild_arc = message.guild().unwrap();
    let guild = guild_arc.read().unwrap();

    let member = guild.member_named( &_args[0] );


    let mut user = None;

    if member.is_some() {
    let user_arc = member.unwrap();
    user = Some(user_arc.user.read().unwrap());
    }

    let data = _context.data.lock();
    let pool = data.get::<Sqlpool>().unwrap().clone();
    let conn = pool.get().unwrap();
    drop(data);

    if user.is_some() && _args.len() > 1 {
        let user = user.unwrap();
        let mut chain: Chain<String> = Chain::new();

        let mut stmt = conn.prepare("SELECT * FROM messages where author = :id and content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' ").unwrap();
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


            let iter: usize = iter_test.parse::<usize>().unwrap_or(1);

            for line in chain.str_iter_for(iter) {
                let _ = message.channel_id.say(&re.replace_all(&line, "@mention")
                    .into_owned());
                //println!("{}", line);
                let _ = message.channel_id.broadcast_typing();
            }
        } else {
            let _ = message.reply("They haven't said anything");
        }
    } else if user.is_some() {
        let user = user.unwrap();
        let mut chain: Chain<String> = Chain::new();

        let mut stmt = conn.prepare("SELECT * FROM messages where author = :id and content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' ").unwrap();
        let rows = stmt.query_map_named(&[(":id", &(user.id.0.to_string()))], |row| row.get(3))
            .unwrap();

        let mut messages = Vec::<String>::new();
        for content in rows {
            messages.push(content.unwrap());
        }
        let _ = message.channel_id.broadcast_typing();

        if !messages.is_empty() {
            for m in messages {
                chain.feed_str(&m);
            }
            let _ = message.channel_id.say(&re.replace_all(
                &chain.generate_str(),
                "@mention",
            ).into_owned());
        } else {
            let _ = message.reply("They haven't said anything");
        }
    } else {
        let _ = message.reply("No user found");
    }
    Ok(())
}
