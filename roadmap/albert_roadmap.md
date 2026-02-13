# Albert Bot - Implementation Roadmap

## Executive Summary
This roadmap outlines the implementation plan for three major enhancements to the Albert Discord bot:
1. Article summarization via emoji reactions
2. Interactive Q&A in DMs
3. Migration from local deployment to AWS Bedrock + Lambda

---

## ~~Feature 1: Article Summarization~~ (Implemented 2026-02-13)

### Overview
Enable users to summarize linked articles by adding an emoji reaction. ~~The summary will be posted as a thread on the original message~~, with deduplication to prevent re-summarizing the same link.

> **Implementation Notes:** Implemented with a simpler architecture than originally planned. Summary is posted as a reply (not a thread — too noisy). Deduplication uses the bot's own 📖 reaction on the original message (no Redis/DynamoDB needed). Article extraction uses the `readability` crate. Prompt added to existing `prompts.py` rather than a separate file.

### Architecture Changes Required

#### ~~1.1 State Management Layer~~ (Implemented 2026-02-13)
**Problem:** Need to track which URLs have been summarized to avoid duplication.

**Implemented Solution:** Bot reacts with 📖 on the original message after posting the summary. Dedup check uses `msg.reactions` with `me: true` — zero extra API calls, persists on Discord's side, survives bot restarts. No external cache needed.

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
- `fetch_article_text(url: &str) -> Result<String>` — uses `readability::extractor::scrape()`
- `bot_already_replied(msg: &Message) -> bool` — checks bot's own 📖 reaction

**Dependencies added:**
- `reqwest` (with `rustls-tls`) - HTTP client (used internally by readability)
- `readability` - HTML parsing and article extraction

**Not needed:** `url` crate (reqwest validates), `redis` (bot reaction dedup)

#### ~~1.3 Updated Event Handler~~ (Implemented 2026-02-13)
**Modified `rust_bot/src/handle_events.rs`:**

Added `📖` (`\u{1F4D6}`) branch in `reaction_add()`:
1. Fetch the original message
2. Check `bot_already_replied()` — if true, skip
3. Call `extract_url()` — if None, skip
4. Dispatch `Job::SummarizeArticle` to worker queue

#### ~~1.4 Summary Delivery~~ (Implemented 2026-02-13)
Originally planned as thread creation. Changed to **reply to original message** (less noisy) + bot 📖 reaction as dedup marker. Implemented in `summarize_article()` in `rust_bot/src/bot_functions.rs`.

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
10. ~~Implement deduplication checks~~ ✅ (bot 📖 reaction)
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

**Storage Model:**
```
DM_Conversation {
  user_id: u64,
  conversation_id: Uuid,
  original_summary: String,        // From server
  original_context: String,        // Minimal metadata (channel, date)
  qa_history: Vec<QAPair>,         // User questions + bot answers
  created_at: Timestamp,
  expires_at: Timestamp,           // E.g., 24 hours
}
```

#### 2.2 Conversation State Management

**Option A: Stateful Session Store (Redis)**
```
Key: dm_session:{user_id}:{conversation_id}
Value: {
  summary: "...",
  context: "...",
  messages: [
    {role: "user", content: "..."},
    {role: "assistant", content: "..."}
  ],
  ttl: 86400  // 24 hours
}
```

**Option B: Embed Context in Each Message (Stateless)**
- Reconstruct conversation from last N DM messages
- No external state needed
- Higher token usage, more processing

**Recommendation:** Option A for better UX and token efficiency.

#### 2.3 Conversation Lifecycle

**Trigger:** When bot sends initial DM with summary
```rust
// After sending summary DM
create_dm_session(
    user_id: reaction.user_id,
    summary: model_response,
    context: ChannelContext { channel_name, timestamp }
);
```

**Continuation:** When user replies to DM
```rust
async fn message(&self, ctx: Context, msg: Message) {
    if msg.guild_id.is_none() {  // Is DM
        if let Some(session) = get_dm_session(msg.author.id).await {
            // User is in active Q&A session
            handle_followup_question(ctx, msg, session).await;
        }
    }
}
```

**Expiration:** TTL-based cleanup
- Redis: Automatic TTL expiration
- DynamoDB: TTL attribute with DynamoDB Streams cleanup

#### 2.4 Context Window Management

