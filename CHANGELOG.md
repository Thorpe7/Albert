# Changelog

## Unreleased

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
