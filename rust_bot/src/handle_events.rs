use serenity::async_trait;
use serenity::builder::EditInteractionResponse;
use serenity::model::application::{Command, CommandInteraction, CommandType, CommandOptionType, Interaction, ResolvedTarget};
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::ReactionType;
use serenity::prelude::*;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;
use crate::article_handler::{extract_url, bot_already_replied};
use crate::response_target::ResponseTarget;
use crate::worker_and_job::Job;

pub struct Handler {
    pub tx: Sender<Job>
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let ReactionType::Unicode(ref emoji) = reaction.emoji {
            if emoji == "📄" {
                let channel_id = reaction.channel_id;
                let task_prompt = "STANDARD_SUMMARY".to_string();
                let job = Job::SummarizeChat {
                    uuid: Uuid::new_v4(),
                    channel_id,
                    ctx,
                    response_target: ResponseTarget::ReactionDm { reaction },
                    task_prompt,
                };
                self.tx.send(job).await.unwrap();
            }
            else if emoji == "📑" {
                let channel_id = reaction.channel_id;
                let task_prompt = "PER_USER_SUMMARY".to_string();
                let job = Job::SummarizeChat {
                    uuid: Uuid::new_v4(),
                    channel_id,
                    ctx,
                    response_target: ResponseTarget::ReactionDm { reaction },
                    task_prompt,
                };
                self.tx.send(job).await.unwrap();
            }
            else if emoji == "\u{1F4D6}" {
                if let Ok(msg) = reaction
                    .channel_id
                    .message(&ctx.http, reaction.message_id)
                    .await
                {
                    if bot_already_replied(&msg) {
                        println!("Albert already summarized this article, skipping...");
                        return;
                    }
                    if let Some(article_url) = extract_url(&msg.content) {
                        let job = Job::SummarizeArticle {
                            uuid: Uuid::new_v4(),
                            ctx,
                            response_target: ResponseTarget::ReactionReply { reaction },
                            article_url,
                            original_msg: Some(msg),
                        };
                        self.tx.send(job).await.unwrap();
                    } else {
                        println!("No URL found in message, skipping...");
                    }
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "summary-24hr" => self.handle_summary_chat(ctx, command, "STANDARD_SUMMARY").await,
                "summary-peruser" => self.handle_summary_chat(ctx, command, "PER_USER_SUMMARY").await,
                "summary-article" => self.handle_summary_article_slash(ctx, command).await,
                "Summarize Article" => self.handle_summary_article_context_menu(ctx, command).await,
                _ => {}
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let commands = vec![
            CreateCommand::new("summary-24hr")
                .description("Summarize the last 24 hours of this channel"),
            CreateCommand::new("summary-peruser")
                .description("Per-user summary of the last 24 hours"),
            CreateCommand::new("summary-article")
                .description("Summarize an article from a URL")
                .add_option(
                    CreateCommandOption::new(CommandOptionType::String, "url", "The article URL to summarize")
                        .required(true)
                ),
            CreateCommand::new("Summarize Article")
                .kind(CommandType::Message),
        ];

        if let Err(e) = Command::set_global_commands(&ctx.http, commands).await {
            eprintln!("Failed to register global commands: {}", e);
        } else {
            println!("Slash commands registered successfully!");
        }
    }
}

impl Handler {
    async fn handle_summary_chat(&self, ctx: Context, command: CommandInteraction, task_prompt: &str) {
        if let Err(e) = command.defer_ephemeral(&ctx.http).await {
            eprintln!("Failed to defer ephemeral: {}", e);
            return;
        }

        let channel_id = command.channel_id;
        let job = Job::SummarizeChat {
            uuid: Uuid::new_v4(),
            channel_id,
            ctx,
            response_target: ResponseTarget::EphemeralInteraction { interaction: command },
            task_prompt: task_prompt.to_string(),
        };
        self.tx.send(job).await.unwrap();
    }

    async fn handle_summary_article_slash(&self, ctx: Context, command: CommandInteraction) {
        let url = command.data.options.iter()
            .find(|o| o.name == "url")
            .and_then(|o| o.value.as_str())
            .map(|s| s.to_string());

        let article_url = match url {
            Some(u) => u,
            None => {
                let _ = command.defer_ephemeral(&ctx.http).await;
                let response = EditInteractionResponse::new()
                    .content("Please provide a URL to summarize.");
                let _ = command.edit_response(&ctx.http, response).await;
                return;
            }
        };

        if let Err(e) = command.defer_ephemeral(&ctx.http).await {
            eprintln!("Failed to defer ephemeral: {}", e);
            return;
        }

        let job = Job::SummarizeArticle {
            uuid: Uuid::new_v4(),
            ctx,
            response_target: ResponseTarget::EphemeralInteraction { interaction: command },
            article_url,
            original_msg: None,
        };
        self.tx.send(job).await.unwrap();
    }

    async fn handle_summary_article_context_menu(&self, ctx: Context, command: CommandInteraction) {
        // Get message ID from resolved data, then re-fetch for full reaction info
        let msg_id = match command.data.target() {
            Some(ResolvedTarget::Message(msg)) => msg.id,
            _ => {
                let _ = command.defer_ephemeral(&ctx.http).await;
                let response = EditInteractionResponse::new()
                    .content("Could not find the target message.");
                let _ = command.edit_response(&ctx.http, response).await;
                return;
            }
        };

        let target_msg = match command.channel_id.message(&ctx.http, msg_id).await {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Failed to fetch target message: {}", e);
                let _ = command.defer_ephemeral(&ctx.http).await;
                let response = EditInteractionResponse::new()
                    .content("Failed to fetch the target message.");
                let _ = command.edit_response(&ctx.http, response).await;
                return;
            }
        };

        if bot_already_replied(&target_msg) {
            let _ = command.defer_ephemeral(&ctx.http).await;
            let response = EditInteractionResponse::new()
                .content("This article has already been summarized.");
            let _ = command.edit_response(&ctx.http, response).await;
            return;
        }

        let article_url = match extract_url(&target_msg.content) {
            Some(url) => url,
            None => {
                let _ = command.defer_ephemeral(&ctx.http).await;
                let response = EditInteractionResponse::new()
                    .content("No URL found in the target message.");
                let _ = command.edit_response(&ctx.http, response).await;
                return;
            }
        };

        if let Err(e) = command.defer(&ctx.http).await {
            eprintln!("Failed to defer: {}", e);
            return;
        }

        let job = Job::SummarizeArticle {
            uuid: Uuid::new_v4(),
            ctx,
            response_target: ResponseTarget::VisibleInteraction { interaction: command },
            article_url,
            original_msg: Some(target_msg),
        };
        self.tx.send(job).await.unwrap();
    }
}
