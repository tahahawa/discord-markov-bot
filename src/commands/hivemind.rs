use diesel::dsl::*;
use diesel::prelude::*;
use markov::Chain;
use serenity::{
    framework::standard::{
        Args, CommandResult,
        macros::{command},
    },
    model::{
        channel::{Message},
    },
    utils::{content_safe, ContentSafeOptions},
};

use serenity::prelude::*;

use crate::Sqlpool;

#[command]
#[min_args(0)]
pub fn hivemind(_context: &mut Context, message: &Message, mut args: Args) -> CommandResult {
    let _ = message.channel_id.broadcast_typing(&_context.http);

    debug!("args: {:?}", args);

    let count: usize = args.single_quoted().unwrap_or(1);

    let conn;

    {
        let mut data = _context.data.write();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        conn = sql_pool.get().unwrap();
    }

    let mut chain: Chain<String> = Chain::new();

    use crate::schema::messages::dsl::*;
    no_arg_sql_function!(RANDOM, (), "sql RANDOM()");

    let results = messages
        .select(content)
        .filter(not(content.like("%~hivemind%")))
        .filter(not(content.like("%~impersonate%")))
        .filter(not(content.like("%~ping%")))
        .limit(10000)
        .order(RANDOM)
        .load::<String>(&conn)
        .expect("Error loading messages");
    // let mut stmt = conn.prepare("SELECT * FROM messages where content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' ").unwrap();
    // let rows = stmt.query_map_named(&[], |row| row.get(3)).unwrap();

    // let mut messages = Vec::<String>::new();
    // for content in rows {
    //     messages.push(content.unwrap());
    // }

    let mut i = 0;
    let len = results.len();

    if !results.is_empty() {
        for m in results {
            trace!("Feeding message '{}' into chain", m);
            chain.feed_str(&m);

            if i == len / 4 {
                let _ = message.channel_id.broadcast_typing(&_context.http);
                i = 0;
            } else {
                i += 1;
            }
        }

        for line in chain.str_iter_for(count) {
            trace!("Outgoing message: '{}'", line);
            let _ = message.channel_id.say(
                &_context.http,
                content_safe(&_context.cache, &line, &ContentSafeOptions::default()),
            );
            //println!("{}", line);
        }
    } else {
        info!("Requested command has no data available");
        let _ = message.reply(_context, "They haven't said anything");
    }
    Ok(())
}
