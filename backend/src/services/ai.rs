use crate::analytics::correlations::CorrelationInsight;
use crate::bot::daily_checkin::Metrics;
use crate::crypto::Crypto;
use anyhow::{anyhow, Result};
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
    ChatCompletionResponseFormat, ChatCompletionResponseFormatType,
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

    pub async fn analyze_transcript(
        &self,
        transcript: &str,
        context: &str,
        metrics: Option<&Metrics>,
    ) -> Result<AiOutcome> {
        let mut retries = 0;
        let mut system_prompt = String::from(
            "You are a mental health triage assistant for OpsLab Mindguard.\n\
Input: transcription text + brief context (last 3 days).\n\
Return JSON ONLY.\n\
Required JSON fields:\n\
- sentiment: positive|neutral|negative\n\
- emotion_tags: array of 2-5 descriptive emotions\n\
- risk_score: 1-10 (10 = imminent self-harm, 1 = stable)\n\
- topics: array of main themes in the text\n\
- advice: concise, actionable advice for today\n\
If self-harm cues appear, force risk_score=10.\n",
        );

        if let Some(m) = metrics {
            system_prompt.push_str(&format!(
                "User metrics context:\n\
WHO-5: {:.1}/100, PHQ-9: {:.1}/27, GAD-7: {:.1}/21, Burnout: {:.1}%, Sleep: {:.1}h, Stress: {:.1}/40\n",
                m.who5_score,
                m.phq9_score,
                m.gad7_score,
                m.burnout_percentage(),
                m.sleep_duration,
                m.stress_level
            ));
        }

        let contains_urgent = contains_urgent_keywords(transcript);

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
                .response_format(ChatCompletionResponseFormat {
                    r#type: ChatCompletionResponseFormatType::JsonObject,
                })
                .temperature(0.2)
                .max_tokens(300u16)
                .build()?;

            match self.client.chat().create(request).await {
                Ok(resp) => {
                    let content = resp
                        .choices
                        .first()
                        .and_then(|c| c.message.content.clone())
                        .unwrap_or_default();
                    let json = parse_ai_json(&content);
                    let mut risk_score =
                        json.get("risk_score").and_then(|v| v.as_i64()).unwrap_or(1) as i16;
                    if risk_score < 1 {
                        risk_score = 1;
                    } else if risk_score > 10 {
                        risk_score = 10;
                    }
                    let urgent = contains_urgent || risk_score >= 9;
                    if urgent {
                        risk_score = 10;
                    }
                    let mut normalized = normalize_ai_json(json, "Зроби коротку паузу, подихай 4-7-8 і обери одну просту дію на зараз.");
                    if let Some(obj) = normalized.as_object_mut() {
                        obj.insert("risk_score".to_string(), serde_json::json!(risk_score));
                    }
                    return Ok(AiOutcome {
                        transcript: transcript.to_string(),
                        ai_json: normalized,
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
                content: "You are a group chat assistant. Provide a short empathetic tip in Ukrainian. Never mention or request personal metrics or private data. If asked for personal stats, advise to DM the bot.".to_string(),
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

    pub async fn generate_personal_insight(
        &self,
        metrics: &Metrics,
        correlations: &[CorrelationInsight],
    ) -> Result<String> {
        let correlations_text = if correlations.is_empty() {
            "No strong correlations found.".to_string()
        } else {
            correlations
                .iter()
                .take(3)
                .map(|c| format!("{} (r={:.2}): {}", c.correlation_type, c.strength, c.description))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let system_prompt = "You are an empathetic mental health coach for OpsLab Mindguard.\n\
Write a concise personalized insight in Ukrainian.\n\
Constraints:\n\
- 2-4 short sentences.\n\
- Include 1-2 actionable tips (not medical advice).\n\
- Avoid diagnosis, keep supportive tone.\n";

        let user_prompt = format!(
            "User metrics (last 7-10 days):\n\
WHO-5: {:.1}/100\nPHQ-9: {:.1}/27\nGAD-7: {:.1}/21\nBurnout: {:.1}%\nSleep: {:.1}h\nStress: {:.1}/40\n\
Correlations: {}\n\
Compose the insight now.",
            metrics.who5_score,
            metrics.phq9_score,
            metrics.gad7_score,
            metrics.burnout_percentage(),
            metrics.sleep_duration,
            metrics.stress_level,
            correlations_text
        );

        let messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                role: Role::System,
                content: system_prompt.to_string(),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                role: Role::User,
                content: ChatCompletionRequestUserMessageContent::Text(user_prompt),
                name: None,
            }),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(messages)
            .temperature(0.4)
            .max_tokens(200u16)
            .build()?;

        let resp = self.client.chat().create(request).await?;
        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| {
                "Зроби коротку паузу, випий води та обери одну маленьку перемогу на сьогодні."
                    .to_string()
            });

        Ok(content)
    }
}

fn contains_urgent_keywords(text: &str) -> bool {
    let lowered = text.to_lowercase();
    let keywords = [
        "suicide",
        "kill myself",
        "self-harm",
        "hopeless",
        "суїцид",
        "суицид",
        "самогубство",
        "самоубийство",
        "покінчити з життям",
        "покончить с собой",
        "не хочу жити",
        "не хочу жить",
        "немає сенсу",
        "нет смысла",
        "завершити життя",
        "убити себе",
    ];

    keywords.iter().any(|k| lowered.contains(k))
}

fn parse_ai_json(content: &str) -> serde_json::Value {
    if let Ok(json) = serde_json::from_str(content) {
        return json;
    }

    let start = content.find('{');
    let end = content.rfind('}');
    if let (Some(start), Some(end)) = (start, end) {
        let slice = &content[start..=end];
        if let Ok(json) = serde_json::from_str(slice) {
            return json;
        }
    }

    serde_json::json!({})
}

fn normalize_ai_json(mut value: serde_json::Value, fallback_advice: &str) -> serde_json::Value {
    if !value.is_object() {
        return serde_json::json!({
            "sentiment": "unknown",
            "emotion_tags": [],
            "risk_score": 1,
            "topics": [],
            "advice": fallback_advice
        });
    }

    let obj = value.as_object_mut().expect("checked is_object");
    obj.entry("sentiment".to_string())
        .or_insert_with(|| serde_json::json!("unknown"));

    if !obj
        .get("emotion_tags")
        .map(|v| v.is_array())
        .unwrap_or(false)
    {
        obj.insert("emotion_tags".to_string(), serde_json::json!([]));
    }

    if !obj.get("topics").map(|v| v.is_array()).unwrap_or(false) {
        obj.insert("topics".to_string(), serde_json::json!([]));
    }

    let has_advice = obj
        .get("advice")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_advice {
        obj.insert("advice".to_string(), serde_json::json!(fallback_advice));
    }

    if !obj.contains_key("risk_score") {
        obj.insert("risk_score".to_string(), serde_json::json!(1));
    }

    value
}
