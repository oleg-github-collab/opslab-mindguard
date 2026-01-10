///! Voice AI Coach (#11)
///! Enhanced voice message analysis з персоналізованими рекомендаціями на основі user metrics

use crate::bot::daily_checkin::Metrics;
use anyhow::Result;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
    CreateChatCompletionRequestArgs,
};
use async_openai::{Client, config::OpenAIConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCoachResponse {
    pub analysis: String,
    pub recommendations: Vec<String>,
    pub empathy_score: f64, // 0.0 - 1.0
    pub sentiment: String,  // positive, neutral, negative
}

pub struct VoiceCoach {
    client: Client<OpenAIConfig>,
}

impl VoiceCoach {
    pub fn new(api_key: String) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self { client }
    }

    /// Аналізувати голосове повідомлення з контекстом user metrics
    pub async fn analyze_voice_message(
        &self,
        transcription: &str,
        user_metrics: Option<&Metrics>,
    ) -> Result<VoiceCoachResponse> {
        let system_prompt = self.build_system_prompt(user_metrics);

        let messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: system_prompt,
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(transcription.to_string()),
                name: None,
            }),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4-turbo-preview")
            .messages(messages)
            .temperature(0.7)
            .max_tokens(500)
            .build()?;

        let response = self.client.chat().create(request).await?;

        let content = response.choices[0]
            .message
            .content
            .clone()
            .unwrap_or_default();

        // Визначити sentiment
        let sentiment = self.detect_sentiment(&content);

        // Екстрактити рекомендації
        let recommendations = self.extract_recommendations(&content);

        // Розрахувати empathy score
        let empathy_score = self.calculate_empathy_score(&content);

        Ok(VoiceCoachResponse {
            analysis: content,
            recommendations,
            empathy_score,
            sentiment,
        })
    }

    fn build_system_prompt(&self, metrics: Option<&Metrics>) -> String {
        let mut prompt = String::from(
            "Ти - емпатичний AI-коуч для ментального здоров'я співробітників OpsLab.\n\n\
            Твоя роль:\n\
            1. Уважно вислухати (прочитати транскрипцію голосового)\n\
            2. Визначити емоційний стан та основні проблеми\n\
            3. Надати підтримку, розуміння та конкретні рекомендації\n\
            4. Бути стислим (3-5 речень основного аналізу + bullet points рекомендацій)\n\n",
        );

        if let Some(m) = metrics {
            prompt.push_str(&format!(
                "Контекст користувача (останні показники):\n\
                - WHO-5 (well-being): {:.1}/100 {}\n\
                - PHQ-9 (depression): {:.1}/27 {}\n\
                - GAD-7 (anxiety): {:.1}/21 {}\n\
                - Burnout Risk: {:.0}% {}\n\
                - Sleep Quality: {:.1}/10\n\
                - Stress Level: {:.1}/40\n\n",
                m.who5_score,
                self.interpret_who5(m.who5_score),
                m.phq9_score,
                self.interpret_phq9(m.phq9_score),
                m.gad7_score,
                self.interpret_gad7(m.gad7_score),
                m.burnout_percentage(),
                self.interpret_burnout(m.burnout_percentage()),
                m.sleep_quality(),
                m.stress_level
            ));

            // Додати специфічні нотатки на основі metrics
            if m.who5_score < 50.0 {
                prompt.push_str(
                    "⚠️ ВАЖЛИВО: Well-being дуже низький. Будь особливо уважним до ознак депресії.\n\n",
                );
            }

            if m.phq9_score >= 15.0 {
                prompt.push_str(
                    "🚨 КРИТИЧНО: Високі депресивні симптоми. Рекомендуй професійну допомогу!\n\n",
                );
            }

            if m.mbi_score > 70.0 {
                prompt.push_str(
                    "🔥 BURNOUT ALERT: Критичний рівень вигорання. Наполегливо рекомендуй відпочинок.\n\n",
                );
            }
        }

        prompt.push_str(
            "Інструкції:\n\
            - Відповідай УКРАЇНСЬКОЮ мовою\n\
            - Будь теплим, підтримуючим, але чесним\n\
            - Використовуй \"ти\" форму (не \"ви\")\n\
            - Якщо бачиш серйозні проблеми - чітко рекомендуй поговорити з Jane, керівником або психологом\n\
            - Формат відповіді:\n\
              [2-3 речення емпатичного розуміння]\n\
              \n\
              Рекомендації:\n\
              • [конкретна дія 1]\n\
              • [конкретна дія 2]\n\
              • [конкретна дія 3]\n\n\
            - НЕ використовуй банальні фрази типу \"все буде добре\"\n\
            - Давай КОНКРЕТНІ, ACTIONABLE поради",
        );

        prompt
    }

    fn interpret_who5(&self, score: f64) -> &'static str {
        if score >= 75.0 {
            "(відмінно)"
        } else if score >= 50.0 {
            "(норма)"
        } else if score >= 35.0 {
            "(знижений)"
        } else {
            "(критично низький)"
        }
    }

    fn interpret_phq9(&self, score: f64) -> &'static str {
        if score < 5.0 {
            "(мінімальні)"
        } else if score < 10.0 {
            "(легкі)"
        } else if score < 15.0 {
            "(помірні)"
        } else if score < 20.0 {
            "(значні)"
        } else {
            "(важкі)"
        }
    }

    fn interpret_gad7(&self, score: f64) -> &'static str {
        if score < 5.0 {
            "(мінімальна)"
        } else if score < 10.0 {
            "(легка)"
        } else if score < 15.0 {
            "(помірна)"
        } else {
            "(важка)"
        }
    }

    fn interpret_burnout(&self, score: f64) -> &'static str {
        if score < 30.0 {
            "(низький)"
        } else if score < 50.0 {
            "(помірний)"
        } else if score < 70.0 {
            "(високий)"
        } else {
            "(критичний)"
        }
    }

    fn detect_sentiment(&self, text: &str) -> String {
        let text_lower = text.to_lowercase();

        let positive_keywords = [
            "чудово",
            "супер",
            "добре",
            "краще",
            "радий",
            "щасливий",
            "вдячний",
        ];
        let negative_keywords = [
            "погано",
            "важко",
            "стрес",
            "втомився",
            "депресія",
            "тривога",
            "burnout",
        ];

        let positive_count = positive_keywords
            .iter()
            .filter(|&k| text_lower.contains(k))
            .count();
        let negative_count = negative_keywords
            .iter()
            .filter(|&k| text_lower.contains(k))
            .count();

        if positive_count > negative_count + 1 {
            "positive".to_string()
        } else if negative_count > positive_count + 1 {
            "negative".to_string()
        } else {
            "neutral".to_string()
        }
    }

    fn extract_recommendations(&self, analysis: &str) -> Vec<String> {
        analysis
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("•")
                    || trimmed.starts_with("-")
                    || trimmed.starts_with("*")
                    || trimmed.starts_with("1.")
                    || trimmed.starts_with("2.")
                    || trimmed.starts_with("3.")
            })
            .map(|s| {
                s.trim()
                    .trim_start_matches("•")
                    .trim_start_matches("-")
                    .trim_start_matches("*")
                    .trim_start_matches("1.")
                    .trim_start_matches("2.")
                    .trim_start_matches("3.")
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn calculate_empathy_score(&self, analysis: &str) -> f64 {
        let text_lower = analysis.to_lowercase();

        let empathy_words = [
            "розумію",
            "підтримую",
            "важливо",
            "нормально",
            "не один",
            "не сама",
            "не сам",
            "допоможу",
            "тримайся",
            "молодець",
            "відчуваю",
            "бачу",
            "чую тебе",
            "поруч",
        ];

        let count = empathy_words
            .iter()
            .filter(|&word| text_lower.contains(word))
            .count();

        // Normalize to 0.0-1.0
        (count as f64 / empathy_words.len() as f64).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentiment_detection() {
        let coach = VoiceCoach::new("test".to_string());

        let positive_text = "Чудово себе почуваю, добре все, радий";
        assert_eq!(coach.detect_sentiment(positive_text), "positive");

        let negative_text = "Погано, важко, стрес, втомився дуже";
        assert_eq!(coach.detect_sentiment(negative_text), "negative");
    }

    #[test]
    fn test_extract_recommendations() {
        let coach = VoiceCoach::new("test".to_string());

        let text = "Аналіз...\n\nРекомендації:\n• Медитація\n• Прогулянка\n• Поговори з кимось";
        let recs = coach.extract_recommendations(text);

        assert_eq!(recs.len(), 3);
        assert!(recs[0].contains("Медитація"));
    }

    #[test]
    fn test_empathy_score() {
        let coach = VoiceCoach::new("test".to_string());

        let empathic_text = "Розумію тебе, підтримую, ти не один, тримайся";
        let score = coach.calculate_empathy_score(empathic_text);

        assert!(score > 0.0);
    }
}
