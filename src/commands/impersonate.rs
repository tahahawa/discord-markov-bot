use diesel::dsl::*;
use diesel::prelude::*;
use markov::Chain;
use regex::Regex;
use serenity::framework::standard::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use Sqlpool;

enum IdOrUsername {
    Id(u64),
    Username(String),
}

pub fn impersonate(
    _context: &mut Context,
    message: &Message,
    mut args: Args,
) -> Result<(), CommandError> {
    println!("args: {:?}", args);

    let fetch_from = match args.single::<u64>() {
        Ok(id) => IdOrUsername::Id(id),
        Err(_) => IdOrUsername::Username(args.single_quoted().unwrap_or_else(|_| "".to_owned())),
    };

    let count: usize = args.single_quoted().unwrap_or(1);

    // let chan = message.channel_id.get().unwrap();

    let _ = message.channel_id.broadcast_typing();

    let re = Regex::new(r"(<@!?\d*>)").unwrap();

    let guild_arc = message.guild().unwrap();
    let guild = guild_arc.read();

    let member = match fetch_from {
        IdOrUsername::Id(id) => guild.members.get(&UserId(id)),
        IdOrUsername::Username(username) => guild.member_named(&username),
    };

    let user = member.map(|m| m.user.read());

    let data = _context.data.lock();
    let pool = data.get::<Sqlpool>().unwrap().clone();
    let conn = pool.get().unwrap();
    drop(data);

    if let Some(user) = user {
        let mut chain: Chain<String> = Chain::new();

        // use schema::messages;
        // use models::*;
        use schema::messages::dsl::*;

        let results = messages
            .select(content)
            .filter(author.eq(user.id.0.to_string()))
            .filter(not(content.like("%~hivemind%")))
            .filter(not(content.like("%~impersonate%")))
            .filter(not(content.like("%~ping%")))
            .load::<String>(&conn)
            .expect("Error loading messages");

        // let mut stmt = conn.prepare("SELECT * FROM messages where author = :id and content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' ").unwrap();
        // let rows = stmt.query_map_named(&[(":id", &(user.id.0.to_string()))], |row| row.get(3))
        //     .unwrap();

        // let mut messages = Vec::<String>::new();
        // for content in rows {
        //     messages.push(content.unwrap());
        // }

        if !results.is_empty() {
            for m in results {
                chain.feed_str(&m);
            }

            // let re_iter = Regex::new(r"\D").unwrap();
            // let iter_test = re_iter.replace_all(&count, "");

            // let iter: usize = iter_test.parse::<usize>().unwrap_or(1);

            for line in chain.str_iter_for(count) {
                let _ = message
                    .channel_id
                    .say(&re.replace_all(&line, "@mention").into_owned());
                //println!("{}", line);
                let _ = message.channel_id.broadcast_typing();
            }
        } else {
            let _ = message.reply("They haven't said anything");
        }
    // } else if user.is_some() {
    //     let user = user.unwrap();
    //     let mut chain: Chain<String> = Chain::new();

    //     let mut stmt = conn.prepare("SELECT * FROM messages where author = :id and content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' ").unwrap();
    //     let rows = stmt.query_map_named(&[(":id", &(user.id.0.to_string()))], |row| row.get(3))
    //         .unwrap();

    //     let mut messages = Vec::<String>::new();
    //     for content in rows {
    //         messages.push(content.unwrap());
    //     }
    //     let _ = message.channel_id.broadcast_typing();

    //     if !messages.is_empty() {
    //         for m in messages {
    //             chain.feed_str(&m);
    //         }
    //         let _ = message.channel_id.say(&re.replace_all(
    //             &chain.generate_str(),
    //             "@mention",
    //         ).into_owned());
    //     } else {
    //         let _ = message.reply("They haven't said anything");
    //     }
    } else {
        let _ = message.reply("No user found");
    }
    Ok(())
}
