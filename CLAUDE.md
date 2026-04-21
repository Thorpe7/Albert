# Albert — Claude Code Context

## TODO

### Feature 1: Article Summarization — DONE (2026-02-13)
- [x] Article extraction, summarization, and reply-based delivery
- [x] Dedup via bot ✅ reaction (migrating to DynamoDB)
- [x] Fix article fetch blocked by news sites (User-Agent fix, 2026-02-16)
- [x] Change summary marker from 📖 to ✅ (2026-02-16)
- [ ] Unit tests for article handler and output parser

### Feature 3: AWS Migration — IN PROGRESS (`bedrock-migration` branch)
- [x] **Phase 1: Slash Commands** — Register `/summary-24hr`, `/summary-perUser`, `/summary-article`, and `Summarize Article` context menu. Implement interaction handlers in Serenity for local testing. (2026-02-16)
- [x] **Phase 2: Bedrock Integration** — Replace Python/Mistral-7B with Rust `BedrockClient` using Claude 3.5 Haiku. Remove `python_llm/`, `python_runner.rs`, file-based IPC. (2026-02-16)
- [x] **Phase 3: Lambda Conversion** — Create two-Lambda pattern (Gateway + Worker). Implement Discord signature verification. Test with `cargo lambda` / SAM CLI. (2026-02-16)
- [x] **Phase 4: Infrastructure Deploy** — Terraform for API Gateway (HTTP), both Lambdas, DynamoDB tables. Configure Discord Interactions Endpoint URL. CloudWatch monitoring. (2026-02-16)
- [ ] **Phase 5: Dashboard & Monitoring** — Structured logging (Bedrock tokens/latency/cost), CloudWatch dashboard and alarms, DynamoDB per-guild usage tracking (`albert-usage` table), admin-only `/stats` slash command.
- [ ] **Phase 6: Optimization** — Cold start tuning, Bedrock prompt refinement, summary caching in DynamoDB, rate limiting.
- [ ] **Phase 7: Unit Tests** — BedrockClient, webhook verification, DynamoDB state manager, Lambda routing, integration tests.

### Feature 2: Interactive Q&A in DMs — NOT STARTED (depends on Feature 3)
- [ ] DM session management via DynamoDB
- [ ] Q&A prompt with conversation context for Bedrock
- [ ] Context window management (sliding window + token budget)
- [ ] Session expiry and UX

See `roadmap/albert_roadmap.md` for full details. See `implement_strats.md` for code examples.

---

## What This Is
Discord bot that summarizes chat history and articles using AWS Bedrock (Claude 3.5 Haiku). Serverless two-Lambda architecture deployed via Terraform.

## Architecture
- **Serverless two-Lambda pattern**: Discord HTTP POST → API Gateway → Lambda A (gateway) → async invoke → Lambda B (worker) → Bedrock → Discord webhook
- **Lambda A** (`lambda_gateway`): Signature verification, interaction routing, deferred response, async invocation of Lambda B
- **Lambda B** (`lambda_worker`): Fetches messages/articles, calls Bedrock, edits deferred response via Discord API
- **Legacy mode** (`rust_bot`): Serenity gateway bot with MPSC worker queue (still works for local dev)

## Key File Map

### Lambda Gateway (`lambda_gateway/src/`)
| File | Purpose |
|------|---------|
| `main.rs` | Entry point, signature verify, ping/pong, route interactions, async invoke worker |

### Lambda Worker (`lambda_worker/src/`)
| File | Purpose |
|------|---------|
| `main.rs` | Entry point, env setup, routes to handlers |
| `handlers.rs` | `handle_summary_chat()`, `handle_summary_article_slash()`, `handle_summary_article_context_menu()` |
| `bedrock_client.rs` | `BedrockClient` — Converse API wrapper for Claude 3.5 Haiku |
| `discord_client.rs` | `DiscordClient` — REST API calls (messages, channels, webhooks, reactions) |
| `usage_tracker.rs` | `UsageTracker` — DynamoDB per-guild usage tracking and monthly aggregation |

