# Changelog

## Unreleased

## 2026-02-16 — AWS Migration (Phases 1-4)

### Added
- **Terraform infrastructure** (`infrastructure/`) — API Gateway (HTTP), two Lambdas, 3 DynamoDB tables, IAM roles, CloudWatch
- **Lambda Gateway** (`lambda_gateway/`) — Discord signature verification, deferred ACK, async worker invocation
- **Lambda Worker** (`lambda_worker/`) — Bedrock summarization, Discord REST API client, article fetching
- **`BedrockClient`** using AWS Bedrock Converse API with Claude 3.5 Haiku
- **Slash commands**: `/summary-24hr`, `/summary-peruser`, `/summary-article`
- **Context menu**: "Summarize Article" (right-click message)
- **`DiscordClient`** — reqwest-based REST API wrapper for Lambda environment
- **Deploy workflow** via `cargo lambda build` + `terraform apply`

### Changed
- **Interaction model** — Slash commands + context menu replace emoji reactions as primary triggers
- **LLM backend** — AWS Bedrock (Claude 3.5 Haiku) replaces local Python/Mistral-7B
- **Summary delivery** — Ephemeral responses for slash commands, visible reply for context menu
- **Article fetching** — Async `reqwest::Client` replaces `reqwest::blocking::Client` in Lambda
- **Dependencies** — `reqwest` and `readability` use `default-features = false` (rustls instead of OpenSSL)

### Removed
- `python_llm/` directory (Mistral-7B, LangChain, HuggingFace)
- `rust_bot/src/python_runner.rs` (Python subprocess execution)
- `rust_bot/src/read_and_write.rs` (file-based IPC)
- JSON output parsing / 3-level fallback parser (Claude returns clean plain text)

## 2026-02-16 — Article Summarization Fixes

### Fixed
- Article fetch blocked by news sites — added browser User-Agent header
- Summary marker changed from 📖 to ✅ to prevent re-triggering summarization

## 2026-02-13 — Article Summarization

### Added
- **Article Summarization** — React with 📖 on a message containing a URL to get a summary posted as a reply.
  - New `article_handler` module with URL extraction, article fetching (via `readability` crate), and dedup check
  - New `SummarizeArticle` job variant in the worker queue
  - `summarize_article()` orchestrator in `bot_functions.rs`
  - `write_article_to_txt()` in `read_and_write.rs`
  - `ARTICLE_SUMMARIZATION` prompt in `prompts.py` — outputs 📌 Main Takeaway + 📋 Key Points as bullet points
  - `reqwest` and `readability` dependencies added to `rust_bot/Cargo.toml`
- **Deduplication via bot reaction** — Albert reacts with 📖 on the original message after posting a summary. The `bot_already_replied()` check prevents duplicate summaries by looking for the bot's own reaction (`me: true`).

### Changed
- **Article summary delivery** — Changed from creating a thread to replying directly to the original message (less noisy).
- **Model output parser** — `clean_model_output()` in `output_structures.py` is now more resilient:
  - Uses `json.JSONDecoder.raw_decode()` to ignore trailing text after JSON
  - `_normalize_response()` folds any extra keys the model generates (KEY TERMS, KEY POINTS, etc.) into the single `summary` value
  - Regex fallback for malformed JSON (e.g. markdown artifacts outside string quotes)
  - Last-resort fallback wraps plain text output as `{"summary": "..."}` instead of raising an error

### Updated
- **README** — Added architecture overview, feature descriptions, environment variables, and run instructions.
