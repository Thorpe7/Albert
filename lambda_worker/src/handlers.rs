use anyhow::{anyhow, Result};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::article_handler::{extract_url, fetch_article_text};
use crate::bedrock_client::BedrockClient;
use crate::discord_client::DiscordClient;

const SUMMARY_MARKER: &str = "\u{2705}";

// --- Discord interaction types (deserialized from Lambda A payload) ---

#[derive(Debug, Deserialize)]
pub struct Interaction {
    pub token: String,
    pub channel_id: Option<String>,
    pub data: Option<InteractionData>,
}

#[derive(Debug, Deserialize)]
pub struct InteractionData {
    pub name: Option<String>,
    #[serde(default)]
    pub options: Vec<CommandOption>,
    pub target_id: Option<String>,
    pub resolved: Option<ResolvedData>,
}

#[derive(Debug, Deserialize)]
pub struct CommandOption {
    pub name: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ResolvedData {
    #[serde(default)]
    pub messages: std::collections::HashMap<String, ResolvedMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ResolvedMessage {
    pub id: String,
    pub content: String,
}

// --- Helpers ---

fn get_24h_ago() -> OffsetDateTime {
    OffsetDateTime::now_utc() - time::Duration::hours(24)
}

fn format_messages(messages: &[crate::discord_client::DiscordMessage]) -> String {
    messages
        .iter()
        .rev()
        .map(|msg| format!("Author: **{}**; Content: {}", msg.author.username, msg.content))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_to_discord_limit(text: &str) -> String {
    if text.len() > 2000 {
        format!("{}...", &text[..1997])
    } else {
        text.to_string()
    }
}

// --- Command Handlers ---

pub async fn handle_summary_chat(
    interaction: &Interaction,
    task_prompt: &str,
    discord: &DiscordClient,
    bedrock: &BedrockClient,
) -> Result<()> {
    let channel_id = interaction.channel_id.as_deref()
        .ok_or_else(|| anyhow!("No channel_id in interaction"))?;

    let channel_name = discord.get_channel_name(channel_id).await?;
    let all_messages = discord.get_messages(channel_id, 100).await?;

    let cutoff = get_24h_ago();
    let messages_today: Vec<_> = all_messages
        .into_iter()
        .filter(|msg| {
            time::OffsetDateTime::parse(&msg.timestamp, &time::format_description::well_known::Rfc3339)
                .map(|ts| ts >= cutoff)
                .unwrap_or(false)
        })
        .collect();

    let summary_text = if messages_today.len() > 1 {
        let formatted = format_messages(&messages_today);
        let summary = bedrock.summarize_chat(&formatted, task_prompt).await?;
        format!("**Channel: **{}\n{}", channel_name, summary)
    } else {
        "No messages found to summarize...".to_string()
    };

    let response = truncate_to_discord_limit(&summary_text);
    discord.edit_original_response(&interaction.token, &response).await
}

pub async fn handle_summary_article_slash(
    interaction: &Interaction,
    discord: &DiscordClient,
    bedrock: &BedrockClient,
) -> Result<()> {
    let data = interaction.data.as_ref()
        .ok_or_else(|| anyhow!("No data in interaction"))?;

    let url = data.options.iter()
        .find(|o| o.name == "url")
        .and_then(|o| o.value.as_ref())
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No URL provided in slash command"))?
        .to_string();

    let article_text = fetch_article_text(&url).await?;

    let summary = bedrock.summarize_article(&article_text).await?;
    let response = truncate_to_discord_limit(&summary);
    discord.edit_original_response(&interaction.token, &response).await
}

pub async fn handle_summary_article_context_menu(
    interaction: &Interaction,
    discord: &DiscordClient,
    bedrock: &BedrockClient,
) -> Result<()> {
    let channel_id = interaction.channel_id.as_deref()
        .ok_or_else(|| anyhow!("No channel_id in interaction"))?;

    let data = interaction.data.as_ref()
        .ok_or_else(|| anyhow!("No data in interaction"))?;

    let target_id = data.target_id.as_deref()
        .ok_or_else(|| anyhow!("No target_id in context menu interaction"))?;

    // Fetch the target message to check reactions (dedup)
    let target_msg = discord.get_message(channel_id, target_id).await?;

    let already_summarized = target_msg.reactions.iter().any(|r| {
        r.me && r.emoji.name.as_deref() == Some(SUMMARY_MARKER)
    });

    if already_summarized {
        discord.edit_original_response(
            &interaction.token,
            "This article has already been summarized.",
        ).await?;
        return Ok(());
    }

    // Extract URL from message content, or from resolved data as fallback
    let url = extract_url(&target_msg.content)
        .or_else(|| {
            data.resolved.as_ref()
                .and_then(|r| r.messages.get(target_id))
                .and_then(|m| extract_url(&m.content))
        })
        .ok_or_else(|| anyhow!("No URL found in the target message"))?;

    let article_text = fetch_article_text(&url).await?;

    let summary = bedrock.summarize_article(&article_text).await?;
    let response = truncate_to_discord_limit(&summary);

    discord.edit_original_response(&interaction.token, &response).await?;
    discord.add_reaction(channel_id, target_id, SUMMARY_MARKER).await?;

    Ok(())
}
