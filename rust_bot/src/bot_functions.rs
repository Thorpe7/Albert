use serenity::all::Message;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use anyhow::Result;

use crate::article_handler::fetch_article_text;
use crate::bedrock_client::BedrockClient;
use crate::message_utils::{
    string_format_today_messages, get_channel_name, get_messages
};
use crate::response_target::{ResponseTarget, deliver_chat_summary, deliver_article_summary};


pub async fn summarize_chat(channel_id: ChannelId, ctx: &Context, response_target: &ResponseTarget, task_prompt: &str, bedrock_client: &BedrockClient) -> Result<()> {
    let channel_name = get_channel_name(channel_id, ctx).await?;
    let messages_today = get_messages(channel_id, ctx).await;

    let summary_text: String;
    if messages_today.len() > 1 {
        let formatted_messages = string_format_today_messages(&messages_today);
        let summary = bedrock_client.summarize_chat(&formatted_messages, task_prompt).await?;
        summary_text = format!("**Channel: **{}\n{}", channel_name, summary);
    } else {
        summary_text = "No messages found to summarize...".to_string();
    }

    deliver_chat_summary(response_target, ctx, &summary_text).await
}

pub async fn summarize_article(ctx: &Context, response_target: &ResponseTarget, article_url: String, original_msg: Option<&Message>, bedrock_client: &BedrockClient) -> Result<()> {
    let article_text = tokio::task::spawn_blocking(move || {
        fetch_article_text(&article_url)
    }).await??;

    let summary = bedrock_client.summarize_article(&article_text).await?;

    let summary_text = if summary.len() > 2000 {
        format!("{}...", &summary[..1997])
    } else {
        summary
    };

    deliver_article_summary(response_target, ctx, original_msg, &summary_text).await
}
