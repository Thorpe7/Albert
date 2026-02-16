use serenity::all::Message;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use tokio::sync::mpsc::Receiver;
use std::sync::Arc;

use crate::bedrock_client::BedrockClient;
use crate::bot_functions::{summarize_chat, summarize_article};
use crate::response_target::{ResponseTarget, send_error_response};

pub enum Job {
    SummarizeChat {
        channel_id: ChannelId,
        ctx: Context,
        response_target: ResponseTarget,
        task_prompt: String,
    },
    SummarizeArticle {
        ctx: Context,
        response_target: ResponseTarget,
        article_url: String,
        original_msg: Option<Message>,
    },
}

pub fn start_worker(mut rx: Receiver<Job>, bedrock_client: Arc<BedrockClient>) {
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            match job {
                Job::SummarizeChat { channel_id, ctx, response_target, task_prompt } => {
                    if let Err(e) = summarize_chat(channel_id, &ctx, &response_target, &task_prompt, &bedrock_client).await {
                        eprintln!("Summarizing failed: {}", e);
                        if let Err(notify_err) = send_error_response(&response_target, &ctx, &e.to_string()).await {
                            eprintln!("Failed to notify user of error: {}", notify_err);
                        }
                    }
                }
                Job::SummarizeArticle { ctx, response_target, article_url, original_msg } => {
                    if let Err(e) = summarize_article(&ctx, &response_target, article_url, original_msg.as_ref(), &bedrock_client).await {
                        eprintln!("Article summarization failed: {}", e);
                        if let Err(notify_err) = send_error_response(&response_target, &ctx, &e.to_string()).await {
                            eprintln!("Failed to notify user of error: {}", notify_err);
                        }
                    }
                }
            }
        }
    });
}
