# Albert Bot - Implementation Roadmap

## Executive Summary
This roadmap outlines the implementation plan for three major enhancements to the Albert Discord bot:
1. Article summarization ~~via emoji reactions~~ (implemented 2026-02-13, migrating to slash commands/context menu)
2. Interactive Q&A in DMs — not started
3. Migration from local deployment to AWS Lambda + API Gateway + Bedrock

---

## ~~Feature 1: Article Summarization~~ (Implemented 2026-02-13)

### Overview
Enable users to summarize linked articles by adding an emoji reaction. ~~The summary will be posted as a thread on the original message~~, with deduplication to prevent re-summarizing the same link.

> **Implementation Notes:** Implemented with a simpler architecture than originally planned. Summary is posted as a reply (not a thread — too noisy). Deduplication uses the bot's own ✅ reaction on the original message (no Redis/DynamoDB needed — will migrate to DynamoDB in Feature 3). Article fetching uses `reqwest::blocking::Client` with browser User-Agent + `readability::extractor::extract()`. Prompt added to existing `prompts.py` rather than a separate file.

### Architecture Changes Required

#### ~~1.1 State Management Layer~~ (Implemented 2026-02-13)
**Problem:** Need to track which URLs have been summarized to avoid duplication.

**Implemented Solution (current):** Bot reacts with ✅ on the original message after posting the summary. Dedup check uses `msg.reactions` with `me: true` — zero extra API calls, persists on Discord's side, survives bot restarts. No external cache needed.

> **Note:** This will be superseded by DynamoDB-based dedup (URL hash lookup in `albert-articles` table) during the AWS migration (Feature 3). Required because `/summary-article` slash command has no target message to react to, and dedup needs to work across servers.

<details>
<summary>Original options considered</summary>

- **Option A (Lightweight):** In-memory cache with Redis
  - TTL-based expiration (e.g., 7 days)
  - Key: `article_summary:{message_id}` or `article_summary:{url_hash}`
  - Value: Summary text + metadata

- **Option B (Persistent):** DynamoDB table
  - Partition key: `url_hash` (SHA256 of URL)
  - Attributes: `summary`, `message_id`, `channel_id`, `timestamp`, `thread_id`
  - GSI on `message_id` for quick lookups

**Recommendation:** Start with Option A (Redis) for MVP, migrate to Option B for production scale.
</details>

#### ~~1.2 URL Extraction & Article Fetching~~ (Implemented 2026-02-13)
**Rust Component (`rust_bot/src/article_handler.rs`):**

Implemented as free functions (no struct needed without Redis):
- `extract_url(content: &str) -> Option<String>` — finds first HTTP(S) URL
- `fetch_article_text(url: &str) -> Result<String>` — fetches HTML via `reqwest::blocking::Client` with browser User-Agent, then extracts content with `readability::extractor::extract()` (updated 2026-02-16, was `scrape()` which got blocked by news sites)
- `bot_already_replied(msg: &Message) -> bool` — checks bot's own ✅ reaction (updated 2026-02-16, was 📖 which retriggered summarization)

**Dependencies added:**
- `reqwest` (with `rustls-tls`, `blocking`) - HTTP client for article fetching with custom User-Agent
- `readability` - HTML parsing and article content extraction

**Not needed:** `url` crate (reqwest re-exports `url::Url`), `redis` (bot reaction dedup)

#### ~~1.3 Updated Event Handler~~ (Implemented 2026-02-13)
**Modified `rust_bot/src/handle_events.rs`:**

Added `📖` (`\u{1F4D6}`) branch in `reaction_add()`:
1. Fetch the original message
2. Check `bot_already_replied()` — if true, skip
3. Call `extract_url()` — if None, skip
4. Dispatch `Job::SummarizeArticle` to worker queue

#### ~~1.4 Summary Delivery~~ (Implemented 2026-02-13)
Originally planned as thread creation. Changed to **reply to original message** (less noisy) + bot ✅ reaction as dedup marker (updated 2026-02-16, was 📖). Implemented in `summarize_article()` in `rust_bot/src/bot_functions.rs`.

#### ~~1.5 Python LLM Updates~~ (Implemented 2026-02-13)
Added `ARTICLE_SUMMARIZATION` prompt to existing `python_llm/src/utils/prompts.py` (no separate file needed). Outputs main takeaway + key points as bullet points.

Also hardened `python_llm/src/utils/output_structures.py` parser with 3 fallback levels to handle Mistral-7B's frequent malformed JSON output:
1. `raw_decode()` — ignores trailing text after JSON
2. Regex fallback — handles markdown artifacts outside string quotes
3. Plain text fallback — wraps raw output as `{"summary": "..."}`

