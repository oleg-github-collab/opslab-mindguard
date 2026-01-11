use crate::crypto::Crypto;
use anyhow::{anyhow, Result};
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
    CreateChatCompletionRequestArgs, Role,
};
use async_openai::{config::OpenAIConfig, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiOutcome {
    pub transcript: String,
    pub ai_json: serde_json::Value,
    pub risk_score: i16,
    pub urgent: bool,
}

#[derive(Clone)]
pub struct AiService {
    client: Client<OpenAIConfig>,
    crypto: Arc<Crypto>,
    api_key: String,
}

impl AiService {
    pub fn new(api_key: String, crypto: Arc<Crypto>) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key.clone());
        let client = Client::with_config(config);
        Self {
            client,
            crypto,
            api_key,
        }
    }

    pub async fn analyze_transcript(&self, transcript: &str, context: &str) -> Result<AiOutcome> {
        let mut retries = 0;
        let system_prompt = r#"You are a mental health triage assistant for OpsLab Mindguard.
Input: transcription text + brief context (last 3 days).
Output JSON fields:
- sentiment: positive|neutral|negative
- emotion_tags: array of 2-5 descriptive emotions
- risk_score: 1-10 (10 = imminent self-harm, 1 = stable)
- topics: array of main themes in the text
- advice: concise actionable advice for today
If self-harm cues such as "suicide", "kill myself", "hopeless" appear, force risk_score=10 and mark urgent."#;

        let lowered = transcript.to_lowercase();
        let contains_urgent = ["suicide", "kill myself", "hopeless"]
            .iter()
            .any(|k| lowered.contains(k));

        loop {
            let messages = vec![
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    role: Role::System,
                    content: system_prompt.to_string(),
                    name: None,
                }),
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    role: Role::User,
                    content: ChatCompletionRequestUserMessageContent::Text(format!(
                        "Context (last 3 days): {context}\nTranscription:\n{transcript}"
                    )),
                    name: None,
                }),
            ];

            let request = CreateChatCompletionRequestArgs::default()
                .model("gpt-4o")
                .messages(messages)
                .build()?;

            match self.client.chat().create(request).await {
                Ok(resp) => {
                    let content = resp
                        .choices
                        .first()
                        .and_then(|c| c.message.content.clone())
                        .unwrap_or_default();
                    let json: serde_json::Value = serde_json::from_str(&content)
                        .unwrap_or_else(|_| serde_json::json!({ "sentiment": "unknown" }));
                    let mut risk_score =
                        json.get("risk_score").and_then(|v| v.as_i64()).unwrap_or(1) as i16;
                    let urgent = contains_urgent || risk_score >= 9;
                    if urgent {
                        risk_score = risk_score.max(10);
                    }
                    return Ok(AiOutcome {
                        transcript: transcript.to_string(),
                        ai_json: json,
                        risk_score,
                        urgent,
                    });
                }
                Err(err) => {
                    retries += 1;
                    if retries > 3 {
                        return Err(anyhow!("OpenAI error: {err}"));
                    }
                    sleep(Duration::from_millis(500 * retries)).await;
                }
            }
        }
    }

    pub fn encrypt_payload(&self, value: &serde_json::Value) -> Result<String> {
        self.crypto
            .encrypt_str(&value.to_string())
            .map_err(|e| anyhow!("encryption error: {e}"))
    }

    pub async fn transcribe_voice(&self, audio_bytes: Vec<u8>) -> Result<String> {
        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_bytes)
                    .file_name("voice.ogg")
                    .mime_str("audio/ogg")?,
            );

        let resp = reqwest::Client::new()
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        let json: serde_json::Value = resp.json().await?;
        json.get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("no transcription text returned"))
    }

    pub async fn group_coach_response(&self, mention_text: &str) -> Result<String> {
        let messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                role: Role::System,
                content: "Return a short empathetic tip. Never mention personal metrics.".to_string(),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                role: Role::User,
                content: ChatCompletionRequestUserMessageContent::Text(format!(
                    "You are OpsLab Mindguard group assistant. Provide concise, non-clinical tips (breathing, productivity, focus) in Ukrainian.\nQuestion: {mention_text}"
                )),
                name: None,
            }),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(messages)
            .build()?;

        let resp = self.client.chat().create(request).await?;
        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "Тримайся! Зроби глибокий вдих і коротку прогулянку.".to_string());
        Ok(content)
    }
}
