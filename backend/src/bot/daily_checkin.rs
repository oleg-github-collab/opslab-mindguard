///! Система щоденних чекінів з варіативними формулюваннями
///! - Короткі опитування (2-4 питання, до 3 хвилин)
///! - Різні варіанти питань для підтримки інтересу
///! - Повна картина за 7-10 днів
use chrono::{Datelike, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionType {
    Mood,
    Energy,
    Stress,
    Sleep,
    Workload,
    Motivation,
    Focus,
    Wellbeing,
    Reflection,
    Support,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: i32,
    pub qtype: String,
    pub text: String,
    pub emoji: String,
    pub scale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckIn {
    pub id: String,
    pub user_id: Uuid,
    pub date: chrono::DateTime<Utc>,
    pub day_of_week: u32,
    pub questions: Vec<Question>,
    pub intro_message: String,
    pub estimated_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckInAnswer {
    pub question_id: i32,
    pub qtype: String,
    pub value: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub who5_score: f64,
    pub phq9_score: f64,
    pub gad7_score: f64,
    #[serde(alias = "burnout_percentage")]
    pub mbi_score: f64,
    #[serde(default)]
    pub sleep_duration: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sleep_quality: Option<f64>,
    pub work_life_balance: f64,
    pub stress_level: f64,
}

impl Metrics {
    /// Alias for backward compatibility
    pub fn burnout_percentage(&self) -> f64 {
        self.mbi_score
    }

    pub fn sleep_quality(&self) -> f64 {
        self.sleep_quality.unwrap_or(self.sleep_duration)
    }
}

/// Банк варіативних питань (укр)
pub struct QuestionBank;

impl QuestionBank {
    /// Питання про настрій (WHO-5 базовані)
    pub fn mood_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Як твій настрій сьогодні?", "😊"),
            ("Як ти себе почуваєш цього ранку?", "🌅"),
            ("Оціни свій емоційний стан зараз", "💭"),
            ("Наскільки позитивно ти відчуваєш себе сьогодні?", "✨"),
        ]
    }

    /// Питання про енергію
    pub fn energy_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Який у тебе рівень енергії?", "⚡"),
            ("Наскільки ти відчуваєш себе бадьорим?", "🔋"),
            ("Як твоя витривалість сьогодні?", "💪"),
            ("Чи є у тебе сили на продуктивний день?", "🚀"),
        ]
    }

    /// Питання про стрес
    pub fn stress_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Наскільки ти відчуваєш стрес?", "😰"),
            ("Чи відчуваєш тиск або напругу?", "⚠️"),
            ("Наскільки спокійно ти себе почуваєш?", "🧘"),
            ("Чи турбують тебе якісь переживання?", "💭"),
        ]
    }

    /// Питання про сон
    pub fn sleep_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Як ти спав минулої ночі?", "😴"),
            ("Наскільки якісним був твій сон?", "🌙"),
            ("Чи відчуваєш себе відпочившим?", "🛌"),
            ("Скільки годин ти спав?", "⏰"),
        ]
    }

    /// Питання про робоче навантаження
    pub fn workload_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Наскільки високе твоє робоче навантаження?", "📊"),
            ("Чи справляєшся з кількістю задач?", "✅"),
            ("Як відчуваєш баланс роботи та відпочинку?", "⚖️"),
            ("Чи вистачає часу на все важливе?", "⏱️"),
        ]
    }

    /// Питання про мотивацію
    pub fn motivation_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Наскільки ти вмотивований сьогодні?", "🎯"),
            ("Чи є у тебе натхнення до роботи?", "💡"),
            ("Як твоя продуктивність сьогодні?", "📈"),
            ("Чи відчуваєш драйв до досягнень?", "🚀"),
        ]
    }

    /// Питання про фокус
    pub fn focus_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Наскільки легко тобі зосередитися?", "🎯"),
            ("Як твоя здатність до концентрації?", "🧠"),
            ("Чи вдається уникати відволікань?", "🔕"),
        ]
    }

    /// Питання про загальне благополуччя
    pub fn wellbeing_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Як оцінюєш своє загальне самопочуття?", "🌟"),
            ("Наскільки ти задоволений життям зараз?", "😊"),
            ("Чи відчуваєш себе комфортно?", "✨"),
        ]
    }

    /// Глибокі рефлексивні питання
    pub fn reflection_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Що сьогодні найбільше забрало енергію?", "🧭"),
            ("Що було найскладнішим моментом дня?", "🧩"),
            ("Яка одна річ зараз найбільше турбує?", "🫧"),
        ]
    }

    /// Підтримуючі питання
    pub fn support_questions() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Наскільки ти відчуваєш підтримку навколо?", "🤝"),
            ("Чи є щось, що могло б полегшити твій день?", "💬"),
            ("Наскільки ти відчуваєш безпеку говорити про труднощі?", "🛟"),
        ]
    }

    /// Отримати випадкове питання за типом
    pub fn get_random_question(qtype: QuestionType) -> (&'static str, &'static str) {
        let mut rng = rand::thread_rng();
        let questions = match qtype {
            QuestionType::Mood => Self::mood_questions(),
            QuestionType::Energy => Self::energy_questions(),
            QuestionType::Stress => Self::stress_questions(),
            QuestionType::Sleep => Self::sleep_questions(),
            QuestionType::Workload => Self::workload_questions(),
            QuestionType::Motivation => Self::motivation_questions(),
            QuestionType::Focus => Self::focus_questions(),
            QuestionType::Wellbeing => Self::wellbeing_questions(),
            QuestionType::Reflection => Self::reflection_questions(),
            QuestionType::Support => Self::support_questions(),
        };
        let idx = rng.gen_range(0..questions.len());
        questions[idx]
    }
}