**Strategy:**
1. **Sliding window:** Keep last N question-answer pairs
2. **Token budget:** Dynamically trim based on context length
3. **Summary compression:** Re-summarize Q&A history if it grows too large

**Implementation in `model_chain.py`:**
```python
def build_qa_prompt(summary: str, qa_history: List[dict], user_question: str) -> str:
    # Keep original summary always
    # Include last 3-5 Q&A pairs
    # Add current question
    
    context_tokens = estimate_tokens(summary + format_qa_history(qa_history))
    if context_tokens > MAX_CONTEXT_TOKENS:
        qa_history = compress_qa_history(qa_history)  # Keep only recent ones
    
    return construct_prompt(summary, qa_history, user_question)
```

### Implementation Steps

**Phase 1: DM Detection & Session Management (Week 1)**
1. ✅ Add DM message event handler
2. ✅ Implement session storage (Redis/DynamoDB)
3. ✅ Create session on initial summary send
4. ✅ Add session lookup on DM receive

**Phase 2: Q&A Pipeline (Week 2)**
5. ✅ Create Q&A prompt template in Python
6. ✅ Modify Python LLM to accept conversation history
7. ✅ Implement context window management
8. ✅ Add conversation history formatting

**Phase 3: UX & Edge Cases (Week 3)**
9. ✅ Add "end conversation" command or auto-expire notification
10. ✅ Handle multiple concurrent conversations per user
11. ✅ Add typing indicators while processing
12. ✅ Graceful handling of expired sessions

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
1. User reacts with 🤖 to channel messages
   → Bot DMs user with summary
   → Bot creates DM session (24h TTL)

2. User receives DM: "Here's your summary... [summary text]
   You can ask me follow-up questions about this for the next 24 hours!"

3. User replies in DM: "What was the consensus on topic X?"
   → Bot retrieves session
   → Bot generates answer with full context
   → Bot updates session history

4. User asks another question: "Any dissenting opinions?"
   → Bot has context from previous questions
   → Bot provides coherent, contextual answer

5. After 24 hours or user sends "!end"
   → Session expires
   → Bot notifies: "This conversation has ended. React to new messages to start fresh!"
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
```
┌─────────────────┐
│  Discord Server │
└────────┬────────┘
         │
         v
┌──────────────────────────────────────┐
│         API Gateway (REST)           │
│   POST /discord-webhook              │
└────────┬─────────────────────────────┘
         │
         v
┌──────────────────────────────────────┐
│   Lambda Function (Rust)             │
│   - discord-event-handler            │
│   - Parse Discord events             │
│   - Route to appropriate handler     │
└────────┬─────────────────────────────┘
         │
         ├─────────────────────────────┐
         │                             │
         v                             v
┌─────────────────┐         ┌──────────────────────┐
│  DynamoDB       │         │  Bedrock Runtime API │
│  - Summaries    │         │  - Claude/Titan      │
│  - DM Sessions  │         │  - Managed inference │
│  - Cache        │         └──────────────────────┘
└─────────────────┘
         │
         v
┌─────────────────┐
│  S3 (Optional)  │
│  - Long content │
│  - Logs         │
└─────────────────┘
```

### Migration Components

#### 3.1 Replace Local Mistral with AWS Bedrock

**Why Bedrock?**
- ✅ No infrastructure to manage (no GPU instances)
- ✅ Pay per token (cost-efficient for bursty workload)
- ✅ Instant scaling
- ✅ Access to multiple models (Claude, Titan, Llama)
- ✅ Built-in guardrails and content filtering

**Model Selection:**
- **Claude 3 Haiku:** Fast, cost-effective for summarization ($0.25/1M input tokens)
- **Claude 3 Sonnet:** Better quality if needed ($3/1M input tokens)
- **Titan Text Express:** Cheapest option ($0.13/1M tokens)

**Code Changes Required:**

**Remove:** `python_llm/` entire directory

**Add:** Rust AWS SDK integration
```rust
// New file: rust_bot/src/bedrock_client.rs
use aws_sdk_bedrockruntime::{Client, types::ContentBlock};

pub struct BedrockClient {
    client: Client,
    model_id: String,
}

impl BedrockClient {
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Self {
            client,
            model_id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
        }
    }

    pub async fn generate_summary(&self, content: String) -> Result<String, Error> {
        let prompt = format!(
            "Summarize the following Discord conversation:\n\n{}",
            content
        );

        let response = self.client
            .invoke_model()
            .model_id(&self.model_id)
            .body(/* Claude API format */)
            .send()
            .await?;

        // Parse response
        Ok(extract_text_from_response(response))
    }
}
```

