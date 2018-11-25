use diesel::dsl::*;
use diesel::prelude::*;
use markov::Chain;
use serenity::framework::standard::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::{content_safe, ContentSafeOptions};
use Sqlpool;

pub fn hivemind(
    _context: &mut Context,
    message: &Message,
    mut args: Args,
) -> Result<(), CommandError> {
    let _ = message.channel_id.broadcast_typing();

    debug!("args: {:?}", args);

    let count: usize = args.single_quoted().unwrap_or(1);

    let conn;

    {
        let mut data = _context.data.lock();
        let sql_pool = data.get_mut::<Sqlpool>().unwrap().clone();

        conn = sql_pool.get().unwrap();
    }

    let mut chain: Chain<String> = Chain::new();

    use schema::messages::dsl::*;

    let results = messages
        .select(content)
        .filter(not(content.like("%~hivemind%")))
        .filter(not(content.like("%~impersonate%")))
        .filter(not(content.like("%~ping%")))
        .load::<String>(&conn)
        .expect("Error loading messages");
    // let mut stmt = conn.prepare("SELECT * FROM messages where content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' ").unwrap();
    // let rows = stmt.query_map_named(&[], |row| row.get(3)).unwrap();

    // let mut messages = Vec::<String>::new();
    // for content in rows {
    //     messages.push(content.unwrap());
    // }

    if !results.is_empty() {
        for m in results {
            trace!("Feeding message '{}' into chain", m);
            chain.feed_str(&m);
            
            let _ = message.channel_id.broadcast_typing();
        }

        for line in chain.str_iter_for(count) {
            trace!("Outgoing message: '{}'", line);
            let _ = message
                .channel_id
                .say(content_safe(&line, &ContentSafeOptions::default()));
            //println!("{}", line);
        }
    } else {
        info!("Requested command has no data available");
        let _ = message.reply("They haven't said anything");
    }
    Ok(())
}