### ~~Implementation Steps~~ (Completed 2026-02-13)

**Phase 1: Foundation**
1. ~~Add Redis/DynamoDB client to Rust project~~ → Replaced with bot reaction dedup (no external store)
2. ~~Implement URL extraction from Discord messages~~ ✅
3. ~~Add HTTP client and article fetching logic~~ ✅ (via `readability` crate)
4. ~~Create caching layer with TTL~~ → Not needed (bot reaction dedup)

**Phase 2: Integration**
5. ~~Implement article content extraction (HTML → text)~~ ✅
6. ~~Update Python prompt for article summarization~~ ✅
7. ~~Modify file I/O to handle article content~~ ✅ (`write_article_to_txt()`)
8. ~~Add thread creation functionality~~ → Changed to reply-based delivery ✅

**Phase 3: Polish**
9. ~~Add error handling for bad URLs, timeouts, paywalls~~ ✅
10. ~~Implement deduplication checks~~ ✅ (bot ✅ reaction, migrating to DynamoDB)
11. Add logging and monitoring — basic `println`/`eprintln` only
12. Integration testing with various article types — manual testing done

**Phase 4: Unit Tests**
13. Unit tests for `extract_url()` — valid URLs, no URLs, multiple URLs, URLs with surrounding text
14. Unit tests for `bot_already_replied()` — no reactions, other reactions, bot's own marker reaction
15. Unit tests for `fetch_article_text()` — mock successful fetch, empty content, network errors
16. Unit tests for `_normalize_response()` — standard JSON, extra keys, list values, nested structures
17. Unit tests for `clean_model_output()` — valid JSON, trailing text, malformed JSON, plain text fallback
18. Integration test for full article summarization pipeline (mock Discord + HTTP)

### Edge Cases
- ~~**Paywalled articles:**~~ Returns error if extracted text is empty (logged, skipped silently) ✅
- **Multiple URLs in one message:** Currently takes first URL only. Summarize all or prompt user to choose?
- **Non-article URLs (videos, images):** Not yet handled — `readability` will fail and error is logged
- **Rate limiting:** Not yet implemented
- ~~**Thread creation failures:**~~ N/A — switched to reply-based delivery

---

## Feature 2: Interactive Q&A in DMs

### Overview
Allow users to ask follow-up questions about summaries in DM conversations, with context retention for the duration of the conversation.

### Architecture Challenges

#### 2.1 Context Retention Problem
**Challenge:** Don't want to store server messages long-term, but need context for Q&A.

**Solution Strategy - "Summary-Based Context":**
Instead of storing raw messages, store only:
1. The generated summary (already created)
2. User questions and bot responses in DM
3. TTL-based expiration

**Storage Model:** `DM_Conversation` with user_id, conversation_id, original_summary, original_context, qa_history, timestamps, and 24h TTL. See `implement_strats.md` for schema.

#### 2.2 Conversation State Management

**Option A: Stateful Session Store (DynamoDB)** — Store session with summary, context, message history, and TTL. See `implement_strats.md` for schema.

**Option B: Embed Context in Each Message (Stateless)** — Reconstruct conversation from last N DM messages. No external state needed, but higher token usage.

**Decision:** Option A (DynamoDB) for better UX, token efficiency, and serverless compatibility.

#### 2.3 Conversation Lifecycle

- **Trigger:** Bot creates DM session in DynamoDB after sending ephemeral summary
- **Continuation:** When user DMs the bot, look up active session by user_id and handle follow-up
- **Expiration:** DynamoDB TTL auto-deletes sessions after 24 hours

See `implement_strats.md` for code examples.

#### 2.4 Context Window Management

**Strategy:**
1. **Sliding window:** Keep last N question-answer pairs
2. **Token budget:** Dynamically trim based on context length
3. **Summary compression:** Re-summarize Q&A history if it grows too large

**Implementation:** Will be in Rust with Bedrock (Python LLM being removed in Feature 3). See `implement_strats.md` for conceptual code example.

### Implementation Steps

**Phase 1: DM Detection & Session Management**
1. Add DM interaction handler (slash command or button-triggered, not reaction-based)
2. Implement session storage in DynamoDB (`albert-dm-sessions` table from Feature 3)
3. Create session on initial summary send
4. Add session lookup on DM receive

**Phase 2: Q&A Pipeline**

5. Create Q&A prompt for Bedrock (Claude 3.5 Haiku)
6. Implement conversation history formatting in Rust
7. Implement context window management (sliding window + token budget)
8. Add Bedrock call with conversation context

**Phase 3: UX & Edge Cases**

9. Add session expiry notification (auto-expire after 24 hours)
10. Handle multiple concurrent conversations per user
11. Add deferred response ("thinking...") while processing
12. Graceful handling of expired sessions

