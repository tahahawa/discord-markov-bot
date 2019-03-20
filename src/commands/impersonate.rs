use diesel::dsl::*;
use diesel::prelude::*;
use markov::Chain;
use serenity::framework::standard::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::{content_safe, ContentSafeOptions};
use crate::Sqlpool;

enum IdOrUsername {
    Id(u64),
    Username(String),
}

pub fn impersonate(
    _context: &mut Context,
    message: &Message,
    mut args: Args,
) -> Result<(), CommandError> {
    debug!("args: {:?}", args);

    let fetch_from = match args.single::<u64>() {
        Ok(id) => IdOrUsername::Id(id),
        Err(_) => IdOrUsername::Username(args.single_quoted().unwrap_or_else(|_| "".to_owned())),
    };

    let count: usize = args.single_quoted().unwrap_or(1);

    // let chan = message.channel_id.get().unwrap();

    let _ = message.channel_id.broadcast_typing();

    let guild_arc = message.guild().unwrap();
    let guild = guild_arc.read();

    let user = match fetch_from {
        IdOrUsername::Id(id) => Some(UserId(id)),
        IdOrUsername::Username(username) => guild.member_named(&username).and_then(|m| Some(m.user_id())),
    };

    let conn;

    {
        let mut data = _context.data.lock();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        conn = sql_pool.get().unwrap();
    }

    if let Some(user) = user {
        let mut chain: Chain<String> = Chain::new();

        // use schema::messages;
        // use models::*;
        use crate::schema::messages::dsl::*;

        no_arg_sql_function!(RANDOM, (), "sql RANDOM()");

        let results = messages
            .select(content)
            .filter(author.eq(user.0 as i64))
            .filter(not(content.like("%~hivemind%")))
            .filter(not(content.like("%~impersonate%")))
            .filter(not(content.like("%~ping%")))
            .limit(10000)
            .order(RANDOM)
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
            let mut i = 0;
            let len = results.len();

            for m in results {
                trace!("Feeding message '{}' into chain", m);
                chain.feed_str(&m);

                if i == len / 4 {
                    let _ = message.channel_id.broadcast_typing();
                    i = 0;
                } else {
                    i += 1;
                }

            }

            // let iter_test = re_iter.replace_all(&count, "");

            // let iter: usize = iter_test.parse::<usize>().unwrap_or(1);

            let _ = message.channel_id.broadcast_typing();

            for line in chain.str_iter_for(count) {
                trace!("Outgoing message: '{}'", line);
                let _ = message
                    .channel_id
                    .say(content_safe(&line, &ContentSafeOptions::default()));
            }
        } else {
            info!("Requested command has no data available");
            let _ = message.reply("Either they've never said anything, or I haven't seen them");
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
        info!("User not found");
        let _ = message.reply("No user found");
    }
    Ok(())
}
