use serenity::client::*;
use serenity::model::*;
use regex::Regex;
use markov::Chain;
use Sqlpool;

pub fn hivemind(
    _context: &mut Context,
    message: &Message,
    _args: Vec<String>,
) -> Result<(), String> {

    let _ = message.channel_id.broadcast_typing();

    let re = Regex::new(r"(<@!?\d*>)").unwrap();

    let data = _context.data.lock().unwrap();
    let pool = data.get::<Sqlpool>().unwrap().clone();
    let conn = pool.get().unwrap();
    drop(data);

    if !_args.is_empty() {
        let mut chain: Chain<String> = Chain::new();

        let mut stmt = conn.prepare("SELECT * FROM messages where content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' " ).unwrap();
        let rows = stmt.query_map_named(&[], |row| row.get(3)).unwrap();

        let mut messages = Vec::<String>::new();
        for content in rows {
            messages.push(content.unwrap());
        }

        if !messages.is_empty() {

            for m in messages {
                let _ = message.channel_id.broadcast_typing();

                chain.feed_str(&m);
            }

            let re_iter = Regex::new(r"\D").unwrap();
            let iter_test = re_iter.replace_all(&_args[0], "");

            let iter: usize = if !iter_test.is_empty() {
                if iter_test.parse::<usize>().is_ok() {
                    iter_test.parse::<usize>().unwrap()
                } else {
                    1
                }
            } else {
                1
            };

            for line in chain.str_iter_for(iter) {
                let _ = message.channel_id.say(&re.replace_all(&line, "@mention")
                    .into_owned());
                //println!("{}", line);
                let _ = message.channel_id.broadcast_typing();

            }

        } else {
            let _ = message.reply("They haven't said anything");
        }

    } else {
        let mut chain: Chain<String> = Chain::new();

        let mut stmt = conn.prepare("SELECT * FROM messages where content not like '%~hivemind%' and content not like '%~impersonate%' and content not like '%~ping%' " ).unwrap();
        let rows = stmt.query_map_named(&[], |row| row.get(3)).unwrap();

        let mut messages = Vec::<String>::new();
        for content in rows {
            messages.push(content.unwrap());
        }

        if !messages.is_empty() {
            for m in messages {
                let _ = message.channel_id.broadcast_typing();

                chain.feed_str(&m);
            }
            let _ = message.channel_id.say(&re.replace_all(
                &chain.generate_str(),
                "@mention",
            ).into_owned());
        } else {
            let _ = message.reply("They haven't said anything");
        }
    }
    Ok(())

}
