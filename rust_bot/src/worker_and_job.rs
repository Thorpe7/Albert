use serenity::all::Message;
use serenity::client::Context;
use serenity::model::channel::Reaction;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;
use std::fs;

use crate::bot_functions::{summarize_chat, summarize_article};

pub enum Job {
    SummarizeChat {
        uuid: Uuid,
        msg: Message,
        ctx: Context,
        reaction: Reaction,
        task_prompt: String
    },
    SummarizeArticle {
        uuid: Uuid,
        msg: Message,
        ctx: Context,
        reaction: Reaction,
        article_url: String,
    },
}

pub fn start_worker(mut rx: Receiver<Job>) {
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            match job {
                Job::SummarizeChat { uuid, msg, ctx, reaction, task_prompt } => {
                    match summarize_chat(uuid, msg, &ctx, reaction, task_prompt).await {
                        Ok(_) => {
                            let dir_path = format!("jobs/{}", uuid);
                            if let Err(e) = fs::remove_dir_all(&dir_path) {
                                eprintln!("Failed to delete job folder {}: {}", dir_path, e);
                            }
                        },
                        Err(e) => {
                            eprint!("Summarizing failed: {}",e);
                        }
                    }
                }
                Job::SummarizeArticle { uuid, msg, ctx, reaction, article_url } => {
                    match summarize_article(uuid, msg, &ctx, reaction, article_url).await {
                        Ok(_) => {
                            let dir_path = format!("jobs/{}", uuid);
                            if let Err(e) = fs::remove_dir_all(&dir_path) {
                                eprintln!("Failed to delete job folder {}: {}", dir_path, e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Article summarization failed: {}", e);
                        }
                    }
                }
            }
        }
    });
}