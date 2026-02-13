# Albert
AI chatbot for automating tasks in Discord and improving user experience.

## TLDR
| Emoji | Discord Name | What It Does |
|-------|-------------|--------------|
| 📄 | `:page_facing_up:` | DMs you a summary of today's chat |
| 📑 | `:bookmark_tabs:` | DMs you a per-user summary of today's chat |
| 📖 | `:open_book:` | Replies with a bullet-point summary of a linked article |

## Architecture
Hybrid Rust/Python application with an event-driven worker queue pattern.

- **rust_bot/** — Discord bot built with Serenity. Handles events, job dispatch, and Discord API interactions.
- **python_llm/** — LLM pipeline using Mistral-7B (4-bit quantized) via LangChain and HuggingFace for text summarization.

## Features

### Chat Summarization
React to any message with an emoji to get a summary of the channel's chat history for the day, delivered as a DM.

| Emoji | Name | Behavior |
|-------|------|----------|
| 📄 | `:page_facing_up:` | Standard summary of the day's discussion |
| 📑 | `:bookmark_tabs:` | Per-user summary attributing points to each participant |

### Article Summarization
React with 📖 (`:open_book:`) on a message containing a URL. Albert fetches the article, summarizes it, and replies to the original message. Summaries are formatted with a 📌 Main Takeaway and 📋 Key Points as bullet points.

- Deduplication: Albert reacts with 📖 after posting a summary. If the bot's reaction is already present, subsequent triggers are skipped.
- Handles paywall/JS-only pages gracefully (logs error, skips silently).

## How It Works
1. Rust bot listens for emoji reactions on messages
2. Reaction triggers a `Job` dispatched via a tokio MPSC channel to a worker
3. Worker processes the job (fetches messages or article content, writes to temp file)
4. Python subprocess loads Mistral-7B and generates a summary
5. Rust bot reads the model's JSON response and sends it to Discord (DM or reply)
6. Temp job directory is cleaned up

## Environment Variables
| Variable | Description |
|----------|-------------|
| `DISCORD_TOKEN` | Discord bot token |
| `MISTRAL_TOKEN` | HuggingFace token for Mistral model access |

## Running
```bash
# With Docker
docker build -t albert .
docker run --env-file .env albert

# Local development
cd rust_bot && cargo run
```