**Phase 4: Unit Tests (Week 4)**

13. Unit tests for session creation, lookup, and expiration
14. Unit tests for DM message routing (guild vs DM detection)
15. Unit tests for Q&A prompt construction and context window trimming
16. Unit tests for conversation history formatting and token budget limits
17. Integration test for full DM Q&A flow (mock Discord + session store)

### Security & Privacy Considerations

**Data Retention:**
- ❌ DON'T store: Full server message history
- ✅ DO store: Summaries, user questions, bot responses
- ⏰ Expire after: 24 hours (configurable)

**Access Control:**
- Sessions scoped to specific user_id
- No cross-user data access
- Clear session data on explicit user request

### User Experience Flow

```
1. User runs /summary-24hr or /summary-perUser
   → Bot sends ephemeral summary to user
   → Bot creates DM session in DynamoDB (24h TTL)
   → Ephemeral message includes "Ask me follow-up questions in DMs!"

2. User DMs the bot: "What was the consensus on topic X?"
   → Bot looks up active session in DynamoDB
   → Bot generates answer via Bedrock with full context
   → Bot updates session history

3. User asks another question: "Any dissenting opinions?"
   → Bot has context from previous questions
   → Bot provides coherent, contextual answer

4. After 24 hours
   → Session TTL expires in DynamoDB
   → Bot notifies: "This conversation has ended. Use /summary-24hr to start fresh!"
```

---

## Feature 3: AWS Migration (Local → Bedrock + Lambda)

### Overview
Migrate from local deployment to AWS serverless architecture for scalability, cost-efficiency, and managed ML infrastructure.

### Current Architecture
```
┌─────────────────┐
│  Discord Server │
└────────┬────────┘
         │
         v
┌─────────────────────┐
│   Rust Bot (Local)  │
│  - Serenity client  │
│  - Event handling   │
└────────┬────────────┘
         │
         v
┌─────────────────────┐
│  Python LLM (Local) │
│  - Mistral 7B       │
│  - HuggingFace      │
│  - File I/O         │
└─────────────────────┘
```

### Target Architecture

**Decision:** Lambda + API Gateway (HTTP) over ECS Fargate. At low-to-moderate scale, Lambda's pay-per-invocation model costs ~$0.01-$0.65/month for compute vs Fargate's fixed ~$9/month. As scale grows, Bedrock token costs dominate either way (88%+ of total bill at 500 servers), so the infrastructure choice matters less — but Lambda gives better unit economics before the first paying customer.

**Decision:** Discord HTTP Interactions model (slash commands + message context menu) replaces Gateway WebSocket + emoji reactions. Required for Lambda (no persistent WebSocket), and slash command invocations are invisible to other channel members — better UX for a multi-server product.

**New interaction model:**
| Current (reactions) | New (interactions) |
|---|---|
| React 📄 anywhere | `/summary-24hr` slash command (ephemeral) |
| React 📑 anywhere | `/summary-perUser` slash command (ephemeral) |
| React 📖 on message with URL | Right-click message > Apps > Summarize Article (visible reply) |
| (no equivalent) | `/summary-article url:<link>` slash command (ephemeral, user-only) |

```
┌─────────────────┐
│  Discord Server │
│  (slash cmd /   │
│   context menu) │
└────────┬────────┘
         │ HTTP POST
         v
┌──────────────────────────────────────┐
│      API Gateway (HTTP API)          │
│   POST /discord-interactions         │
│   ~$1.00/1M requests                 │
└────────┬─────────────────────────────┘
         │
         v
┌──────────────────────────────────────┐
│   Lambda A — "Gateway" (Rust)        │
│   - Verify Discord signature         │
│   - ACK within 3 seconds             │
│   - Return DEFERRED response         │
│   - Invoke Lambda B async            │
└────────┬─────────────────────────────┘
         │ async invoke
         v
┌──────────────────────────────────────┐
│   Lambda B — "Worker" (Rust)         │
│   - Fetch article / channel messages │
│   - Call Bedrock for summarization   │
│   - POST followup via Discord webhook│
└────────┬─────────────────────────────┘
         │
         ├─────────────────────────────┐
         │                             │
         v                             v
┌─────────────────┐         ┌──────────────────────┐
│  DynamoDB       │         │  Bedrock Runtime API │
│  - Summaries    │         │  - Claude 3.5 Haiku  │
│  - DM Sessions  │         │  - Pay per token     │
│  - Cache        │         └──────────────────────┘
└─────────────────┘
```

