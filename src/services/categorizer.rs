///! Wall Post Auto-Categorization (#12)
///! AI-powered категоризація постів на стіні плачу

use anyhow::Result;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
    CreateChatCompletionRequestArgs,
};
use async_openai::{Client, config::OpenAIConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "post_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PostCategory {
    Complaint,
    Suggestion,
    Celebration,
    Question,
    SupportNeeded,
}

impl PostCategory {
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Complaint => "😤",
            Self::Suggestion => "💡",
            Self::Celebration => "🎉",
            Self::Question => "❓",
            Self::SupportNeeded => "💙",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Complaint => "Complaint",
            Self::Suggestion => "Suggestion",
            Self::Celebration => "Celebration",
            Self::Question => "Question",
            Self::SupportNeeded => "Support Needed",
        }
    }

    pub fn label_uk(&self) -> &'static str {
        match self {
            Self::Complaint => "Скарга",
            Self::Suggestion => "Пропозиція",
            Self::Celebration => "Успіх",
            Self::Question => "Питання",
            Self::SupportNeeded => "Потрібна підтримка",
        }
    }
}

pub struct WallPostCategorizer {
    client: Client<OpenAIConfig>,
}

impl WallPostCategorizer {
    pub fn new(api_key: String) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self { client }
    }

    /// Категоризувати пост на стіні
    pub async fn categorize(&self, content: &str) -> Result<PostCategory> {
        let system_prompt = "Ти - класифікатор постів на стіні плачу (Wall of Complaints) для платформи ментального здоров'я.\n\n\
            Категорії:\n\
            - COMPLAINT: скарги на роботу, умови, процеси, невдоволення, проблеми в команді\n\
            - SUGGESTION: ідеї покращень, пропозиції змін, конструктивна критика з рішеннями\n\
            - CELEBRATION: успіхи, досягнення, позитивний фідбек, подяки, радісні події\n\
            - QUESTION: питання про процеси, прохання порад, запити інформації\n\
            - SUPPORT_NEEDED: прохання про допомогу, емоційна підтримка, burnout, стрес, ментальне здоров'я\n\n\
            Інструкції:\n\
            - Аналізуй ЗМІСТ та TONE повідомлення\n\
            - Якщо згадується burnout, stress, anxiety, depression → SUPPORT_NEEDED\n\
            - Якщо є конкретна пропозиція рішення → SUGGESTION (навіть якщо є скарга)\n\
            - Якщо про успіх, досягнення, подяку → CELEBRATION\n\
            - Якщо питання без скарги → QUESTION\n\
            - Інакше → COMPLAINT\n\n\
            Відповідай ТІЛЬКИ однією категорією: COMPLAINT, SUGGESTION, CELEBRATION, QUESTION, або SUPPORT_NEEDED\n\
            Без пояснень, без коментарів - тільки категорія.";

        let messages = vec![
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: system_prompt.to_string(),
                name: None,
            }),
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(content.to_string()),
                name: None,
            }),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo") // Faster & cheaper for classification
            .messages(messages)
            .temperature(0.3) // Lower temperature for consistent classification
            .max_tokens(10)
            .build()?;

        let response = self.client.chat().create(request).await?;

        let category_str = response.choices[0]
            .message
            .content
            .clone()
            .unwrap_or_default()
            .trim()
            .to_uppercase();

        // Parse category
        let category = match category_str.as_str() {
            "COMPLAINT" => PostCategory::Complaint,
            "SUGGESTION" => PostCategory::Suggestion,
            "CELEBRATION" => PostCategory::Celebration,
            "QUESTION" => PostCategory::Question,
            "SUPPORT_NEEDED" => PostCategory::SupportNeeded,
            _ => {
                // Fallback: якщо AI повернув щось незрозуміле, використати keyword-based classification
                tracing::warn!(
                    "Unexpected AI category '{}', using keyword fallback",
                    category_str
                );
                self.keyword_based_fallback(content)
            }
        };

        Ok(category)
    }

    /// Fallback keyword-based classification якщо AI failed
    fn keyword_based_fallback(&self, content: &str) -> PostCategory {
        let content_lower = content.to_lowercase();

        // Support needed keywords
        if content_lower.contains("burnout")
            || content_lower.contains("депресія")
            || content_lower.contains("тривога")
            || content_lower.contains("anxiety")
            || content_lower.contains("допоможіть")
            || content_lower.contains("важко")
            || content_lower.contains("не можу")
        {
            return PostCategory::SupportNeeded;
        }

        // Celebration keywords
        if content_lower.contains("дякую")
            || content_lower.contains("вдалося")
            || content_lower.contains("успіх")
            || content_lower.contains("радий")
            || content_lower.contains("молодці")
        {
            return PostCategory::Celebration;
        }

        // Suggestion keywords
        if content_lower.contains("пропоную")
            || content_lower.contains("можна б")
            || content_lower.contains("краще б")
            || content_lower.contains("ідея")
            || content_lower.contains("варто")
        {
            return PostCategory::Suggestion;
        }

        // Question keywords
        if content_lower.contains("як ")
            || content_lower.contains("чому")
            || content_lower.contains("коли")
            || content_lower.contains("?")
        {
            return PostCategory::Question;
        }

        // Default to complaint
        PostCategory::Complaint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_emoji() {
        assert_eq!(PostCategory::Complaint.emoji(), "😤");
        assert_eq!(PostCategory::Celebration.emoji(), "🎉");
        assert_eq!(PostCategory::SupportNeeded.emoji(), "💙");
    }

    #[test]
    fn test_keyword_fallback() {
        let categorizer = WallPostCategorizer::new("test".to_string());

        let burnout_text = "У мене burnout, дуже важко, не можу більше";
        assert_eq!(
            categorizer.keyword_based_fallback(burnout_text),
            PostCategory::SupportNeeded
        );

        let celebration_text = "Дякую всім! Вдалося запустити проект!";
        assert_eq!(
            categorizer.keyword_based_fallback(celebration_text),
            PostCategory::Celebration
        );

        let suggestion_text = "Пропоную зробити longer breaks";
        assert_eq!(
            categorizer.keyword_based_fallback(suggestion_text),
            PostCategory::Suggestion
        );

        let question_text = "Як працює система відпусток?";
        assert_eq!(
            categorizer.keyword_based_fallback(question_text),
            PostCategory::Question
        );
    }

    #[test]
    fn test_category_labels() {
        assert_eq!(PostCategory::Complaint.label(), "Complaint");
        assert_eq!(PostCategory::Suggestion.label_uk(), "Пропозиція");
    }
}
