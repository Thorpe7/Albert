use anyhow::{anyhow, Result};
use serenity::model::channel::Message;
use serenity::model::prelude::ReactionType;

pub const SUMMARY_MARKER: &str = "\u{1F4D6}";

pub fn extract_url(content: &str) -> Option<String> {
    content
        .split_whitespace()
        .find(|token| token.starts_with("http://") || token.starts_with("https://"))
        .map(|s| s.to_string())
}

pub fn fetch_article_text(url: &str) -> Result<String> {
    let product = readability::extractor::scrape(url)
        .map_err(|e| anyhow!("Failed to fetch article: {}", e))?;

    if product.text.trim().is_empty() {
        return Err(anyhow!("Article content is empty (possibly paywalled or JS-only)"));
    }

    Ok(product.text)
}

pub fn bot_already_replied(msg: &Message) -> bool {
    msg.reactions.iter().any(|r| {
        r.me && matches!(&r.reaction_type, ReactionType::Unicode(s) if s == SUMMARY_MARKER)
    })
}
