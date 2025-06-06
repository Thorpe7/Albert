use crate::message_utils::{
format_json_to_message, get_today_channel_hx, get_channel_name, send_dm_to_user
};
use crate::python_runner::run_python;
use crate::read_and_write::{read_json, write_messages_to_txt};
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;
use std::collections::HashMap;
pub struct Handler;

// Handler struct for message event - called when new message is received.
// ToDo1: Create worker queue
// ToDo2: Fix timezone...
#[async_trait]
impl EventHandler for Handler {

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name)
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) { // serenity::model::channel::Reaction - no timestamp field
        if let ReactionType::Unicode(ref emoji) = reaction.emoji {
            if emoji == "🤖" {
                if let Ok(_msg) = reaction
                    .channel_id
                    .message(&ctx.http, reaction.message_id)
                    .await
                {
                    // Grab the channel name & make sure its valid
                    let channel_name = get_channel_name(_msg, &ctx).await.unwrap();

                    // Grab the time, only want to summarize messages from today to start
                    let (start_of_today, history_result) = get_today_channel_hx(&reaction, &ctx).await;
                    let mut messages_today: Vec<HashMap<String, String>> = Vec::new();
                    
                    // Filtering today's messages
                    if let Ok(history) = history_result {
                        for chat in history.iter() {
                            if chat.timestamp.to_utc() >= start_of_today {
                                println!("{}", chat.timestamp.to_utc());
                                println!("{}", start_of_today);
                                let mut entry = HashMap::new();
                                entry.insert(chat.author.name.clone(), chat.content.clone());
                                messages_today.push(entry)
                            }
                        }
                    }
                    
                    // ToDo: Still a little messy, can be more concise
                    let dm: CreateMessage;
                    if messages_today.len() > 1 {
                        let result_filepath = write_messages_to_txt(&messages_today);
                        if let Ok(filepath) = result_filepath { 
                            run_python(&filepath);
                        }
                        let model_response = match read_json(None) {
                            Ok(data) => data,
                            Err(e) => {
                                println!("Failed to read JSON: {e}");
                                return;
                            }
                        };
                        let message_to_user = format_json_to_message(&model_response,&channel_name);
                        dm = CreateMessage::new().content(&message_to_user);
                    } else {
                        dm = CreateMessage::new().content("No messages found to summarize...");
                    }

                    if let Err(e) = send_dm_to_user(&reaction, &ctx, dm).await{
                        println!("Failed to DM: {:?}", e);
                    };
                }
            }
            if emoji == "📖" {
                // Nothing yet
            }
        }
    }
}