**Why two Lambdas:** Discord requires an interaction response within 3 seconds. Bedrock inference takes longer. Lambda A immediately returns a deferred response ("Bot is thinking..."), then asynchronously invokes Lambda B which does the actual work and posts the result via Discord's followup webhook. This mirrors the current MPSC worker queue pattern.

### Migration Components

#### 3.1 Replace Local Mistral with AWS Bedrock

**Why Bedrock?**
- ✅ No infrastructure to manage (no GPU instances)
- ✅ Pay per token (cost-efficient for bursty workload)
- ✅ Instant scaling
- ✅ Access to multiple models (Claude, Titan, Llama)
- ✅ Built-in guardrails and content filtering

**Model Selection:**
- **Claude 3.5 Haiku (selected):** Fast, cost-effective for summarization ($1.00/1M input tokens, $5.00/1M output tokens)
- **Claude 3.5 Sonnet:** Better quality if needed ($3.00/1M input tokens, $15.00/1M output tokens)

**Code Changes Required:**
- **Remove:** `python_llm/` entire directory, `rust_bot/src/python_runner.rs`, file-based IPC
- **Add:** `BedrockClient` struct in Rust using `aws-sdk-bedrockruntime` with Claude 3.5 Haiku model ID

See `implement_strats.md` for BedrockClient code example.

#### 3.2 Rust Lambda Functions (Two-Lambda Pattern)

**Architecture Decision:** Two Lambdas — a lightweight Gateway (Lambda A) for fast ACK + signature verification, and a Worker (Lambda B) for Bedrock calls and Discord followups. Required because Discord's 3-second deadline conflicts with LLM inference time. Lambda A invokes Lambda B asynchronously via `InvocationType::Event`.

- **Lambda A (Gateway):** Verifies Discord signature, handles Ping for endpoint verification, returns deferred ACK for application commands, invokes Lambda B async
- **Lambda B (Worker):** Dispatches by command name (`summary-24hr`, `summary-perUser`, `summary-article`, `Summarize Article` context menu), calls Bedrock, posts followup via Discord webhook
- **Key dependencies:** `lambda_runtime`, `aws-sdk-lambda`, `aws-sdk-bedrockruntime`, `aws-sdk-dynamodb`, `ed25519-dalek`, `reqwest`

See `implement_strats.md` for handler code and full dependency list.

#### 3.3 Discord HTTP Interactions (Slash Commands + Context Menu)

**Current:** Bot maintains Gateway WebSocket, triggers on emoji reactions

**New:** Discord sends HTTP POST to API Gateway → Lambda on every slash command / context menu interaction. No persistent connection.

**Commands to register:**
1. `/summary-24hr` — Slash command for standard 24-hour chat summary. Ephemeral response (only invoking user sees it).
2. `/summary-perUser` — Slash command for per-user chat summary breakdown. Ephemeral response.
3. `/summary-article` — Slash command with required `url` parameter. User pastes a URL directly. **Ephemeral response (only invoking user sees it)** — prevents spam in shared channels.
4. `Summarize Article` — Message context menu command (right-click message → Apps → Summarize Article). Extracts URL from the target message. **Visible reply to original message** — shared with the whole channel so the group benefits.

Two ways to trigger article summarization with different visibility: context menu for shared summaries, slash command for personal use.

**Article dedup:** Moves from bot reaction (✅) to **DynamoDB lookup by URL hash** (`albert-articles` table). Works for both trigger paths (context menu and slash command) and across servers. The ✅ reaction is still added to the original message on the context menu path as a visual indicator, but the authoritative dedup check is DynamoDB.

**Setup Steps:**
1. Register commands via Discord API (`PUT /applications/{app_id}/commands`)
2. Configure Interactions Endpoint URL in Discord Developer Portal: `https://{api-gw-id}.execute-api.{region}.amazonaws.com/discord-interactions`
3. Lambda A verifies webhook signature on every request
4. Lambda A returns deferred ACK within 3 seconds, invokes Lambda B async

**Webhook Verification (Lambda A):** Uses `ed25519-dalek` to verify Discord's signature on every request. See `implement_strats.md` for code example.

**Ephemeral vs Visible Responses:**
- `/summary-24hr`, `/summary-perUser`, `/summary-article` — **Ephemeral** (only visible to the invoking user). Prevents channel spam.
- `Summarize Article` context menu — **Visible reply** to the original message. The whole channel sees the summary, and the bot adds a ✅ reaction as a visual indicator that the article has been summarized.

**3-Second Response — Two-Lambda Pattern:**

Lambda A returns a deferred ACK and invokes Lambda B via `InvocationType::Event` (async, fire-and-forget). Important: `tokio::spawn` won't work in Lambda — the runtime freezes after returning, so spawned tasks never complete. Lambda B posts results via Discord's interaction followup webhook (`PATCH /webhooks/{app_id}/{token}/messages/@original`).

