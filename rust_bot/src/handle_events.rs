use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;
use crate::worker_and_job::Job;
pub struct Handler {
    pub tx: Sender<Job>
}

#[async_trait]
impl EventHandler for Handler {
    // Handler struct for message event - called when new message is received.

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let ReactionType::Unicode(ref emoji) = reaction.emoji {
            if emoji == "🤖" {
                if let Ok(_msg) = reaction
                    .channel_id
                    .message(&ctx.http, reaction.message_id)
                    .await{
                        let job = Job::SummarizeChat { uuid: Uuid::new_v4(), msg: _msg, ctx: ctx, reaction: reaction };
                        self.tx.send(job).await.unwrap();
                    }
                }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name)
    }
}
