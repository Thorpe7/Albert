use anyhow::{anyhow, Result};
use serde::Deserialize;

const DISCORD_API: &str = "https://discord.com/api/v10";

pub struct DiscordClient {
    http: reqwest::Client,
    bot_token: String,
    application_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DiscordMessage {
    pub id: String,
    pub author: DiscordAuthor,
    pub content: String,
    pub timestamp: String,
    #[serde(default)]
    pub reactions: Vec<DiscordReaction>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordAuthor {
    pub username: String,
    #[serde(default)]
    pub bot: bool,
}

#[derive(Debug, Deserialize)]
pub struct DiscordReaction {
    pub emoji: DiscordEmoji,
    pub me: bool,
}

#[derive(Debug, Deserialize)]
pub struct DiscordEmoji {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordChannel {
    pub name: Option<String>,
}

impl DiscordClient {
    pub fn new(bot_token: String, application_id: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            bot_token,
            application_id,
        }
    }

    pub async fn get_messages(&self, channel_id: &str, limit: u32) -> Result<Vec<DiscordMessage>> {
        let url = format!("{}/channels/{}/messages?limit={}", DISCORD_API, channel_id, limit);
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch messages: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Discord GET messages failed ({}): {}", status, body));
        }

        resp.json::<Vec<DiscordMessage>>()
            .await
            .map_err(|e| anyhow!("Failed to parse messages: {}", e))
    }

    pub async fn get_channel_name(&self, channel_id: &str) -> Result<String> {
        let url = format!("{}/channels/{}", DISCORD_API, channel_id);
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch channel: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Discord GET channel failed ({}): {}", status, body));
        }

        let channel: DiscordChannel = resp.json()
            .await
            .map_err(|e| anyhow!("Failed to parse channel: {}", e))?;

        channel.name.ok_or_else(|| anyhow!("Channel has no name (possibly a DM channel)"))
    }

    pub async fn get_message(&self, channel_id: &str, message_id: &str) -> Result<DiscordMessage> {
        let url = format!("{}/channels/{}/messages/{}", DISCORD_API, channel_id, message_id);
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch message: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Discord GET message failed ({}): {}", status, body));
        }

        resp.json::<DiscordMessage>()
            .await
            .map_err(|e| anyhow!("Failed to parse message: {}", e))
    }

    pub async fn edit_original_response(&self, interaction_token: &str, content: &str) -> Result<()> {
        let url = format!(
            "{}/webhooks/{}/{}/messages/@original",
            DISCORD_API, self.application_id, interaction_token
        );
        let body = serde_json::json!({"content": content});

        let resp = self.http.patch(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to edit original response: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Discord PATCH response failed ({}): {}", status, body));
        }

        Ok(())
    }

    pub async fn add_reaction(&self, channel_id: &str, message_id: &str, emoji: &str) -> Result<()> {
        let encoded_emoji = urlencoding::encode(emoji);
        let url = format!(
            "{}/channels/{}/messages/{}/reactions/{}/@me",
            DISCORD_API, channel_id, message_id, encoded_emoji
        );

        let resp = self.http.put(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Length", "0")
            .send()
            .await
            .map_err(|e| anyhow!("Failed to add reaction: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Discord PUT reaction failed ({}): {}", status, body));
        }

        Ok(())
    }
}