See `implement_strats.md` for the full two-Lambda code pattern.

#### 3.4 State Management with DynamoDB

**Tables Needed:**

1. **`albert-summaries`** — Cache chat summaries. Partition key: `message_id`. TTL: 7 days.
2. **`albert-articles`** — Cache article summaries + dedup. Partition key: `url_hash`. Includes `guild_id` for multi-server tracking. TTL: configurable.
3. **`albert-dm-sessions`** — Q&A conversation state (Feature 2). Partition key: `user_id`, sort key: `session_id`. TTL: 24 hours.

All tables use pay-per-request billing. `StateManager` struct in Rust wraps the DynamoDB client for CRUD operations.

See `implement_strats.md` for full table schemas and Rust client code.

#### 3.5 Deployment & CI/CD

**Infrastructure as Code (Terraform):**

Resources in `infrastructure/main.tf`:
- **Lambda A (Gateway):** `provided.al2023` runtime, 5s timeout, 128MB memory. Env vars: `DISCORD_PUBLIC_KEY`, `WORKER_FUNCTION`
- **Lambda B (Worker):** `provided.al2023` runtime, 60s timeout, 256MB memory. Env var: `DISCORD_BOT_TOKEN`
- **API Gateway:** HTTP API (cheaper than REST), routes to Lambda A
- **DynamoDB tables:** `albert-articles`, `albert-summaries`, `albert-dm-sessions` — all pay-per-request with TTL enabled

**GitHub Actions CI/CD:**
- Trigger: push to `main`
- Build both Lambdas with `cargo build --release --target x86_64-unknown-linux-musl`
- Deploy with `terraform apply`

See `implement_strats.md` for full Terraform HCL and GitHub Actions YAML.

### Migration Steps (Phased Rollout)

**Phase 0: Preparation**
1. Set up AWS account and configure IAM roles
   - **Your IAM user/group (for Terraform deploys):** `AmazonBedrockFullAccess`, `AWSLambda_FullAccess`, `AmazonAPIGatewayAdministrator`, `AmazonDynamoDBFullAccess`, `IAMFullAccess`, `CloudWatchFullAccessV2`
   - **Lambda A execution role:** `lambda:InvokeFunction` (to call Lambda B), CloudWatch Logs
   - **Lambda B execution role:** `bedrock:InvokeModel` (scoped to Claude 3.5 Haiku ARN), DynamoDB CRUD (scoped to `albert-*` tables), CloudWatch Logs
2. Create DynamoDB tables with Terraform
3. Set up dev/staging environments
4. Prototype Bedrock API calls locally

**Phase 1: Slash Commands (local, Gateway-based)** — DONE (2026-02-16)
5. ~~Register `/summary-24hr`, `/summary-perUser`, `/summary-article` slash commands and `Summarize Article` message context menu via Discord API~~ ✅
6. ~~Implement interaction handlers in Serenity (works over Gateway WebSocket for local testing)~~ ✅
7. ~~Add ephemeral responses for chat summaries~~ ✅
8. ~~Test locally — validate slash commands and context menu work end-to-end~~ ✅
9. Remove reaction-based triggers once slash commands are confirmed working

> **Implementation Notes (Phase 1):** Added `ResponseTarget` enum (`response_target.rs`) to decouple delivery mechanism from trigger type — reactions and interactions share the same worker queue and bot_functions. Decoupled `get_messages()` and `get_channel_name()` from `Reaction`/`Message` types (now take `ChannelId`). Commands registered via `Command::set_global_commands()` in `ready()`. Context menu dedup re-fetches message via API (Discord resolved data omits reactions). Error recovery: interaction variants get error message edited into deferred response instead of silent timeout. Bot OAuth2 URL needs `applications.commands` scope. Both trigger paths (reactions + slash commands) coexist.

**Phase 2: Bedrock Integration (local)** — DONE (2026-02-16)
10. ~~Add AWS SDK dependencies to Rust project~~ ✅
11. ~~Implement BedrockClient with Claude 3.5 Haiku~~ ✅
12. ~~Replace Python subprocess calls with direct Bedrock API calls~~ ✅
13. ~~Test locally — validate output quality and latency vs Mistral-7B~~ ✅
14. ~~Remove `python_llm/`, `python_runner.rs`, `read_and_write.rs`, file-based IPC~~ ✅