**Remove Python subprocess calls:**
```rust
// DELETE: rust_bot/src/python_runner.rs
// DELETE: File I/O for message passing
// REPLACE WITH: Direct Bedrock API calls
```

#### 3.2 Rust Lambda Function

**Architecture Decision:**
- **Single Lambda** for all Discord events (simpler, less cold start)
- OR **Multiple Lambdas** per event type (better separation, more complex)

**Recommendation:** Start with single Lambda, split later if needed.

**Lambda Handler Structure:**
```rust
// rust_bot/src/lambda_handler.rs
use lambda_runtime::{service_fn, LambdaEvent, Error};
use serde_json::Value;

async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let discord_event = parse_discord_event(event.payload)?;
    
    match discord_event.event_type {
        "reaction_add" => handle_reaction(discord_event).await,
        "message_create" => handle_message(discord_event).await,
        _ => Ok(json!({"statusCode": 200}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(function_handler)).await
}
```

**Dependencies to add:**
```toml
[dependencies]
lambda_runtime = "0.8"
tokio = { version = "1", features = ["full"] }
aws-config = "1.0"
aws-sdk-bedrockruntime = "1.0"
aws-sdk-dynamodb = "1.0"
serde_json = "1.0"
```

#### 3.3 Discord Webhook Integration

**Current:** Bot maintains WebSocket connection to Discord

**New:** Discord sends events to API Gateway → Lambda

**Setup Steps:**
1. Register slash commands via Discord Developer Portal
2. Configure Interactions Endpoint URL: `https://your-api-gateway.amazonaws.com/discord-webhook`
3. Verify webhook signature in Lambda
4. Respond within 3 seconds (Discord requirement)

**Webhook Verification:**
```rust
fn verify_discord_signature(
    signature: &str,
    timestamp: &str,
    body: &str,
    public_key: &str,
) -> bool {
    // Use ed25519-dalek to verify Discord's signature
    // Prevents unauthorized webhook calls
}
```

**3-Second Response Challenge:**
- Acknowledge immediately: Return 200 OK
- Process asynchronously: Invoke another Lambda or use SQS
- Use Discord's follow-up endpoint for delayed responses

```rust
async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Immediate ACK
    let ack_response = json!({ "type": 5 }); // DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE
    
    // Spawn async task for processing
    tokio::spawn(async move {
        process_event_async(event).await;
        // Use Discord webhook to send follow-up
    });
    
    Ok(ack_response)
}
```

#### 3.4 State Management with DynamoDB

**Tables Needed:**

**1. Summaries Cache:**
```
Table: albert-summaries
Partition Key: message_id (String)
Attributes:
  - summary (String)
  - channel_id (String)
  - user_id (String)
  - created_at (Number)
  - ttl (Number)  // Auto-delete after 7 days
```

**2. Article Cache:**
```
Table: albert-articles
Partition Key: url_hash (String)
Attributes:
  - url (String)
  - summary (String)
  - message_id (String)
  - thread_id (String)
  - created_at (Number)
  - ttl (Number)
```

**3. DM Sessions:**
```
Table: albert-dm-sessions
Partition Key: user_id (String)
Sort Key: session_id (String)
Attributes:
  - summary (String)
  - context (Map)
  - qa_history (List)
  - created_at (Number)
  - ttl (Number)  // Auto-delete after 24 hours
```

**DynamoDB Client in Rust:**
```rust
use aws_sdk_dynamodb::Client;

pub struct StateManager {
    client: Client,
}

impl StateManager {
    pub async fn get_summary(&self, message_id: &str) -> Option<String> {
        let result = self.client
            .get_item()
            .table_name("albert-summaries")
            .key("message_id", AttributeValue::S(message_id.to_string()))
            .send()
            .await
            .ok()?;
        
        result.item?.get("summary")?.as_s().ok().cloned()
    }

    pub async fn store_summary(&self, message_id: &str, summary: &str) {
        let ttl = current_timestamp() + 604800; // 7 days
        
        self.client
            .put_item()
            .table_name("albert-summaries")
            .item("message_id", AttributeValue::S(message_id.to_string()))
            .item("summary", AttributeValue::S(summary.to_string()))
            .item("ttl", AttributeValue::N(ttl.to_string()))
            .send()
            .await
            .ok();
    }
}
```

