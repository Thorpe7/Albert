# Albert — Bedrock Migration Breakdown

## Rust Learning

Key Rust patterns used throughout the migration:

- **`Arc<T>` (Atomic Reference Counting)** — Shared ownership of the `BedrockClient` across async tasks. Created once in `main.rs`, cloned into the worker thread.
- **`tokio::sync::mpsc` channels** — The legacy bot uses a multi-producer, single-consumer channel to dequeue jobs from the event handler to a background worker. This decouples event handling from long-running work.
- **Enum-based dispatch** — `Job` enum variants (`SummarizeChat`, `SummarizeArticle`) carry all context needed for execution. `ResponseTarget` enum variants decouple delivery method from trigger type.
- **`async`/`await` with Tokio** — All Discord and Bedrock calls are async. `spawn_blocking` bridges sync code (legacy `reqwest::blocking` in `rust_bot`).
- **Trait implementations** — `EventHandler` trait from Serenity implemented on `Handler` struct to hook into Discord gateway events.
- **Error handling with `anyhow`** — `Result<T>` with `anyhow::anyhow!()` for ad-hoc error messages throughout handlers and clients.
- **Cargo workspace** — Root `Cargo.toml` defines 3 members (`rust_bot`, `lambda_gateway`, `lambda_worker`) with independent dependency sets. `default-members = ["rust_bot"]` so `cargo run` defaults to legacy bot.
- **Feature flag discipline** — `reqwest` and `readability` use `default-features = false` in Lambda crates to avoid `openssl-sys` (won't compile on Lambda's AL2023 runtime). Uses `rustls-tls` instead.

**Files:** `Cargo.toml` (root workspace), `rust_bot/Cargo.toml`, `lambda_gateway/Cargo.toml`, `lambda_worker/Cargo.toml`, `rust_bot/src/main.rs`, `rust_bot/src/worker_and_job.rs`

---

## Discord Integration

How the bot communicates with Discord:

