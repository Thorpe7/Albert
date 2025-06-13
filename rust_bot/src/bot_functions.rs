use serenity::all::Message;
use serenity::builder::CreateMessage;
use serenity::client::Context;
use serenity::model::channel::Reaction;
use uuid::Uuid;
use anyhow::Result;

use crate::message_utils::{
    format_json_to_message, string_format_today_messages, get_channel_name, get_messages
};
use crate::python_runner::run_python;
use crate::read_and_write::{read_json, write_messages_to_txt};


pub async fn summarize_chat(file_id:Uuid, msg:Message, ctx: &Context, reaction: Reaction, task_prompt: String) -> Result<()>{
    let (channel_name, response_path) = get_channel_name(file_id, msg, &ctx).await.unwrap();
    let messages_today = get_messages(&reaction, &ctx).await;
    
    // TODO: Separate running model from sending message
    let dm: CreateMessage;
    if messages_today.len() > 1 {
        let formatted_messages: String = string_format_today_messages(&messages_today);
        let msg_hx_path = write_messages_to_txt(&formatted_messages, &file_id);
        if let Err(e) = run_python(&msg_hx_path, &task_prompt).await {
            return Err(anyhow::anyhow!("Running python script failed: {}",e));
        }
        let model_response = match read_json(&response_path) {
            Ok(data) => data,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to read JSON ({}): {}",&response_path,e));
            }
        };
        let message_to_user = format_json_to_message(&model_response,&channel_name);
        dm = CreateMessage::new().content(&message_to_user);
    } else {
        dm = CreateMessage::new().content("No messages found to summarize...");
    }

    if let Some(user_id) = reaction.user_id {
        if let Ok(user) = user_id.to_user(&ctx.http).await {
            if let Err(why) = user.direct_message(&ctx.http, dm).await {
                return Err(anyhow::anyhow!("Failed to send dm to user: {why:?}"));
            }
        } else {
            return Err(anyhow::anyhow!("Failed to fetch user from user_id..."));
        }
    } else {
        return Err(anyhow::anyhow!("No user_id on reaction..."));
    }
    Ok(())
}