#### 3.5 Deployment & CI/CD

**Infrastructure as Code (Terraform):**

```hcl
# infrastructure/main.tf

# Lambda Function
resource "aws_lambda_function" "albert_bot" {
  filename      = "target/lambda/bootstrap.zip"
  function_name = "albert-discord-bot"
  role          = aws_iam_role.lambda_exec.arn
  handler       = "bootstrap"
  runtime       = "provided.al2"
  timeout       = 30
  memory_size   = 512

  environment {
    variables = {
      DISCORD_PUBLIC_KEY = var.discord_public_key
      DISCORD_BOT_TOKEN  = var.discord_bot_token
    }
  }
}

# API Gateway
resource "aws_apigatewayv2_api" "discord_webhook" {
  name          = "albert-discord-api"
  protocol_type = "HTTP"
}

resource "aws_apigatewayv2_integration" "lambda" {
  api_id           = aws_apigatewayv2_api.discord_webhook.id
  integration_type = "AWS_PROXY"
  integration_uri  = aws_lambda_function.albert_bot.invoke_arn
}

# DynamoDB Tables
resource "aws_dynamodb_table" "summaries" {
  name         = "albert-summaries"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "message_id"

  attribute {
    name = "message_id"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }
}

# Similar for other tables...
```

**GitHub Actions CI/CD:**
```yaml
# .github/workflows/deploy.yml
name: Deploy Albert to AWS

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: x86_64-unknown-linux-musl
      
      - name: Build Lambda
        run: |
          cargo build --release --target x86_64-unknown-linux-musl
          cp target/x86_64-unknown-linux-musl/release/albert bootstrap
          zip lambda.zip bootstrap
      
      - name: Deploy with Terraform
        env:
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
        run: |
          cd infrastructure
          terraform init
          terraform apply -auto-approve
```

### Migration Steps (Phased Rollout)

**Phase 0: Preparation (Week 1)**
1. ✅ Set up AWS account and configure IAM roles
2. ✅ Create DynamoDB tables with Terraform
3. ✅ Set up dev/staging environments
4. ✅ Prototype Bedrock API calls

**Phase 1: Bedrock Integration (Week 2)**
5. ✅ Add AWS SDK dependencies to Rust project
6. ✅ Implement BedrockClient with Claude/Titan
7. ✅ Replace Python LLM calls with Bedrock calls
8. ✅ Test locally with both systems in parallel
9. ✅ Validate output quality and latency

**Phase 2: Lambda Conversion (Week 3)**
10. ✅ Convert Serenity event handlers to Lambda handlers
11. ✅ Implement webhook signature verification
12. ✅ Add DynamoDB state management
13. ✅ Handle 3-second timeout requirement
14. ✅ Local testing with Lambda runtime emulator

**Phase 3: Infrastructure Setup (Week 4)**
15. ✅ Deploy API Gateway and Lambda with Terraform
16. ✅ Configure Discord webhook endpoint
17. ✅ Set up CloudWatch logging and monitoring
18. ✅ Configure alerts for errors and latency

**Phase 4: Migration & Cutover (Week 5)**
19. ✅ Deploy to staging environment
20. ✅ Run parallel testing (old bot + new Lambda)
21. ✅ Monitor for 48 hours
22. ✅ Gradual traffic shift (10% → 50% → 100%)
23. ✅ Decommission local bot

**Phase 5: Optimization (Week 6)**
24. ✅ Implement Lambda provisioned concurrency if needed
25. ✅ Optimize cold start times
26. ✅ Fine-tune Bedrock prompts
27. ✅ Set up cost monitoring and budgets

**Phase 6: Unit Tests (Week 7)**
28. Unit tests for BedrockClient — mock API responses, error handling, token limits
29. Unit tests for webhook signature verification
30. Unit tests for DynamoDB state manager — CRUD operations, TTL behavior
31. Unit tests for Lambda handler routing — event parsing, response formatting
32. Unit tests for 3-second timeout handling and async deferral
33. Integration test for full Lambda pipeline (mock API Gateway + Bedrock + DynamoDB)

### Cost Estimation (Monthly)

