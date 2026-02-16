# Albert
AI Discord bot that summarizes chat history and articles using AWS Bedrock (Claude 3.5 Haiku). Serverless architecture deployed via Terraform.

## TLDR
| Command | What It Does |
|---------|--------------|
| `/summary-24hr` | Sends you an ephemeral summary of today's chat |
| `/summary-peruser` | Sends you an ephemeral per-user summary of today's chat |
| `/summary-article url:<link>` | Sends you an ephemeral bullet-point summary of a linked article |
| Right-click message → Apps → "Summarize Article" | Replies with a bullet-point summary visible to the channel |

## Architecture
Serverless two-Lambda pattern on AWS, deployed via Terraform.

```
Discord (slash cmd / context menu)
  │ HTTP POST
  v
API Gateway (HTTP API)
  │ POST /discord-interactions
  v
Lambda A — Gateway (Rust)
  │ Verify signature, ACK within 3s, invoke Lambda B async
  v
Lambda B — Worker (Rust)
  │ Fetch messages/articles, call Bedrock, edit deferred response
  ├──> AWS Bedrock (Claude 3.5 Haiku)
  └──> DynamoDB (articles, summaries, dm-sessions)
```

**Why two Lambdas:** Discord requires a response within 3 seconds. Bedrock inference takes longer. Lambda A immediately returns a deferred response ("Bot is thinking..."), then asynchronously invokes Lambda B which does the actual work and posts the result via Discord's followup webhook.

A legacy Serenity Gateway bot (`rust_bot/`) is kept as a local dev fallback.

## Features

### Chat Summarization
Use slash commands to get a summary of the channel's chat history, delivered as an ephemeral message (only you see it).

| Command | Behavior |
|---------|----------|
| `/summary-24hr` | Standard summary of the day's discussion |
| `/summary-peruser` | Per-user summary attributing points to each participant |

### Article Summarization
Two ways to summarize articles:

- **Context menu** (right-click a message → Apps → "Summarize Article"): Visible reply to the channel. Bot adds a checkmark reaction as a dedup marker.
- **Slash command** (`/summary-article url:<link>`): Ephemeral response only you see.

Summaries are formatted with a main takeaway and key bullet points. Handles paywall/JS-only pages gracefully.

## Project Structure
```
Albert/
├── lambda_gateway/        # Lambda A — signature verify, deferred ACK, async invoke
├── lambda_worker/         # Lambda B — Bedrock calls, Discord followup, article fetch
├── rust_bot/              # Legacy Serenity Gateway bot (local dev fallback)
├── infrastructure/        # Terraform — API Gateway, Lambdas, DynamoDB, IAM, CloudWatch
└── roadmap/               # Implementation roadmap and planning docs
```

## Environment Variables
| Variable | Where | Description |
|----------|-------|-------------|
| `DISCORD_PUBLIC_KEY` | Lambda A | Discord app public key (signature verification) |
| `WORKER_FUNCTION_NAME` | Lambda A | Lambda B function name (auto-set by Terraform) |
| `DISCORD_BOT_TOKEN` | Lambda B | Discord bot token (API calls) |
| `DISCORD_APPLICATION_ID` | Lambda B | Discord app ID (interaction callbacks) |
| `RUST_LOG` | Both | Log level (set to `info`) |
| `DISCORD_TOKEN` | Local `.env` | For legacy `rust_bot` mode |

## Deploying
```bash
# 1. Build Lambda binaries
cargo lambda build --release --output-format zip -p lambda_gateway -p lambda_worker

# 2. Deploy infrastructure
cd infrastructure && terraform init
terraform apply \
  -var="discord_public_key=$DISCORD_PUBLIC_KEY" \
  -var="discord_bot_token=$DISCORD_TOKEN" \
  -var="discord_application_id=$DISCORD_APP_ID"

# 3. Copy the api_gateway_url output
# 4. Paste into Discord Developer Portal → General Information → Interactions Endpoint URL
```

### Prerequisites
- AWS credentials configured (`aws configure`)
- [cargo-lambda](https://www.cargo-lambda.info/) installed
- [Terraform](https://www.terraform.io/) >= 1.5
- Claude 3.5 Haiku enabled in [AWS Bedrock console](https://console.aws.amazon.com/bedrock/) under Model Access

## Local Development
```bash
# Run the legacy Serenity Gateway bot
cd rust_bot && cargo run
```
Requires `.env` with `DISCORD_TOKEN` and AWS credentials configured.
