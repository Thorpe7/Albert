# Albert Bot — Roadmap

## Overview

Albert is a Discord bot that summarizes chat history and articles using AWS Bedrock (Claude 3.5 Haiku). It runs as a serverless two-Lambda architecture deployed via Terraform, triggered by Discord slash commands and context menus.

---

## Architecture

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
└────────┬─────────────────────────────┘
         │
         v
┌──────────────────────────────────────┐
│   Lambda A — "Gateway" (Rust)        │
│   - Verify Discord signature         │
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
         v                             v
┌─────────────────┐         ┌──────────────────────┐
│  DynamoDB       │         │  Bedrock Runtime API │
│  - Summaries    │         │  - Claude 3.5 Haiku  │
│  - DM Sessions  │         │  - Pay per token     │
│  - Cache        │         └──────────────────────┘
└─────────────────┘
```

Two Lambdas because Discord requires an interaction response within 3 seconds, but Bedrock inference takes longer. Lambda A defers immediately; Lambda B does the work and posts results via webhook.

---

## Feature 1: Article Summarization — DONE (2026-02-13)

- Users trigger article summarization via context menu or `/summary-article` slash command
- Fetches article HTML, extracts content with `readability`, summarizes via Bedrock
- Dedup: bot reacts with ✅ on original message (migrating to DynamoDB URL hash lookup)

---

## Feature 2: AWS Migration — IN PROGRESS (`bedrock-migration`)

| Phase | Description | Status |
|-------|-------------|--------|
| **1. Slash Commands** | Register `/summary-24hr`, `/summary-peruser`, `/summary-article`, context menu. Interaction handlers in Serenity for local testing. | DONE |
| **2. Bedrock Integration** | Replace Python/Mistral-7B with `BedrockClient` using Claude 3.5 Haiku. Remove `python_llm/`, file-based IPC. | DONE |
| **3. Lambda Conversion** | Two-Lambda pattern (Gateway + Worker). Discord signature verification. Cargo workspace with `rust_bot/` kept as fallback. | DONE |
| **4. Infrastructure Deploy** | Terraform for API Gateway, Lambdas, DynamoDB tables, CloudWatch. Discord endpoint configured. | DONE |
| **5. Dashboard & Monitoring** | Structured logging (Bedrock tokens/latency/cost). CloudWatch dashboard and alarms. `albert-usage` DynamoDB table. Admin-only `/stats` command. | NOT STARTED |
| **6. Optimization** | Cold start tuning, Bedrock prompt refinement, summary caching in DynamoDB, rate limiting. | NOT STARTED |
| **7. Unit Tests** | BedrockClient, webhook verification, DynamoDB state manager, Lambda routing, integration tests. | NOT STARTED |

### DynamoDB Tables

- `albert-articles` — Article summary cache + dedup (URL hash key)
- `albert-summaries` — Chat summary cache (message ID key, 7-day TTL)
- `albert-dm-sessions` — Q&A conversation state for Feature 3 (user ID + session ID, 24h TTL)
- `albert-usage` — Per-guild monthly usage tracking (Phase 5)

---

## Feature 3: Interactive Q&A in DMs — NOT STARTED

Depends on Feature 2 (DynamoDB + Bedrock).

- After receiving a summary, users can DM the bot with follow-up questions
- Sessions stored in DynamoDB with 24h TTL
- Context window management: sliding window + token budget
- Session expires automatically; user runs a new summary command to start fresh

---

## Open Questions

1. **DM session duration:** Fixed 24 hours, or configurable per user?
2. **Multi-region:** Start with single region or multi-region from day 1?
3. **Rate limiting:** Per-user limits on API calls?
4. **Cost alerts:** What monthly budget threshold should trigger alerts?

---

## Next Steps

1. **Dashboard & Monitoring** (Feature 2, Phase 5) — structured logging, CloudWatch dashboard/alarms, `/stats` command
2. **Optimization** (Feature 2, Phase 6) — cold start tuning, DynamoDB caching, rate limiting
3. **Unit Tests** (Feature 2, Phase 7) — BedrockClient, webhook verification, state manager, Lambda routing
4. **Begin Feature 3** (Interactive Q&A) — depends on Feature 2 completion
