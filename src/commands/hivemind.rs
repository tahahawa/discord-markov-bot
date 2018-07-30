use diesel::dsl::*;
use diesel::prelude::*;
use markov::Chain;
use regex::Regex;
use serenity::framework::standard::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use Sqlpool;

pub fn hivemind(
    _context: &mut Context,
    message: &Message,
    mut args: Args,
) -> Result<(), CommandError> {
    let _ = message.channel_id.broadcast_typing();

    let re = Regex::new(r"(<@!?\d*>)").unwrap();

    println!("args: {:?}", args);

    let count: usize = args.single_quoted().unwrap_or(1);

    let data = _context.data.lock();
    let pool = data.get::<Sqlpool>().unwrap().clone();
    let conn = pool.get().unwrap();
    drop(data);

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
            chain.feed_str(&m);
        }

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
    Ok(())
}
