use serenity::all::Message;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;
use std::fs;

use crate::bot_functions::{summarize_chat, summarize_article};
use crate::response_target::{ResponseTarget, send_error_response};

pub enum Job {
    SummarizeChat {
        uuid: Uuid,
        channel_id: ChannelId,
        ctx: Context,
        response_target: ResponseTarget,
        task_prompt: String,
    },
    SummarizeArticle {
        uuid: Uuid,
        ctx: Context,
        response_target: ResponseTarget,
        article_url: String,
        original_msg: Option<Message>,
    },
}

pub fn start_worker(mut rx: Receiver<Job>) {
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            match job {
                Job::SummarizeChat { uuid, channel_id, ctx, response_target, task_prompt } => {
                    match summarize_chat(uuid, channel_id, &ctx, &response_target, task_prompt).await {
                        Ok(_) => {
                            let dir_path = format!("jobs/{}", uuid);
                            if let Err(e) = fs::remove_dir_all(&dir_path) {
                                eprintln!("Failed to delete job folder {}: {}", dir_path, e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Summarizing failed: {}", e);
                            if let Err(notify_err) = send_error_response(&response_target, &ctx, &e.to_string()).await {
                                eprintln!("Failed to notify user of error: {}", notify_err);
                            }
                        }
                    }
                }
                Job::SummarizeArticle { uuid, ctx, response_target, article_url, original_msg } => {
                    match summarize_article(uuid, &ctx, &response_target, article_url, original_msg.as_ref()).await {
                        Ok(_) => {
                            let dir_path = format!("jobs/{}", uuid);
                            if let Err(e) = fs::remove_dir_all(&dir_path) {
                                eprintln!("Failed to delete job folder {}: {}", dir_path, e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Article summarization failed: {}", e);
                            if let Err(notify_err) = send_error_response(&response_target, &ctx, &e.to_string()).await {
                                eprintln!("Failed to notify user of error: {}", notify_err);
                            }
                        }
                    }
                }
            }
        }
    });
}
