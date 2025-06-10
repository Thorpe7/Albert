use serenity::all::Message;
use serenity::client::Context;
use serenity::model::channel::Reaction;
use tokio::sync::mpsc::Receiver;

use crate::bot_functions::summarize_chat;

pub enum Job {
    SummarizeChat {
        msg: Message,
        ctx: Context,
        reaction: Reaction
    },
    // SummarizeArticle {
    //     For the next bot feature
    // }

}

pub fn start_worker(mut rx: Receiver<Job>) {
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            match job {
                Job::SummarizeChat { msg, ctx, reaction } => summarize_chat(msg, &ctx, reaction).await
            }
        }
    });
}