### Rust Bot — Legacy (`rust_bot/src/`)
| File | Purpose |
|------|---------|
| `main.rs` | Entry point, intents, MPSC channel setup, spawns worker |
| `handle_events.rs` | `EventHandler` impl — routes emoji reactions and slash commands to jobs |
| `worker_and_job.rs` | `Job` enum, worker loop |
| `bot_functions.rs` | `summarize_chat()`, `summarize_article()` orchestrators |
| `bedrock_client.rs` | `BedrockClient` — same Converse API pattern as lambda_worker |
| `article_handler.rs` | `extract_url()`, `fetch_article_text()`, `bot_already_replied()`, `SUMMARY_MARKER` |
| `response_target.rs` | `ResponseTarget` enum, delivery helpers |
| `message_utils.rs` | Discord helpers: get messages, format JSON, get channel name |

### Infrastructure (`infrastructure/`)
| File | Purpose |
|------|---------|
| `main.tf` | Provider, IAM roles, Lambda functions, API Gateway, DynamoDB, CloudWatch |
| `variables.tf` | Input variables (secrets, config, zip paths) |
| `outputs.tf` | API Gateway URL (for Discord Developer Portal) |

## Interaction Triggers
| Trigger | Command | Delivery |
|---------|---------|----------|
| `/summary-24hr` | Standard chat summary | Ephemeral (user only) |
| `/summary-peruser` | Per-user chat summary | Ephemeral (user only) |
| `/summary-article url:<link>` | Article summarization | Ephemeral (user only) |
| Right-click → Apps → "Summarize Article" | Article summarization | Visible reply to message |
| `/stats` | Server usage stats (admin only) | Ephemeral (admin only) |

Legacy emoji triggers (📄, 📑, 📖) still work in `rust_bot` mode.

## Important Patterns

### BedrockClient uses Converse API
System prompt + user content as structured messages. Claude returns plain text — no JSON wrapping or fallback parser needed.

### Model ID requires inference profile prefix
`us.anthropic.claude-3-5-haiku-20241022-v1:0` — the `us.` prefix enables cross-region routing. Raw model ID gives `ValidationException`.

### Lambda dependencies must avoid openssl-sys
`reqwest` and `readability` need `default-features = false` to use rustls instead of native-tls/OpenSSL.

### Article dedup uses bot's own reaction (legacy + context menu)
`bot_already_replied()` checks `msg.reactions` for `r.me == true` with ✅. Moving to DynamoDB in Phase 6.

## Environment
- Requires `.env` with `DISCORD_TOKEN` (for legacy bot) and AWS credentials configured
- Legacy bot: `cd rust_bot && cargo run`
- Lambda deploy workflow:
  ```bash
  cargo lambda build --release --output-format zip -p lambda_gateway -p lambda_worker
  cd infrastructure && terraform init
  terraform apply \
    -var="discord_public_key=$DISCORD_PUBLIC_KEY" \
    -var="discord_bot_token=$DISCORD_TOKEN" \
    -var="discord_application_id=$DISCORD_APP_ID"
  # Copy api_gateway_url output → Discord Developer Portal → Interactions Endpoint URL
  ```

## Branches
| Branch | Purpose |
|--------|---------|
| `main` | Production — merged worker queue + article summarization |
| `bedrock-migration` | Active — swapping Mistral-7B for AWS Bedrock |

## Roadmap
See `roadmap/albert_roadmap.md` for planned features:
- Feature 1: Article Summarization — **implemented** (2026-02-13)
- Feature 2: Interactive Q&A in DMs — not started
- Feature 3: AWS Migration (Bedrock + Lambda) — in progress on `bedrock-migration`

## User Preferences
- Strongly typed Python (type hints for params and return types)