- **Two communication models exist side-by-side:**
  1. **Gateway WebSocket** (legacy `rust_bot`) — Serenity maintains a persistent connection. Discord pushes events (messages, reactions, interactions) in real-time. The bot responds via `ctx.http` (Serenity's built-in HTTP client).
  2. **HTTP Interactions** (Lambda path) — Discord POSTs interaction payloads to a webhook URL (API Gateway). No persistent connection. The bot responds via REST API calls using a custom `DiscordClient` (raw `reqwest`).

- **Deferred responses** — Discord requires a response within 3 seconds. For slow work (Bedrock inference), the bot returns a "type 5" deferred response immediately, then edits it later via `PATCH /webhooks/{app_id}/{token}/messages/@original`.

- **Ephemeral vs. visible** — Slash commands use `flags: 64` (only the invoker sees the response). Context menu responses are visible to everyone in the channel.

- **Signature verification** — Discord signs every HTTP POST with Ed25519. Lambda A verifies using `ed25519-dalek` before processing. Required for the Interactions Endpoint to pass Discord's initial validation ping.

- **Article dedup** — Bot reacts with a checkmark on summarized messages. Before summarizing, it checks if that reaction already exists (`bot_already_replied()` / reaction check in handlers).

**Files:** `rust_bot/src/handle_events.rs`, `rust_bot/src/response_target.rs`, `lambda_gateway/src/main.rs` (signature verification), `lambda_worker/src/discord_client.rs` (REST API wrapper), `lambda_worker/src/handlers.rs`

---

## Slash Commands

How commands are registered, routed, and handled:

- **Registration** — In the legacy bot, commands are registered globally in `ready()` via `Command::set_global_commands()`. In the Lambda path, commands are registered once via the Discord Developer Portal or API (not on every boot).

- **Four commands:**
  - `/summary-24hr` — standard chat summary
  - `/summary-peruser` — per-user chat summary
  - `/summary-article url:<link>` — article summarization with a required URL option
  - `Summarize Article` — context menu (right-click a message, appears under "Apps")

- **Routing** — Both paths dispatch by `command.data.name`:
  - Legacy: `interaction_create()` in `handle_events.rs` matches command name strings and calls handler methods
  - Lambda: `lambda_worker/src/main.rs` matches command name and calls functions in `handlers.rs`

- **Handler pattern** — Each handler: extracts parameters from the interaction payload, does the work (fetch messages/article, call Bedrock), and delivers the result (edit deferred response or DM).

**Files:** `rust_bot/src/handle_events.rs` (registration + legacy routing), `lambda_worker/src/main.rs` (Lambda routing), `lambda_worker/src/handlers.rs` (Lambda handler implementations)

---

## Terraform Implementation

Infrastructure defined as code in `infrastructure/`:

- **API Gateway** — HTTP API with a single `POST /discord-interactions` route. This is the public URL that Discord sends interaction payloads to. The output `api_gateway_url` is what you paste into the Discord Developer Portal.

- **Two Lambda functions** with separate IAM roles following least-privilege:
  - Gateway (Lambda A): can invoke the Worker Lambda and write CloudWatch logs
  - Worker (Lambda B): can call Bedrock (`InvokeModel`), CRUD DynamoDB `albert-*` tables, and write CloudWatch logs

- **DynamoDB tables** — Three tables provisioned (PAY_PER_REQUEST, TTL-enabled) for future use: `albert-articles`, `albert-summaries`, `albert-dm-sessions`

- **CloudWatch log groups** — 14-day retention for both Lambdas

- **Variables** — Discord secrets (`public_key`, `bot_token`, `application_id`) passed in at apply time. Zip paths default to `cargo lambda build` output locations.

- **Deploy flow:** `cargo lambda build` compiles Rust to Lambda-compatible zips, then `terraform apply` provisions/updates everything.

**Files:** `infrastructure/main.tf`, `infrastructure/variables.tf`, `infrastructure/outputs.tf`

---

## AWS Bedrock Integration

How the bot calls Claude 3.5 Haiku for inference:

- **Converse API** — Uses AWS SDK's `converse()` method (not `invoke_model()`). Takes structured messages: a system prompt (`SystemContentBlock::Text`) and user content (`Message` with `ConversationRole::User`). Returns plain text.

- **Model ID** — `us.anthropic.claude-3-5-haiku-20241022-v1:0`. The `us.` prefix is an inference profile that enables cross-region routing. Using the raw model ID without it gives a `ValidationException`.

- **Three system prompts** — `STANDARD_SUMMARY_SYSTEM`, `PER_USER_SUMMARY_SYSTEM`, `ARTICLE_SUMMARY_SYSTEM`. Each instructs Claude to return plain text only (no JSON, no markdown fences).

- **Inference config** — `max_tokens: 2048`, `temperature: 0.5`

- **`BedrockClient` struct** — Wraps the AWS SDK client. Two public methods: `summarize_chat(messages, task_prompt)` and `summarize_article(article_text)`. Both call a private `invoke()` method that builds the request and extracts the text response.

- **Identical in both crates** — `rust_bot/src/bedrock_client.rs` and `lambda_worker/src/bedrock_client.rs` are the same logic (copy, not shared crate).

**Files:** `rust_bot/src/bedrock_client.rs`, `lambda_worker/src/bedrock_client.rs`

---

## AWS Lambda Integration

The two-Lambda serverless pattern:

- **Why two Lambdas?** — Discord demands a response in 3 seconds. Bedrock inference takes longer. Lambda freezes the process after returning a response, so you can't do background work in a single Lambda. Lambda A responds immediately, Lambda B works asynchronously.

- **Lambda A (Gateway)** — Receives the HTTP POST from API Gateway. Verifies the Discord signature. Responds to pings. For application commands: invokes Lambda B asynchronously (`InvocationType::Event` = fire-and-forget), then returns a deferred response to Discord. 5s timeout, 128MB.

- **Lambda B (Worker)** — Receives the raw interaction payload from Lambda A. Creates `BedrockClient` and `DiscordClient` per invocation. Routes by command name to handler functions. Does the actual work (fetch messages, call Bedrock, edit the deferred response). On error, edits the deferred response with an error message so users don't see infinite "thinking...". 60s timeout, 256MB.

- **`DiscordClient`** — Custom REST client replacing Serenity's `ctx.http`. Methods for fetching messages, channels, editing webhook responses, and adding reactions. Uses `reqwest` with `rustls-tls`.

- **Runtime** — `provided.al2023` (custom runtime). The Rust binary is compiled as `bootstrap` by `cargo lambda build`.

**Files:** `lambda_gateway/src/main.rs`, `lambda_worker/src/main.rs`, `lambda_worker/src/handlers.rs`, `lambda_worker/src/discord_client.rs`, `lambda_worker/src/bedrock_client.rs`, `lambda_worker/src/article_handler.rs`

---

## Feedback Loop

*Placeholder for future exploration — acceptance criteria & testing frameworks.*

- How to verify each component works end-to-end (local dev with `cargo lambda`, SAM CLI, live deploy)
- Unit testing strategy for `BedrockClient`, `DiscordClient`, webhook verification, DynamoDB operations
- Integration testing: Lambda routing, command dispatch, deferred response editing
- Acceptance criteria: what "working" means for each command and interaction type
- Rust testing tools and patterns (`#[tokio::test]`, mocking AWS SDK clients, test fixtures)
