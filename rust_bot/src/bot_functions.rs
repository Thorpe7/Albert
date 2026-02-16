use serenity::all::Message;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use uuid::Uuid;
use anyhow::Result;

use crate::article_handler::fetch_article_text;
use crate::message_utils::{
    format_json_to_message, string_format_today_messages, get_channel_name, get_messages
};
use crate::python_runner::run_python;
use crate::read_and_write::{read_json, write_messages_to_txt, write_article_to_txt};
use crate::response_target::{ResponseTarget, deliver_chat_summary, deliver_article_summary};


pub async fn summarize_chat(file_id: Uuid, channel_id: ChannelId, ctx: &Context, response_target: &ResponseTarget, task_prompt: String) -> Result<()> {
    let (channel_name, response_path) = get_channel_name(file_id, channel_id, &ctx).await?;
    let messages_today = get_messages(channel_id, &ctx).await;

    let summary_text: String;
    if messages_today.len() > 1 {
        let formatted_messages: String = string_format_today_messages(&messages_today);
        let msg_hx_path = write_messages_to_txt(&formatted_messages, &file_id);
        if let Err(e) = run_python(&msg_hx_path, &task_prompt).await {
            return Err(anyhow::anyhow!("Running python script failed: {}", e));
        }
        let model_response = match read_json(&response_path) {
            Ok(data) => data,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to read JSON ({}): {}", &response_path, e));
            }
        };
        summary_text = format_json_to_message(&model_response, &channel_name);
    } else {
        summary_text = "No messages found to summarize...".to_string();
    }

    deliver_chat_summary(response_target, ctx, &summary_text).await
}

pub async fn summarize_article(file_id: Uuid, ctx: &Context, response_target: &ResponseTarget, article_url: String, original_msg: Option<&Message>) -> Result<()> {
    let article_text = tokio::task::spawn_blocking(move || {
        fetch_article_text(&article_url)
    }).await??;

    let article_path = write_article_to_txt(&article_text, &file_id);
    let task_prompt = "ARTICLE_SUMMARIZATION";

    if let Err(e) = run_python(&article_path, task_prompt).await {
        return Err(anyhow::anyhow!("Running python script failed: {}", e));
    }

    let response_path = format!("jobs/{}/model_response.json", file_id);
    let model_response = match read_json(&response_path) {
        Ok(data) => data,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to read JSON ({}): {}", &response_path, e));
        }
    };

    let summary = match model_response.get("summary") {
        Some(val) => val.clone(),
        None => return Err(anyhow::anyhow!("No 'summary' key in model response")),
    };

    let summary_text = if summary.len() > 2000 {
        format!("{}...", &summary[..1997])
    } else {
        summary
    };

    deliver_article_summary(response_target, ctx, original_msg, &summary_text).await
}
