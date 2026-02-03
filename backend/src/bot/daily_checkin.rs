///! Система щоденних чекінів з варіативними формулюваннями
///! - Короткі опитування (2-4 питання, до 3 хвилин)
///! - Різні варіанти питань для підтримки інтересу
///! - Повна картина за 7-10 днів
use chrono::{Datelike, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::domain::checkin::CheckinFrequency;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuestionWindow {
    Daily,
    Every3Days,
    Weekly,
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
    fn mood_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Як твій настрій сьогодні?", "😊"),
                ("Як ти себе почуваєш цього ранку?", "🌅"),
                ("Оціни свій емоційний стан зараз", "💭"),
                ("Наскільки позитивно ти відчуваєш себе сьогодні?", "✨"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Як змінювався твій настрій за останні 3 дні?", "😊"),
                ("Як ти почувався в середньому останні кілька днів?", "🌅"),
                ("Наскільки стабільним був твій настрій останніми днями?", "💭"),
                ("Яким був загальний емоційний фон за останні 3 дні?", "✨"),
            ],
            QuestionWindow::Weekly => vec![
                ("Як ти оцінюєш свій настрій цього тижня?", "😊"),
                ("Яким був емоційний фон за останній тиждень?", "🌅"),
                ("Наскільки позитивним був твій настрій цього тижня?", "💭"),
                ("Як загалом почувався протягом тижня?", "✨"),
            ],
        }
    }

    /// Питання про енергію
    fn energy_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Який у тебе рівень енергії?", "⚡"),
                ("Наскільки ти відчуваєш себе бадьорим?", "🔋"),
                ("Як твоя витривалість сьогодні?", "💪"),
                ("Чи є у тебе сили на продуктивний день?", "🚀"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Який рівень енергії був у середньому за останні 3 дні?", "⚡"),
                ("Наскільки бадьорим ти був останніми днями?", "🔋"),
                ("Як змінювався твій рівень сил останні кілька днів?", "💪"),
                ("Чи вистачало енергії на справи останні 3 дні?", "🚀"),
            ],
            QuestionWindow::Weekly => vec![
                ("Яким був твій рівень енергії цього тижня?", "⚡"),
                ("Наскільки стабільною була енергія протягом тижня?", "🔋"),
                ("Чи вистачало сил на завдання цього тижня?", "💪"),
                ("Як загалом з енергією за тиждень?", "🚀"),
            ],
        }
    }

    /// Питання про стрес
    fn stress_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Наскільки ти відчуваєш стрес?", "😰"),
                ("Чи відчуваєш тиск або напругу?", "⚠️"),
                ("Наскільки спокійно ти себе почуваєш?", "🧘"),
                ("Чи турбують тебе якісь переживання?", "💭"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Наскільки напруженим ти був останні 3 дні?", "😰"),
                ("Як багато стресу було останніми днями?", "⚠️"),
                ("Наскільки часто відчував тиск останні 3 дні?", "🧘"),
                ("Чи було відчуття перевантаження останніми днями?", "💭"),
            ],
            QuestionWindow::Weekly => vec![
                ("Яким був рівень стресу цього тижня?", "😰"),
                ("Наскільки напруженим був тиждень?", "⚠️"),
                ("Чи відчував тиск протягом тижня?", "🧘"),
                ("Як часто турбували переживання цього тижня?", "💭"),
            ],
        }
    }

    /// Питання про сон
    fn sleep_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Як ти спав минулої ночі?", "😴"),
                ("Наскільки якісним був твій сон?", "🌙"),
                ("Чи відчуваєш себе відпочившим?", "🛌"),
                ("Скільки годин ти спав?", "⏰"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Як ти спав останні кілька ночей?", "😴"),
                ("Наскільки якісним був сон останні 3 дні?", "🌙"),
                ("Чи відчував себе відпочившим у ці дні?", "🛌"),
                ("Скільки годин сну було в середньому останні 3 ночі?", "⏰"),
            ],
            QuestionWindow::Weekly => vec![
                ("Як ти спав цього тижня?", "😴"),
                ("Наскільки якісним був сон за тиждень?", "🌙"),
                ("Чи відчував себе відпочившим протягом тижня?", "🛌"),
                ("Скільки годин сну було в середньому за тиждень?", "⏰"),
            ],
        }
    }

    /// Питання про робоче навантаження
    fn workload_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Наскільки високе твоє робоче навантаження?", "📊"),
                ("Чи справляєшся з кількістю задач?", "✅"),
                ("Як відчуваєш баланс роботи та відпочинку?", "⚖️"),
                ("Чи вистачає часу на все важливе?", "⏱️"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Яким було робоче навантаження останні 3 дні?", "📊"),
                ("Чи справлявся з кількістю задач останніми днями?", "✅"),
                ("Як відчував баланс роботи та відпочинку останні 3 дні?", "⚖️"),
                ("Чи вистачало часу на важливе останніми днями?", "⏱️"),
            ],
            QuestionWindow::Weekly => vec![
                ("Яким було робоче навантаження цього тижня?", "📊"),
                ("Чи справлявся з кількістю задач цього тижня?", "✅"),
                ("Як був баланс роботи та відпочинку протягом тижня?", "⚖️"),
                ("Чи вистачало часу на важливе цього тижня?", "⏱️"),
            ],
        }
    }

    /// Питання про мотивацію
    fn motivation_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Наскільки ти вмотивований сьогодні?", "🎯"),
                ("Чи є у тебе натхнення до роботи?", "💡"),
                ("Як твоя продуктивність сьогодні?", "📈"),
                ("Чи відчуваєш драйв до досягнень?", "🚀"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Наскільки вмотивованим ти був останні 3 дні?", "🎯"),
                ("Чи було натхнення до роботи останніми днями?", "💡"),
                ("Як із продуктивністю останні 3 дні?", "📈"),
                ("Чи відчував драйв до досягнень останніми днями?", "🚀"),
            ],
            QuestionWindow::Weekly => vec![
                ("Наскільки вмотивованим ти був цього тижня?", "🎯"),
                ("Чи було натхнення до роботи протягом тижня?", "💡"),
                ("Як із продуктивністю цього тижня?", "📈"),
                ("Чи відчував драйв до досягнень протягом тижня?", "🚀"),
            ],
        }
    }

    /// Питання про фокус
    fn focus_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Наскільки легко тобі зосередитися?", "🎯"),
                ("Як твоя здатність до концентрації?", "🧠"),
                ("Чи вдається уникати відволікань?", "🔕"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Як було з концентрацією останні 3 дні?", "🎯"),
                ("Наскільки легко було зосередитися останніми днями?", "🧠"),
                ("Чи вдавалось уникати відволікань останні 3 дні?", "🔕"),
            ],
            QuestionWindow::Weekly => vec![
                ("Як було з концентрацією цього тижня?", "🎯"),
                ("Наскільки легко було зосередитися протягом тижня?", "🧠"),
                ("Чи вдавалось уникати відволікань цього тижня?", "🔕"),
            ],
        }
    }

    /// Питання про загальне благополуччя
    fn wellbeing_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Як оцінюєш своє загальне самопочуття?", "🌟"),
                ("Наскільки ти задоволений життям зараз?", "😊"),
                ("Чи відчуваєш себе комфортно?", "✨"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Як загалом почувався останні кілька днів?", "🌟"),
                ("Наскільки задоволений самопочуттям за останні 3 дні?", "😊"),
                ("Чи відчував комфорт останніми днями?", "✨"),
            ],
            QuestionWindow::Weekly => vec![
                ("Як загалом самопочуття цього тижня?", "🌟"),
                ("Наскільки задоволений самопочуттям за тиждень?", "😊"),
                ("Чи було відчуття комфорту цього тижня?", "✨"),
            ],
        }
    }

    /// Глибокі рефлексивні питання
    fn reflection_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Що сьогодні найбільше забрало енергію?", "🧭"),
                ("Що було найскладнішим моментом дня?", "🧩"),
                ("Яка одна річ зараз найбільше турбує?", "🫧"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Що останніми днями найбільше забирало енергію?", "🧭"),
                ("Які моменти були найскладнішими останні 3 дні?", "🧩"),
                ("Що найбільше турбувало останні кілька днів?", "🫧"),
            ],
            QuestionWindow::Weekly => vec![
                ("Що цього тижня найбільше забирало енергію?", "🧭"),
                ("Які моменти були найскладнішими цього тижня?", "🧩"),
                ("Що найбільше турбувало цього тижня?", "🫧"),
            ],
        }
    }

    /// Підтримуючі питання
    fn support_questions(window: QuestionWindow) -> Vec<(&'static str, &'static str)> {
        match window {
            QuestionWindow::Daily => vec![
                ("Що зараз найбільше допомагає відчувати підтримку?", "🤝"),
                ("Що могло б полегшити твій день?", "💬"),
                ("Що зробило б розмову про труднощі безпечнішою?", "🛟"),
            ],
            QuestionWindow::Every3Days => vec![
                ("Що останніми днями допомагало відчувати підтримку?", "🤝"),
                ("Що могло б полегшити ці останні дні?", "💬"),
                ("Що зробило б розмову про труднощі безпечнішою останніми днями?", "🛟"),
            ],
            QuestionWindow::Weekly => vec![
                ("Що цього тижня допомагало відчувати підтримку?", "🤝"),
                ("Що могло б полегшити твій тиждень?", "💬"),
                ("Що зробило б розмову про труднощі безпечнішою цього тижня?", "🛟"),
            ],
        }
    }

    /// Отримати випадкове питання за типом
    fn get_random_question(
        qtype: QuestionType,
        window: QuestionWindow,
    ) -> (&'static str, &'static str) {
        let mut rng = rand::thread_rng();
        let questions = match qtype {
            QuestionType::Mood => Self::mood_questions(window),
            QuestionType::Energy => Self::energy_questions(window),
            QuestionType::Stress => Self::stress_questions(window),
            QuestionType::Sleep => Self::sleep_questions(window),
            QuestionType::Workload => Self::workload_questions(window),
            QuestionType::Motivation => Self::motivation_questions(window),
            QuestionType::Focus => Self::focus_questions(window),
            QuestionType::Wellbeing => Self::wellbeing_questions(window),
            QuestionType::Reflection => Self::reflection_questions(window),
            QuestionType::Support => Self::support_questions(window),
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
            let (text, emoji) = QuestionBank::get_random_question(*qtype, QuestionWindow::Daily);
            questions.push(Question {
                id: idx as i32 + 1,
                qtype: Self::qtype_to_string(*qtype),
                text: text.to_string(),
                emoji: emoji.to_string(),
                scale: Self::scale_for_qtype(*qtype).to_string(),
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
            let (text, emoji) = QuestionBank::get_random_question(*qtype, QuestionWindow::Daily);
            questions.push(Question {
                id: idx as i32 + 1,
                qtype: Self::qtype_to_string(*qtype),
                text: text.to_string(),
                emoji: emoji.to_string(),
                scale: Self::scale_for_qtype(*qtype).to_string(),
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

    /// Генерує web-чекін залежно від вибраної частоти
    pub async fn generate_web_checkin(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        frequency: CheckinFrequency,
    ) -> Result<CheckIn, anyhow::Error> {
        match frequency {
            CheckinFrequency::Daily => Self::generate_adaptive_checkin(pool, user_id).await,
            CheckinFrequency::Every3Days => Self::generate_deep_checkin(pool, user_id).await,
            CheckinFrequency::Weekly => Self::generate_full_checkin(pool, user_id).await,
        }
    }

    async fn generate_deep_checkin(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<CheckIn, anyhow::Error> {
        let day_of_week = Utc::now().weekday().num_days_from_monday();
        let base_types = vec![
            QuestionType::Mood,
            QuestionType::Energy,
            QuestionType::Stress,
            QuestionType::Sleep,
            QuestionType::Workload,
            QuestionType::Motivation,
            QuestionType::Focus,
            QuestionType::Wellbeing,
            QuestionType::Reflection,
            QuestionType::Support,
        ];

        let mut prioritized = AdaptiveQuestionEngine::analyze_priority(pool, user_id)
            .await
            .unwrap_or_default();
        prioritized.retain(|t| base_types.contains(t));

        let mut question_types = Vec::new();
        for qtype in prioritized {
            if !question_types.contains(&qtype) {
                question_types.push(qtype);
            }
        }
        for qtype in base_types {
            if !question_types.contains(&qtype) {
                question_types.push(qtype);
            }
        }

        let questions = Self::build_questions(&question_types, QuestionWindow::Every3Days);
        Ok(CheckIn {
            id: format!("web_checkin_{}_every3", Utc::now().format("%Y%m%d")),
            user_id,
            date: Utc::now(),
            day_of_week,
            questions,
            intro_message: "Сьогодні глибший чекін (10 питань). Поділись, як ти почуваєшся останні дні."
                .to_string(),
            estimated_time: "6-8 хвилин".to_string(),
        })
    }

    async fn generate_full_checkin(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> Result<CheckIn, anyhow::Error> {
        let day_of_week = Utc::now().weekday().num_days_from_monday();
        let base_types = vec![
            QuestionType::Mood,
            QuestionType::Energy,
            QuestionType::Stress,
            QuestionType::Workload,
            QuestionType::Focus,
            QuestionType::Motivation,
            QuestionType::Sleep,
            QuestionType::Wellbeing,
            QuestionType::Reflection,
            QuestionType::Support,
        ];

        let mut question_types = base_types.clone();

        let mut extras = AdaptiveQuestionEngine::analyze_priority(pool, user_id)
            .await
            .unwrap_or_default();
        extras.retain(|t| base_types.contains(t));
        extras.dedup();

        for qtype in extras {
            if question_types.len() >= 12 {
                break;
            }
            question_types.push(qtype);
        }

        while question_types.len() < 12 {
            question_types.push(QuestionType::Mood);
            if question_types.len() < 12 {
                question_types.push(QuestionType::Stress);
            }
        }

        let questions = Self::build_questions(&question_types, QuestionWindow::Weekly);
        Ok(CheckIn {
            id: format!("web_checkin_{}_weekly", Utc::now().format("%Y%m%d")),
            user_id,
            date: Utc::now(),
            day_of_week,
            questions,
            intro_message: "Повний тижневий тест: більше деталей про стан, енергію та відновлення."
                .to_string(),
            estimated_time: "10-12 хвилин".to_string(),
        })
    }

    fn build_questions(question_types: &[QuestionType], window: QuestionWindow) -> Vec<Question> {
        question_types
            .iter()
            .enumerate()
            .map(|(idx, qtype)| {
                let (text, emoji) = QuestionBank::get_random_question(*qtype, window);
                Question {
                    id: idx as i32 + 1,
                    qtype: Self::qtype_to_string(*qtype),
                    text: text.to_string(),
                    emoji: emoji.to_string(),
                    scale: Self::scale_for_qtype(*qtype).to_string(),
                }
            })
            .collect()
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

    fn scale_for_qtype(qtype: QuestionType) -> &'static str {
        match qtype {
            QuestionType::Reflection | QuestionType::Support => "open",
            _ => "1-10",
        }
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