> **Implementation Notes (Phase 2):** Added `bedrock_client.rs` wrapping `aws_sdk_bedrockruntime::Client` with Converse API. `BedrockClient` initialized as `Arc` in `main.rs`, passed to worker. Prompts ported from Python `[INST]` format to Converse system/user messages. Claude 3.5 Haiku produces clean plain text output — no JSON wrapping or 3-level fallback parser needed (eliminated `output_structures.py`). Model ID requires inference profile prefix: `us.anthropic.claude-3-5-haiku-20241022-v1:0`. Removed `uuid`, `serde`, `serde_json` deps from rust_bot. Deleted entire `python_llm/` directory, `python_runner.rs`, `read_and_write.rs`, and all `jobs/{uuid}/` temp dir logic.

**Phase 3: Lambda Conversion** — DONE (2026-02-16)
15. ~~Create Lambda A (Gateway) — signature verification, deferred ACK, async invoke~~ ✅
16. ~~Create Lambda B (Worker) — Bedrock calls, Discord followup webhook~~ ✅
17. ~~Implement ed25519 signature verification~~ ✅
18. Local testing with `cargo lambda` / SAM CLI

> **Implementation Notes (Phase 3):** Converted to Cargo workspace with three members. `rust_bot/` kept as fallback (still compiles and can run as a Gateway bot if Lambda has issues). Lambda A (`lambda_gateway`) uses `lambda_http`, `ed25519-dalek`, and `aws-sdk-lambda` for async worker invocation. Lambda B (`lambda_worker`) uses `lambda_runtime` with a custom `DiscordClient` (reqwest-based REST API wrapper) replacing Serenity's `ctx.http`. `BedrockClient` and `article_handler` copied verbatim from `rust_bot/`. DynamoDB deferred to Phase 5 — article dedup still works via bot reaction check over REST for the context menu path. `SKIP_SIGNATURE_VERIFY` env var for local testing.
>
> **Post-Phase 3 workspace structure:**
> ```
> Albert/
> ├── Cargo.toml                       # Workspace root (3 members)
> ├── rust_bot/                        # KEPT as Gateway fallback during migration
> │   └── src/
> │       ├── main.rs                  # Serenity Gateway entry point
> │       ├── handle_events.rs         # EventHandler (reactions + slash commands)
> │       ├── worker_and_job.rs        # MPSC worker queue
> │       ├── bot_functions.rs         # Orchestrators (uses Serenity ctx.http)
> │       ├── bedrock_client.rs        # BedrockClient (shared with lambda_worker)
> │       ├── article_handler.rs       # URL extraction + article fetch + dedup
> │       ├── response_target.rs       # ResponseTarget enum for delivery routing
> │       └── message_utils.rs         # Discord helpers (fetch messages, channel names)
> ├── lambda_gateway/                  # Lambda A — fast ACK + signature verify
> │   └── src/
> │       └── main.rs                  # Ed25519 verify, ping, defer, async invoke
> └── lambda_worker/                   # Lambda B — actual summarization work
>     └── src/
>         ├── main.rs                  # Entry point, command dispatch, error handling
>         ├── bedrock_client.rs        # BedrockClient (copy from rust_bot)
>         ├── article_handler.rs       # extract_url + fetch_article_text (from rust_bot)
>         ├── discord_client.rs        # reqwest-based Discord REST API wrapper
>         └── handlers.rs             # handle_summary_chat, handle_summary_article_*
> ```

**Phase 4: Infrastructure & Deploy**
19. Deploy API Gateway (HTTP) + both Lambdas with Terraform
20. Configure Discord Interactions Endpoint URL to API Gateway
21. Set up CloudWatch logging, monitoring, and cost alerts
22. Deploy to staging, test for 48 hours
23. Cut over to production

**Phase 5: Optimization**
24. Optimize Lambda cold start times (binary size, provisioned concurrency if needed)
25. Fine-tune Bedrock prompts for Claude 3.5 Haiku
26. Implement summary caching in DynamoDB to reduce duplicate Bedrock calls
27. Set up billing alerts and per-user rate limiting
28. Tighten IAM user/group policies — replace broad managed policies (e.g. `FullAccess`) with scoped custom policies limited to only the resources this project uses. The Lambda execution roles should already be minimal from Phase 0.

**Phase 6: Unit Tests**
28. Unit tests for BedrockClient — mock API responses, error handling, token limits
29. Unit tests for webhook signature verification (ed25519)
30. Unit tests for DynamoDB state manager — CRUD operations, TTL behavior
31. Unit tests for Lambda A routing — interaction type parsing, deferred response format
32. Unit tests for Lambda B — slash command dispatch, context menu dispatch, followup webhook
33. Integration test for full pipeline (mock API Gateway + Bedrock + DynamoDB + Discord webhook)

### Cost Estimation (Monthly) — Lambda + API Gateway + Bedrock

**Assumptions per request:** ~2,000 input tokens, ~500 output tokens, ~10s Lambda execution at 256MB.

