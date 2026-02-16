use std::io::Cursor;

use anyhow::{anyhow, Result};

pub fn extract_url(content: &str) -> Option<String> {
    content
        .split_whitespace()
        .find(|token| token.starts_with("http://") || token.starts_with("https://"))
        .map(|s| s.to_string())
}

pub fn fetch_article_text(url_str: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url_str)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .send()
        .map_err(|e| anyhow!("Failed to fetch article: {}", e))?;

    let html = response
        .text()
        .map_err(|e| anyhow!("Failed to read response body: {}", e))?;

    let url = reqwest::Url::parse(url_str)
        .map_err(|e| anyhow!("Invalid URL: {}", e))?;

    let mut cursor = Cursor::new(html.into_bytes());
    let product = readability::extractor::extract(&mut cursor, &url)
        .map_err(|e| anyhow!("Failed to extract article content: {}", e))?;

    if product.text.trim().is_empty() {
        return Err(anyhow!("Article content is empty (possibly paywalled or JS-only)"));
    }

    Ok(product.text)
}