/// Adaptive Question Engine (#1 WOW Feature)
/// Аналізує попередні відповіді (останні 3 дні) і пріоритизує питання
pub struct AdaptiveQuestionEngine;

impl AdaptiveQuestionEngine {
    /// Аналізує патерни і визначає пріоритет питань
    pub async fn analyze_priority(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<Vec<QuestionType>, anyhow::Error> {
        use crate::db;

        // Отримати патерни з БД
        let patterns = db::get_user_recent_pattern(pool, user_id).await?;

        let mut priorities = Vec::new();
        let mut scores: Vec<(QuestionType, f64, f64)> = Vec::new(); // (type, avg_value, priority_score)

        // Аналізувати кожен тип питання
        for (qtype, avg_value) in patterns {
            let question_type = match qtype.as_str() {
                "stress" => QuestionType::Stress,
                "sleep" => QuestionType::Sleep,
                "energy" => QuestionType::Energy,
                "mood" => QuestionType::Mood,
                "workload" => QuestionType::Workload,
                "focus" => QuestionType::Focus,
                "motivation" => QuestionType::Motivation,
                "wellbeing" => QuestionType::Wellbeing,
                _ => continue,
            };

            // Логіка пріоритизації:
            // - Високий стрес (>= 7) → топ пріоритет
            // - Поганий сон (<= 5) → топ пріоритет
            // - Низька енергія (<= 4) → високий пріоритет
            // - Низький mood (<= 4) → високий пріоритет
            let priority_score = match question_type {
                QuestionType::Stress if avg_value >= 7.0 => 100.0,
                QuestionType::Sleep if avg_value <= 5.0 => 95.0,
                QuestionType::Energy if avg_value <= 4.0 => 90.0,
                QuestionType::Mood if avg_value <= 4.0 => 85.0,
                QuestionType::Workload if avg_value >= 8.0 => 80.0,
                QuestionType::Focus if avg_value <= 4.0 => 75.0,
                _ => 50.0, // Нормальний пріоритет
            };

            scores.push((question_type, avg_value, priority_score));
        }

        // Сортувати за пріоритетом
        scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        // Вибрати топ 3 з найвищим пріоритетом
        for (qtype, _, score) in scores.iter().take(3) {
            if *score > 70.0 {
                // Тільки якщо справді високий пріоритет
                priorities.push(*qtype);
            }
        }

        Ok(priorities)
    }

    pub async fn needs_support(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<bool, anyhow::Error> {
        use crate::db;
        let patterns = db::get_user_recent_pattern(pool, user_id).await?;
        let mut stress = None;
        let mut mood = None;
        let mut energy = None;
        let mut workload = None;

        for (qtype, avg_value) in patterns {
            match qtype.as_str() {
                "stress" => stress = Some(avg_value),
                "mood" => mood = Some(avg_value),
                "energy" => energy = Some(avg_value),
                "workload" => workload = Some(avg_value),
                _ => {}
            }
        }

        let high_stress = stress.map(|v| v >= 7.0).unwrap_or(false);
        let low_mood = mood.map(|v| v <= 4.0).unwrap_or(false);
        let low_energy = energy.map(|v| v <= 4.0).unwrap_or(false);
        let high_workload = workload.map(|v| v >= 8.0).unwrap_or(false);

        Ok(high_stress || low_mood || low_energy || high_workload)
    }

    /// Генерує adaptive intro message на основі пріоритетів
    pub fn get_adaptive_intro(types: &[QuestionType]) -> String {
        if let Some(first) = types.first() {
            match first {
                QuestionType::Stress => {
                    "Доброго дня! 🌅 Помітив що stress високий. Як сьогодні?".to_string()
                }
                QuestionType::Sleep => {
                    "Привіт! 😴 Як спалося? Сон дуже важливий для здоров'я.".to_string()
                }
                QuestionType::Energy => "Вітаю! ⚡ Як рівень енергії? Подбай про себе.".to_string(),
                QuestionType::Mood => {
                    "Доброго ранку! 💙 Як настрій? Ти не один, ми поруч.".to_string()
                }
                QuestionType::Reflection => {
                    "Бачу напруження останнім часом. Давай коротко звіримось.".to_string()
                }
                QuestionType::Support => {
                    "Доброго дня! 🤝 Хочу зрозуміти як ти, щоб краще підтримати.".to_string()
                }
                _ => "Доброго ранку! Як справи сьогодні?".to_string(),
            }
        } else {
            "Доброго ранку! Як справи сьогодні?".to_string()
        }
    }
}

/// Генератор чекінів
pub struct CheckInGenerator;

impl CheckInGenerator {
    /// Генерує adaptive чекін (використовує аналіз попередніх відповідей)
    pub async fn generate_adaptive_checkin(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<CheckIn, anyhow::Error> {
        let day_of_week = Utc::now().weekday().num_days_from_monday();

        // Спробувати отримати adaptive пріоритети
        let mut question_types = AdaptiveQuestionEngine::analyze_priority(pool, user_id)
            .await
            .unwrap_or_default();

        // Якщо немає adaptive пріоритетів, використати стандартну логіку
        if question_types.is_empty() {
            question_types = Self::select_question_types(day_of_week);
        } else {
            // Доповнити до 3 питань, якщо потрібно
            while question_types.len() < 3 {
                let day_types = Self::select_question_types(day_of_week);
                for dt in day_types {
                    if !question_types.contains(&dt) {
                        question_types.push(dt);
                        if question_types.len() >= 3 {
                            break;
                        }
                    }
                }
                if question_types.len() >= 3 {
                    break;
                }
            }
        }

        // Якщо був складний період, додати глибоке + підтримуюче питання
        let needs_support = AdaptiveQuestionEngine::needs_support(pool, user_id)
            .await
            .unwrap_or(false);
        if needs_support {
            let mut prioritized = vec![QuestionType::Reflection, QuestionType::Support];
            for qt in question_types {
                if !prioritized.contains(&qt) {
                    prioritized.push(qt);
                }
            }
            question_types = prioritized;
        }

        let mut questions = Vec::new();
        for (idx, qtype) in question_types.iter().enumerate().take(3) {
            let (text, emoji) = QuestionBank::get_random_question(*qtype);
            questions.push(Question {
                id: idx as i32 + 1,
                qtype: Self::qtype_to_string(*qtype),
                text: text.to_string(),
                emoji: emoji.to_string(),
                scale: "1-10".to_string(),
            });
        }

        let intro_message = if question_types.len() > 0
            && AdaptiveQuestionEngine::analyze_priority(pool, user_id)
                .await
                .ok()
                .map(|p| !p.is_empty())
                .unwrap_or(false)
        {
            AdaptiveQuestionEngine::get_adaptive_intro(&question_types)
        } else {
            Self::get_intro_message(day_of_week)
        };

        Ok(CheckIn {
            id: format!("checkin_{}", Utc::now().format("%Y%m%d")),
            user_id,
            date: Utc::now(),
            day_of_week,
            questions,
            intro_message,
            estimated_time: "2-3 хвилини".to_string(),
        })
    }

    /// Генерує чекін залежно від дня тижня (legacy, використовується як fallback)
    pub fn generate_checkin(user_id: Uuid, day_of_week: u32) -> CheckIn {
        let question_types = Self::select_question_types(day_of_week);
        let mut questions = Vec::new();

        for (idx, qtype) in question_types.iter().enumerate() {
            let (text, emoji) = QuestionBank::get_random_question(*qtype);
            questions.push(Question {
                id: idx as i32 + 1,
                qtype: Self::qtype_to_string(*qtype),
                text: text.to_string(),
                emoji: emoji.to_string(),
                scale: "1-10".to_string(),
            });
        }

        CheckIn {
            id: format!("checkin_{}", Utc::now().format("%Y%m%d")),
            user_id,
            date: Utc::now(),
            day_of_week,
            questions,
            intro_message: Self::get_intro_message(day_of_week),
            estimated_time: "2-3 хвилини".to_string(),
        }
    }

    /// Вибір типів питань залежно від дня тижня
    pub fn select_question_types(day_of_week: u32) -> Vec<QuestionType> {
        match day_of_week {
            0 => vec![
                QuestionType::Mood,
                QuestionType::Energy,
                QuestionType::Motivation,
            ], // Понеділок
            1 | 2 | 3 => vec![
                QuestionType::Mood,
                QuestionType::Stress,
                QuestionType::Workload,
            ], // Вт-Чт
            4 => vec![
                QuestionType::Mood,
                QuestionType::Wellbeing,
                QuestionType::Energy,
            ], // П'ятниця
            _ => vec![
                QuestionType::Mood,
                QuestionType::Sleep,
                QuestionType::Wellbeing,
            ], // Вихідні
        }
    }

    /// Привітальне повідомлення
    fn get_intro_message(day_of_week: u32) -> String {
        match day_of_week {
            0 => "Доброго ранку! 🌅 Новий тиждень починається. Як твій настрій?",
            1 => "Привіт! ☀️ Вівторок - продуктивний день. Як справи?",
            2 => "Вітаю! 💪 Середина тижня. Як ти себе почуваєш?",
            3 => "Привіт! 🚀 Четвер - майже вихідні. Як настрій?",
            4 => "Доброго дня! 🎉 П'ятниця! Як відчуваєш себе?",
            5 => "Вітаю! 🌈 Субота - час відновлення. Як справи?",
            6 => "Привіт! ☕ Неділя - день відпочинку. Як настрій?",
            _ => "Привіт! Як ти себе почуваєш сьогодні?",
        }
        .to_string()
    }

    fn qtype_to_string(qtype: QuestionType) -> String {
        match qtype {
            QuestionType::Mood => "mood",
            QuestionType::Energy => "energy",
            QuestionType::Stress => "stress",
            QuestionType::Sleep => "sleep",
            QuestionType::Workload => "workload",
            QuestionType::Motivation => "motivation",
            QuestionType::Focus => "focus",
            QuestionType::Wellbeing => "wellbeing",
            QuestionType::Reflection => "reflection",
            QuestionType::Support => "support",
        }
        .to_string()
    }
}

/// Розрахунок метрик на основі відповідей
pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Розраховує метрики за 7-10 днів відповідей
    pub fn calculate_metrics(answers: &[CheckInAnswer]) -> Option<Metrics> {
        if answers.len() < 21 {
            // Мінімум 7 днів * 3 питання = 21 відповідь
            return None;
        }

        let mut mood_values = Vec::new();
        let mut energy_values = Vec::new();
        let mut stress_values = Vec::new();
        let mut sleep_values = Vec::new();
        let mut workload_values = Vec::new();
        let mut motivation_values = Vec::new();
        let mut focus_values = Vec::new();
        let mut wellbeing_values = Vec::new();

        for answer in answers {
            match answer.qtype.as_str() {
                "mood" => mood_values.push(answer.value as f64),
                "energy" => energy_values.push(answer.value as f64),
                "stress" => stress_values.push(answer.value as f64),
                "sleep" => sleep_values.push(answer.value as f64),
                "workload" => workload_values.push(answer.value as f64),
                "motivation" => motivation_values.push(answer.value as f64),
                "focus" => focus_values.push(answer.value as f64),
                "wellbeing" => wellbeing_values.push(answer.value as f64),
                _ => {}
            }
        }

        let avg = |vals: &[f64]| -> f64 {
            if vals.is_empty() {
                0.0
            } else {
                vals.iter().sum::<f64>() / vals.len() as f64
            }
        };

        // WHO-5 Well-Being Index (0-100)
        let who5_components: Vec<f64> = mood_values
            .iter()
            .chain(energy_values.iter())
            .chain(wellbeing_values.iter())
            .copied()
            .collect();
        let who5 = (avg(&who5_components) * 10.0).min(100.0).max(0.0) as i32;

        // PHQ-9 Depression (0-27) - інверсія позитивних показників
        let phq9_inv: Vec<f64> = mood_values
            .iter()
            .chain(energy_values.iter())
            .chain(motivation_values.iter())
            .map(|v| 10.0 - v)
            .collect();
        let phq9 = (avg(&phq9_inv) * 2.7).min(27.0).max(0.0) as i32;

        // GAD-7 Anxiety (0-21)
        let gad7_components: Vec<f64> = stress_values
            .iter()
            .copied()
            .chain(focus_values.iter().map(|v| 10.0 - v))
            .collect();
        let gad7 = (avg(&gad7_components) * 2.1).min(21.0).max(0.0) as i32;

        // MBI Burnout (0-100%)
        let mbi_components: Vec<f64> = stress_values
            .iter()
            .chain(workload_values.iter())
            .copied()
            .chain(energy_values.iter().map(|v| 10.0 - v))
            .chain(motivation_values.iter().map(|v| 10.0 - v))
            .collect();
        let mbi = (avg(&mbi_components) * 10.0).min(100.0).max(0.0);

        // Sleep
        let sleep_duration = avg(&sleep_values);

        // Work-Life Balance
        let work_life_balance = 10.0 - avg(&workload_values);

        // Stress Level (PSS 0-40)
        let stress_level = avg(&stress_values) * 4.0;

        Some(Metrics {
            who5_score: who5 as f64,
            phq9_score: phq9 as f64,
            gad7_score: gad7 as f64,
            mbi_score: mbi,
            sleep_duration,
            sleep_quality: Some(sleep_duration),
            work_life_balance,
            stress_level,
        })
    }

    /// Перевірка чи показники критичні
    pub fn is_critical(metrics: &Metrics) -> bool {
        metrics.who5_score < 50.0
            || metrics.phq9_score >= 15.0
            || metrics.gad7_score >= 15.0
            || metrics.mbi_score >= 70.0
    }

    /// Визначення рівня ризику
    pub fn risk_level(metrics: &Metrics) -> &'static str {
        if Self::is_critical(metrics) {
            "critical"
        } else if metrics.who5_score < 60.0
            || metrics.phq9_score >= 10.0
            || metrics.gad7_score >= 10.0
            || metrics.mbi_score >= 50.0
        {
            "high"
        } else if metrics.who5_score < 70.0
            || metrics.phq9_score >= 5.0
            || metrics.gad7_score >= 5.0
            || metrics.mbi_score >= 35.0
        {
            "medium"
        } else {
            "low"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_checkin() {
        let user_id = Uuid::new_v4();
        let checkin = CheckInGenerator::generate_checkin(user_id, 0); // Понеділок

        assert_eq!(checkin.questions.len(), 3);
        assert_eq!(checkin.day_of_week, 0);
        assert!(
            checkin.intro_message.contains("Понеділок") || checkin.intro_message.contains("ранку")
        );
    }

    #[test]
    fn test_metrics_calculation() {
        let answers = vec![
            CheckInAnswer {
                question_id: 1,
                qtype: "mood".to_string(),
                value: 7,
            },
            CheckInAnswer {
                question_id: 2,
                qtype: "energy".to_string(),
                value: 8,
            },
            CheckInAnswer {
                question_id: 3,
                qtype: "stress".to_string(),
                value: 4,
            },
        ];

        // Недостатньо даних для розрахунку
        let result = MetricsCalculator::calculate_metrics(&answers);
        assert!(result.is_none());

        // Достатньо даних (7 днів * 3 = 21 відповідь)
        let mut full_answers = Vec::new();
        for _ in 0..7 {
            full_answers.extend_from_slice(&answers);
        }

        let metrics = MetricsCalculator::calculate_metrics(&full_answers);
        assert!(metrics.is_some());

        let m = metrics.unwrap();
        assert!(m.who5_score > 0 && m.who5_score <= 100);
        assert!(m.phq9_score >= 0 && m.phq9_score <= 27);
    }
}