| Component | Rate | Small (10 servers, 300 req/mo) | Medium (100 servers, 3K req/mo) | Large (500 servers, 15K req/mo) |
|---|---|---|---|---|
| **Lambda compute** | $0.0000166667/GB-s | $0.01 | $0.13 | $0.63 |
| **Lambda requests** | $0.20/1M | $0.00 | $0.00 | $0.00 |
| **API Gateway HTTP** | $1.00/1M | $0.00 | $0.00 | $0.02 |
| **Bedrock input** (Claude 3.5 Haiku) | $0.001/1K tokens | $0.60 | $6.00 | $30.00 |
| **Bedrock output** (Claude 3.5 Haiku) | $0.005/1K tokens | $0.75 | $7.50 | $37.50 |
| **DynamoDB** | Pay-per-request | ~$0.50 | ~$1.00 | ~$3.00 |
| **Total** | | **~$1.86** | **~$14.63** | **~$71.15** |

Key insight: Bedrock token costs are 88%+ of the total bill at scale. The infrastructure (Lambda + APIGW) is nearly free. The main lever for profitability is managing token usage — input length limits, summary caching in DynamoDB, and potentially Bedrock batch pricing for non-urgent requests.

**For comparison — ECS Fargate would add a flat ~$9/month** for a 24/7 container (0.25 vCPU, 0.5GB RAM) regardless of usage. Meaningful at small scale, negligible at large scale.

### Monitoring & Observability

**CloudWatch Dashboards:**
```
Dashboard: Albert Bot Metrics
├─ Lambda Invocations (per event type)
├─ Lambda Duration (p50, p95, p99)
├─ Lambda Errors (by error type)
├─ Bedrock API Calls
├─ Bedrock Token Usage
├─ DynamoDB Read/Write Units
└─ API Gateway 4xx/5xx Errors
```

**Alarms:**
- Lambda error rate > 5%
- Lambda duration > 25 seconds (timeout warning)
- Bedrock throttling errors
- DynamoDB throttling
- Monthly cost exceeds $50

**Logging Strategy:** Use `tracing` crate with structured logging. Instrument handlers with `#[tracing::instrument]`, log guild_id on success and error details on failure. CloudWatch receives logs automatically from Lambda. See `implement_strats.md` for code example.

### Rollback Plan

**If migration fails:**
1. Revert DNS/webhook endpoint to old bot
2. Keep old bot running until migration is stable
3. DynamoDB data can be exported and imported to local DB if needed

**Rollback strategy:** Keep the current Serenity Gateway bot running in parallel during migration. If Lambda has issues, revert the Discord Interactions Endpoint URL back to empty (re-enables Gateway event handling). No code-level feature flags needed — the switch is at the Discord configuration level.

---

## Risk Assessment & Mitigation

### Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Bedrock prompt tuning needed for new model | Low | Medium | Claude 3.5 Haiku is significantly more capable than Mistral-7B — main risk is prompt format differences, not quality |
| Lambda cold starts cause timeouts | Medium | High | Use provisioned concurrency for critical functions, optimize binary size |
| DynamoDB costs exceed estimates | Low | Low | Set up billing alerts, use on-demand pricing initially |
| Article fetching blocked by sites | Medium | High | Implement retry logic, add user-agent rotation, graceful degradation |
| DM conversation context grows too large | Medium | Medium | Implement sliding window, compression, hard limits |

### Operational Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Discord API changes break webhook | High | Low | Monitor Discord changelog, implement version detection |
| AWS region outage | High | Very Low | Multi-region deployment (future), graceful error messages |
| Cost overruns from abuse | Medium | Medium | Rate limiting per user, max tokens per request |
| Lost state during migration | Medium | Low | Run parallel systems, export/import state carefully |

---

## Success Metrics

### Feature 1 (Article Summarization)
- [ ] 90%+ of article URLs successfully fetched and parsed
- [ ] <5 seconds from command to summary delivered
- [ ] Cache hit rate >60% for repeated URLs (via DynamoDB)
- [ ] User satisfaction (measured by continued usage)

### Feature 2 (Interactive Q&A)
- [ ] Average of 2-3 follow-up questions per conversation
- [ ] 80%+ questions answered accurately
- [ ] <3 seconds response time for Q&A
- [ ] Session expiration rate <10% due to errors

### Feature 3 (AWS Migration)
- [ ] Zero downtime during migration
- [ ] <500ms Lambda cold start (p95)
- [ ] <2s total response time (p95)
- [ ] 95%+ cost reduction vs. local deployment
- [ ] <1% error rate post-migration

---

## Timeline Summary

