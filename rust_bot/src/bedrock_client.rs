use anyhow::{anyhow, Result};
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, Message, SystemContentBlock, InferenceConfiguration,
};

const MODEL_ID: &str = "us.anthropic.claude-3-5-haiku-20241022-v1:0";

const STANDARD_SUMMARY_SYSTEM: &str = "\
You are a summarization assistant for Discord chat messages. \
Summarize the main points discussed in a detailed and descriptive manner. \
If a particularly good point is made by a user, include what that user said. \
Include relevant user concerns, specific examples mentioned, and highlight overall sentiment or themes. \
Aim for a comprehensive and thoughtful summary with depth.\n\n\
If a message only contains a link, image, or GIF, summarize it as \"[User shared a link]\" or skip it if irrelevant. \
Do NOT try to describe or interpret links.\n\n\
Provide ONLY the summary text. No JSON wrapping, no markdown fences.";

const PER_USER_SUMMARY_SYSTEM: &str = "\
You are a summarization assistant for Discord chat messages. \
Summarize the main points discussed and always specify who said what or who you are summarizing. \
Summaries should be detailed and descriptive and highlight overall sentiment or themes.\n\n\
If a message only contains a link, image, or GIF, summarize it as \"[User shared a link]\" or skip it if irrelevant. \
Do NOT try to describe or interpret links.\n\n\
Provide ONLY the summary text. No JSON wrapping, no markdown fences.";

const ARTICLE_SUMMARY_SYSTEM: &str = "\
You are an article summarization assistant. \
Summarize the following article as bullet points.\n\n\
Format:\n\
Start with a main takeaway line prefixed with a pin emoji.\n\
Then list key points as bullet items (- prefix), each one to two sentences.\n\
Be concise but informative.\n\n\
Provide ONLY the summary text. No JSON wrapping, no markdown fences.";

pub struct BedrockClient {
    client: aws_sdk_bedrockruntime::Client,
    model_id: String,
}

impl BedrockClient {
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = aws_sdk_bedrockruntime::Client::new(&config);
        Ok(Self {
            client,
            model_id: MODEL_ID.to_string(),
        })
    }

    pub async fn summarize_chat(&self, messages: &str, task_prompt: &str) -> Result<String> {
        let system_prompt = match task_prompt {
            "STANDARD_SUMMARY" => STANDARD_SUMMARY_SYSTEM,
            "PER_USER_SUMMARY" => PER_USER_SUMMARY_SYSTEM,
            _ => return Err(anyhow!("Unknown task prompt: {}", task_prompt)),
        };

        self.invoke(system_prompt, messages).await
    }

    pub async fn summarize_article(&self, article_text: &str) -> Result<String> {
        self.invoke(ARTICLE_SUMMARY_SYSTEM, article_text).await
    }

    async fn invoke(&self, system_prompt: &str, user_content: &str) -> Result<String> {
        let message = Message::builder()
            .role(ConversationRole::User)
            .content(ContentBlock::Text(user_content.to_string()))
            .build()
            .map_err(|e| anyhow!("Failed to build message: {}", e))?;

        let response = self.client.converse()
            .model_id(&self.model_id)
            .system(SystemContentBlock::Text(system_prompt.to_string()))
            .messages(message)
            .inference_config(
                InferenceConfiguration::builder()
                    .max_tokens(2048)
                    .temperature(0.5)
                    .build(),
            )
            .send()
            .await
            .map_err(|e| anyhow!("Bedrock API call failed: {}", e))?;

        let output = response.output()
            .ok_or_else(|| anyhow!("No output in Bedrock response"))?;

        let message = output.as_message()
            .map_err(|_| anyhow!("Output is not a message"))?;

        let text = message.content()
            .first()
            .ok_or_else(|| anyhow!("No content blocks in response"))?
            .as_text()
            .map_err(|_| anyhow!("Content block is not text"))?;

        Ok(text.to_string())
    }
}