**Current (Local Deployment):**
- Server: $50-100/month (assuming small VPS)
- GPU instance: $200-500/month (if using GPU)
- **Total:** ~$250-600/month + maintenance time

**New (AWS Serverless):**
- Lambda:
  - 100,000 invocations/month: ~$0.20
  - 30M compute-seconds: ~$5.00
- Bedrock (Claude Haiku):
  - 10M input tokens: ~$2.50
  - 2M output tokens: ~$3.00
- DynamoDB:
  - Pay-per-request: ~$1-5/month
- API Gateway:
  - 100K requests: ~$0.10
- **Total:** ~$12-16/month

**Savings:** ~95% cost reduction + no maintenance overhead

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

**Logging Strategy:**
```rust
use tracing::{info, error, warn};

#[tracing::instrument]
async fn handle_reaction(event: DiscordEvent) -> Result<(), Error> {
    info!("Processing reaction event");
    
    match process_reaction(&event).await {
        Ok(summary) => {
            info!(message_id = %event.message_id, "Summary generated");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Failed to generate summary");
            Err(e)
        }
    }
}
```

### Rollback Plan

**If migration fails:**
1. Revert DNS/webhook endpoint to old bot
2. Keep old bot running until migration is stable
3. DynamoDB data can be exported and imported to local DB if needed

**Feature flags:**
```rust
const USE_BEDROCK: bool = env::var("USE_BEDROCK")
    .unwrap_or_else(|_| "true".to_string()) == "true";

if USE_BEDROCK {
    bedrock_client.generate_summary(content).await
} else {
    python_runner::run_python(&filepath) // Fallback to old system
}
```

---

## Risk Assessment & Mitigation

### Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Bedrock quality worse than local model | High | Medium | A/B test responses, keep local model as fallback initially |
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
- [ ] <5 seconds from reaction to summary posted
- [ ] Cache hit rate >60% for repeated URLs
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
Week 1-2:  Article Summarization Foundation
Week 2-3:  Article Summarization Integration & Testing
Week 3-4:  Interactive Q&A Implementation
Week 4-5:  AWS Migration Preparation & Bedrock Integration
Week 5-6:  Lambda Conversion & Infrastructure Setup
Week 6-7:  Migration Execution & Monitoring
Week 7-8:  Optimization & Stabilization
```

**Total estimated time:** 8 weeks for all three initiatives

**Can be parallelized:**
- Features 1 & 2 can be developed concurrently
- Feature 3 migration can start while 1 & 2 are in testing
- Suggested: Build features 1 & 2 with AWS SDK from the start to ease migration

---

## Appendix: Key Code Locations

### New Files to Create
```
rust_bot/src/
├── article_handler.rs          # Article fetching and processing
├── bedrock_client.rs            # AWS Bedrock integration
├── dm_session_manager.rs        # DM conversation state
├── lambda_handler.rs            # Lambda entry point (for migration)
└── state_manager.rs             # DynamoDB wrapper

python_llm/src/
├── article_prompt.py            # Article-specific prompts
└── qa_prompt.py                 # Q&A conversation prompts
```

### Files to Modify
```
rust_bot/src/
├── bot.rs                       # Add article & DM event handlers
├── message_utils.rs             # Add thread creation, URL extraction
└── main.rs                      # AWS SDK initialization

python_llm/src/
└── model_chain.py               # Add article & Q&A methods
```

### Files to Eventually Delete (Post-Migration)
```
python_llm/                      # Entire directory
rust_bot/src/python_runner.rs   # Subprocess execution
rust_bot/src/read_and_write.rs  # File-based IPC (partially)
```

---

## Questions & Decisions Needed

1. ~~**Article summarization emoji:** Confirm 📖 is the chosen emoji?~~ → Yes, 📖 (`:open_book:`) confirmed ✅
2. **DM session duration:** 24 hours or configurable per user?
3. **Bedrock model preference:** Claude Haiku (fast/cheap) or Sonnet (better quality)?
4. **Multi-region:** Start with single region or multi-region from day 1?
5. **Rate limiting:** Per-user limits on API calls?
6. **Cost alerts:** What monthly budget threshold should trigger alerts?

---

## Next Steps

1. **Review & approve this roadmap**
2. **Prioritize features:** All three together or phased delivery?
3. **Set up AWS account and dev environment**
4. **Create detailed task breakdown in project management tool**
5. **Begin Phase 1 implementation**