```
COMPLETED:  Feature 1 — Article Summarization (2026-02-13)
COMPLETED:  Article fetch fix + dedup marker change (2026-02-16)
COMPLETED:  Feature 3 Phase 1 — Slash commands + context menu (2026-02-16)
COMPLETED:  Feature 3 Phase 2 — Bedrock integration (2026-02-16)
COMPLETED:  Feature 3 Phase 3 — Lambda conversion / Cargo workspace (2026-02-16)
IN PROGRESS: Feature 3 — AWS Migration (bedrock-migration branch)
  Phase 1:  Slash commands (local, Gateway-based) — DONE
  Phase 2:  Bedrock integration (replace Python/Mistral-7B) — DONE
  Phase 3:  Lambda conversion (two-Lambda pattern, rust_bot kept as fallback) — DONE
  Phase 4:  Infrastructure deploy (Terraform)
  Phase 5:  Optimization
NOT STARTED: Feature 2 — Interactive Q&A in DMs (depends on Feature 3 for DynamoDB + Bedrock)
```

**Sequencing:** Feature 3 must come before Feature 2, since Q&A sessions depend on DynamoDB (from Feature 3) and Bedrock (also Feature 3). Slash command migration (Feature 3 Phase 1) can begin immediately.

---

## Appendix: Key Code Locations

### Files Created (Phase 2-3)
```
lambda_gateway/src/
└── main.rs                      # Lambda A — signature verify, ACK, async invoke

lambda_worker/src/
├── main.rs                      # Lambda B — entry point, command dispatch
├── bedrock_client.rs            # AWS Bedrock integration (copied from rust_bot)
├── article_handler.rs           # URL extraction + article fetch (copied from rust_bot)
├── discord_client.rs            # reqwest-based Discord REST API wrapper
└── handlers.rs                  # Command handlers (chat summary, article summary)

rust_bot/src/
└── bedrock_client.rs            # BedrockClient (Phase 2, also copied to lambda_worker)
```

### Files Still To Create
```
lambda_worker/src/
├── state_manager.rs             # DynamoDB wrapper (Phase 5)
└── dm_session_manager.rs        # DM conversation state (Feature 2)

infrastructure/
└── main.tf                      # Terraform — API Gateway, Lambdas, DynamoDB, IAM (Phase 4)
```

### Files Already Deleted (Phase 2)
```
python_llm/                      # Entire directory (replaced by Bedrock)
rust_bot/src/python_runner.rs   # Subprocess execution (replaced by Bedrock)
rust_bot/src/read_and_write.rs  # File-based IPC (no longer needed)
```

### Files Kept as Fallback (rust_bot/)
```
rust_bot/                        # Full Serenity Gateway bot — kept as rollback option
├── src/handle_events.rs         # Serenity reaction + slash command handlers
├── src/worker_and_job.rs        # MPSC worker queue
├── src/bot_functions.rs         # Orchestrators (Serenity ctx.http)
├── src/response_target.rs       # ResponseTarget enum
└── src/message_utils.rs         # Discord helpers
```

---

## Questions & Decisions

### Resolved
1. ~~**Article summarization trigger:**~~ → Migrating from 📖 reaction to `Summarize Article` context menu + `/summary-article` slash command
2. ~~**Bedrock model:**~~ → Claude 3.5 Haiku (fast, cost-effective)
3. ~~**Hosting architecture:**~~ → Lambda + API Gateway (HTTP) over ECS Fargate
4. ~~**Interaction model:**~~ → Slash commands + context menu (Discord HTTP Interactions), replacing Gateway WebSocket + emoji reactions
5. ~~**Article dedup:**~~ → DynamoDB URL hash lookup (replaces bot reaction check)
6. ~~**Article summary visibility:**~~ → Context menu = visible reply to channel; `/summary-article` = ephemeral (user-only)

### Open
1. **DM session duration:** 24 hours or configurable per user?
2. **Multi-region:** Start with single region or multi-region from day 1?
3. **Rate limiting:** Per-user limits on API calls?
4. **Cost alerts:** What monthly budget threshold should trigger alerts?

---

## Next Steps

1. ~~**Implement slash commands locally** (Feature 3, Phase 1)~~ — DONE (2026-02-16)
2. ~~**Integrate Bedrock** (Feature 3, Phase 2)~~ — DONE (2026-02-16)
3. ~~**Convert to Lambda** (Feature 3, Phase 3)~~ — DONE (2026-02-16)
4. **Deploy infrastructure** (Feature 3, Phase 4) — Terraform for API Gateway, Lambdas, IAM roles
5. **Optimization** (Feature 3, Phase 5) — cold start tuning, DynamoDB caching, rate limiting
6. **Begin Feature 2** (Interactive Q&A) — depends on DynamoDB + Bedrock from Feature 3
