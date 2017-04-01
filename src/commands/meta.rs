use serenity::client::*;
use serenity::model::*;

pub fn ping(_context: &mut Context,
               message: &Message, _args: Vec<String>)-> Result<(), String>  {
    let _ = message.reply("Pong!");
    Ok(())
}
