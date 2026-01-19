use crate::crypto::Crypto;
use crate::domain::models::UserRole;
use crate::db::{MonthlyMetricInput, upsert_monthly_metric};
use anyhow::Result;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

struct SeedUser<'a> {
    code: &'a str,
    name: &'a str,
    email: &'a str,
    role: UserRole,
    note: &'a str,
}

pub async fn seed_all(pool: &PgPool, crypto: &Crypto) -> Result<()> {
    seed_questions(pool).await?;
    seed_users(pool, crypto).await?;
    seed_analytics(pool).await?;
    Ok(())
}

async fn seed_users(pool: &PgPool, crypto: &Crypto) -> Result<()> {
    let users = vec![
        SeedUser {
            code: "0000",
            name: "Олег Камінський",
            email: "work.olegkaminskyi@gmail.com",
            role: UserRole::Admin,
            note: "Адмін",
        },
        SeedUser {
            code: "7139",
            name: "Jane Давидюк",
            email: "janedavydiuk@opslab.uk",
            role: UserRole::Founder,
            note: "Фаундерка (Participant + Viewer)",
        },
        SeedUser {
            code: "4582",
            name: "Вероніка Кухарчук",
            email: "veronika.kukharchuk@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "9267",
            name: "Михайло Іващук",
            email: "mykhailo.ivashchuk@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "3814",
            name: "Ірина Мячкова",
            email: "iryna.miachkova@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "8463",
            name: "Оксана Клінчаян",
            email: "oksana.klinchaian@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "6738",
            name: "Іванна Сакало",
            email: "ivanna.sakalo@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "1425",
            name: "Марія Василик",
            email: "mariya.vasylyk@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
        SeedUser {
            code: "1122",
            name: "Катерина Петухова",
            email: "kateryna.petukhova@opslab.uk",
            role: UserRole::Employee,
            note: "",
        },
    ];

    let argon = Argon2::default();
    for user in users {
        let salt = SaltString::generate(rand_core::OsRng);
        let hash = argon
            .hash_password(user.code.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();

        let enc_name = crypto.encrypt_str(user.name)?;
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, hash, enc_name, role, note)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (email) DO NOTHING
            "#,
            Uuid::new_v4(),
            user.email,
            hash,
            enc_name,
            user.role as UserRole,
            user.note
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_questions(pool: &PgPool) -> Result<()> {
    let existing: i64 = sqlx::query_scalar!("SELECT COUNT(*) as \"count!\" FROM questions")
        .fetch_one(pool)
        .await?;
    if existing > 0 {
        return Ok(());
    }

    let prompts = vec![
        "Який сьогодні рівень енергії?",
        "Чи відчували ви тривогу сьогодні?",
        "Як оцінюєте якість сну?",
        "Були проблеми з концентрацією?",
        "Наскільки ви відчували підтримку команди?",
        "Чи траплялися епізоди роздратування?",
        "Відчуття безнадійності сьогодні?",
        "Наскільки часто ви робили перерви?",
        "Чи виникали думки про уникнення завдань?",
        "Чи було бажання ізоляції від колег?",
        "Темп роботи: комфортний чи надмірний?",
        "Чи відчували ви сенс у роботі сьогодні?",
        "Рівень мотивації протягом дня?",
        "Чи було достатньо відпочинку?",
        "Чи були панічні епізоди?",
        "Чи відчували швидку втому?",
        "Чи були порушення апетиту?",
        "Наскільки ви задоволені прогресом?",
        "Чи отримали ви зворотній зв'язок від колег?",
        "Чи було важко розпочати робочий день?",
        "Чи відчували ви гордість за свою роботу сьогодні?",
    ];

    for (idx, prompt) in prompts.into_iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO questions (id, text, order_index)
            VALUES ($1, $2, $3)
            "#,
            (idx + 1) as i32,
            prompt,
            (idx + 1) as i32
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

struct SeedMetric<'a> {
    email: &'a str,
    period: NaiveDate,
    metrics: MonthlyMetricInput,
}

fn metrics(
    who5: f64,
    phq9: f64,
    gad7: f64,
    mbi: f64,
    sleep_duration: f64,
    sleep_quality: f64,
    work_life_balance: f64,
    stress_level: f64,
) -> MonthlyMetricInput {
    MonthlyMetricInput {
        who5,
        phq9,
        gad7,
        mbi,
        sleep_duration,
        sleep_quality,
        work_life_balance,
        stress_level,
    }
}

fn period(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).expect("valid period")
}

async fn seed_analytics(pool: &PgPool) -> Result<()> {
    let existing: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) as \"count!\" FROM analytics_monthly_metrics")
            .fetch_one(pool)
            .await?;
    let seeded_metrics = existing == 0;
    if seeded_metrics {
        let rows = sqlx::query!("SELECT id, email FROM users")
            .fetch_all(pool)
            .await?;
        let mut user_map: HashMap<String, Uuid> = HashMap::new();
        for row in rows {
            user_map.insert(row.email, row.id);
        }

        let seeds = vec![
            SeedMetric {
                email: "kateryna.petukhova@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(72.0, 1.0, 2.0, 40.0, 7.2, 7.0, 7.0, 12.0),
            },
            SeedMetric {
                email: "kateryna.petukhova@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(40.0, 14.0, 15.0, 33.33, 6.5, 6.0, 5.0, 18.0),
            },
            SeedMetric {
                email: "kateryna.petukhova@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(48.0, 14.0, 12.0, 44.44, 6.3, 5.0, 5.0, 15.0),
            },
            SeedMetric {
                email: "kateryna.petukhova@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(48.0, 13.0, 11.0, 44.44, 6.2, 5.0, 5.0, 16.0),
            },
            SeedMetric {
                email: "kateryna.petukhova@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(40.0, 16.0, 13.0, 81.5, 5.3, 1.0, 2.0, 39.0),
            },
            SeedMetric {
                email: "ivanna.sakalo@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(60.0, 2.0, 3.0, 45.0, 6.8, 6.0, 6.0, 14.0),
            },
            SeedMetric {
                email: "ivanna.sakalo@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(44.0, 7.0, 10.0, 16.67, 6.5, 6.0, 5.0, 16.0),
            },
            SeedMetric {
                email: "ivanna.sakalo@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(36.0, 11.0, 14.0, 22.22, 6.0, 5.0, 5.0, 18.0),
            },
            SeedMetric {
                email: "ivanna.sakalo@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(72.0, 8.0, 5.0, 22.22, 7.2, 7.0, 7.0, 10.0),
            },
            SeedMetric {
                email: "ivanna.sakalo@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(32.0, 11.0, 8.0, 68.0, 5.7, 3.0, 3.0, 30.0),
            },
            SeedMetric {
                email: "mykhailo.ivashchuk@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(52.0, 6.0, 7.0, 62.0, 6.2, 6.0, 5.0, 18.0),
            },
            SeedMetric {
                email: "mykhailo.ivashchuk@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(44.0, 17.0, 15.0, 50.0, 5.8, 5.0, 4.0, 20.0),
            },
            SeedMetric {
                email: "mykhailo.ivashchuk@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(36.0, 9.0, 14.0, 44.44, 5.5, 4.0, 4.0, 19.0),
            },
            SeedMetric {
                email: "mykhailo.ivashchuk@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(56.0, 13.0, 10.0, 44.44, 6.0, 5.0, 5.0, 15.0),
            },
            SeedMetric {
                email: "mykhailo.ivashchuk@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(56.0, 15.0, 12.0, 70.0, 5.6, 1.0, 3.0, 36.0),
            },
            SeedMetric {
                email: "janedavydiuk@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            },
            SeedMetric {
                email: "janedavydiuk@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(88.0, 2.0, 4.0, 5.56, 7.8, 8.0, 8.0, 4.0),
            },
            SeedMetric {
                email: "janedavydiuk@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(88.0, 3.0, 9.0, 11.11, 7.7, 8.0, 8.0, 5.0),
            },
            SeedMetric {
                email: "janedavydiuk@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(90.0, 3.0, 8.0, 11.11, 7.8, 8.0, 8.0, 5.0),
            },
            SeedMetric {
                email: "janedavydiuk@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(84.0, 5.0, 6.0, 27.0, 6.8, 5.0, 6.0, 18.0),
            },
            SeedMetric {
                email: "oksana.klinchaian@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(72.0, 2.0, 3.0, 40.0, 7.2, 7.0, 7.0, 10.0),
            },
            SeedMetric {
                email: "oksana.klinchaian@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(64.0, 3.0, 4.0, 33.33, 7.0, 6.0, 6.0, 11.0),
            },
            SeedMetric {
                email: "oksana.klinchaian@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(72.0, 5.0, 4.0, 27.78, 7.2, 7.0, 7.0, 8.0),
            },
            SeedMetric {
                email: "oksana.klinchaian@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(60.0, 11.0, 8.0, 27.78, 6.8, 6.0, 6.0, 12.0),
            },
            SeedMetric {
                email: "oksana.klinchaian@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(76.0, 7.0, 7.0, 36.5, 6.6, 5.0, 6.0, 21.0),
            },
            SeedMetric {
                email: "iryna.miachkova@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(72.0, 1.0, 2.0, 40.0, 7.5, 8.0, 7.0, 8.0),
            },
            SeedMetric {
                email: "iryna.miachkova@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(68.0, 4.0, 5.0, 11.11, 7.3, 7.0, 7.0, 8.0),
            },
            SeedMetric {
                email: "iryna.miachkova@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(80.0, 7.0, 8.0, 16.67, 7.5, 8.0, 8.0, 9.0),
            },
            SeedMetric {
                email: "iryna.miachkova@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(60.0, 7.0, 7.0, 16.67, 7.0, 7.0, 7.0, 9.0),
            },
            SeedMetric {
                email: "iryna.miachkova@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(84.0, 6.0, 7.0, 30.5, 6.7, 5.0, 6.0, 20.0),
            },
            SeedMetric {
                email: "veronika.kukharchuk@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(92.0, 1.0, 2.0, 30.0, 7.8, 8.0, 8.0, 6.0),
            },
            SeedMetric {
                email: "veronika.kukharchuk@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(60.0, 4.0, 8.0, 38.89, 7.0, 7.0, 7.0, 9.0),
            },
            SeedMetric {
                email: "veronika.kukharchuk@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(48.0, 7.0, 5.0, 38.89, 6.8, 7.0, 7.0, 8.0),
            },
            SeedMetric {
                email: "veronika.kukharchuk@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(88.0, 6.0, 3.0, 38.89, 7.5, 8.0, 8.0, 7.0),
            },
            SeedMetric {
                email: "veronika.kukharchuk@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(88.0, 3.0, 3.0, 16.5, 7.1, 7.0, 7.0, 12.0),
            },
            SeedMetric {
                email: "mariya.vasylyk@opslab.uk",
                period: period(2025, 8),
                metrics: metrics(60.0, 1.0, 2.0, 45.0, 7.0, 7.0, 7.0, 10.0),
            },
            SeedMetric {
                email: "mariya.vasylyk@opslab.uk",
                period: period(2025, 9),
                metrics: metrics(64.0, 4.0, 2.0, 33.33, 6.8, 7.0, 6.0, 9.0),
            },
            SeedMetric {
                email: "mariya.vasylyk@opslab.uk",
                period: period(2025, 10),
                metrics: metrics(52.0, 3.0, 3.0, 27.78, 6.8, 7.0, 6.0, 7.0),
            },
            SeedMetric {
                email: "mariya.vasylyk@opslab.uk",
                period: period(2025, 11),
                metrics: metrics(60.0, 5.0, 5.0, 27.78, 7.0, 7.0, 6.0, 8.0),
            },
            SeedMetric {
                email: "mariya.vasylyk@opslab.uk",
                period: period(2025, 12),
                metrics: metrics(64.0, 3.0, 3.0, 28.5, 6.8, 7.0, 6.0, 15.0),
            },
        ];

        for seed in seeds {
            if let Some(user_id) = user_map.get(seed.email) {
                upsert_monthly_metric(pool, *user_id, seed.period, &seed.metrics, "seeded")
                    .await?;
            }
        }
    }

    let alert_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) as \"count!\" FROM analytics_alerts")
            .fetch_one(pool)
            .await?;
    if alert_count == 0 {
        let user_lookup = sqlx::query!("SELECT id, email FROM users")
            .fetch_all(pool)
            .await?;
        let mut user_map: HashMap<String, Uuid> = HashMap::new();
        for row in user_lookup {
            user_map.insert(row.email, row.id);
        }
        let timestamp: DateTime<Utc> =
            DateTime::parse_from_rfc3339("2025-12-31T10:00:00Z")
                .expect("valid timestamp")
                .with_timezone(&Utc);

        let alerts = vec![
            (
                "critical",
                "kateryna.petukhova@opslab.uk",
                "Катерина Петухова",
                "КРИТИЧНИЙ СТАН! WHO-5: 40, PHQ-9: 16 (помірно-тяжка депресія), MBI: 81.5%. Негайна психологічна допомога!",
            ),
            (
                "critical",
                "ivanna.sakalo@opslab.uk",
                "Іванна Сакало",
                "КРИТИЧНЕ ПОГІРШЕННЯ! WHO-5 впав з 72 (листопад) до 32 (грудень). PHQ-9: 11. Негайна інтервенція!",
            ),
            (
                "critical",
                "mykhailo.ivashchuk@opslab.uk",
                "Михайло Іващук",
                "Критичний рівень вигорання 70%, PHQ-9: 15 (помірно-тяжка депресія). Негайна підтримка!",
            ),
            (
                "positive",
                "oksana.klinchaian@opslab.uk",
                "Оксана Клінчаян",
                "Відмінне покращення в грудні! WHO-5: 76 (+16 від листопада), PHQ-9: 7 (з 11). Позитивна динаміка!",
            ),
            (
                "positive",
                "iryna.miachkova@opslab.uk",
                "Ірина М'ячкова",
                "Дуже сильне покращення! WHO-5: 84 (+24 від листопада). Відмінна динаміка відновлення!",
            ),
            (
                "positive",
                "veronika.kukharchuk@opslab.uk",
                "Вероніка Кухарчук",
                "Стабільно відмінні показники! WHO-5: 88, MBI знизився до 16.5%. Зразковий стан!",
            ),
        ];

        for (severity, email, name, message) in alerts {
            let employee_id = user_map.get(email).copied();
            sqlx::query!(
                r#"
                INSERT INTO analytics_alerts (severity, employee_id, employee_name, message, timestamp)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                severity,
                employee_id,
                name,
                message,
                timestamp
            )
            .execute(pool)
            .await?;
        }
    }

    let recommendation_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) as \"count!\" FROM analytics_recommendations")
            .fetch_one(pool)
            .await?;
    if recommendation_count == 0 {
        let recommendations = vec![
            (
                "emergency",
                "ТЕРМІНОВА психологічна інтервенція",
                "3 співробітників в критичному стані (Катерина, Іванна, Михайло) потребують НЕГАЙНОЇ професійної психологічної допомоги",
                json!(["Катерина Петухова", "Іванна Сакало", "Михайло Іващук"]),
                "critical",
            ),
            (
                "urgent",
                "Зменшення робочого навантаження",
                "Для співробітників в критичному стані необхідно негайно перерозподілити задачі та надати відпустку",
                json!(["Катерина Петухова", "Іванна Сакало", "Михайло Іващук"]),
                "critical",
            ),
            (
                "positive",
                "Аналіз успішних практик відновлення",
                "Вивчити фактори успіху Оксани, Ірини та Вероніки для застосування в команді",
                json!(["Оксана Клінчаян", "Ірина М'ячкова", "Вероніка Кухарчук"]),
                "high",
            ),
            (
                "general",
                "Організаційні зміни",
                "Критично високий рівень вигорання команди (44.81%) вимагає системних змін в організації роботи",
                json!(["all"]),
                "critical",
            ),
        ];

        for (category, title, description, affected, priority) in recommendations {
            sqlx::query!(
                r#"
                INSERT INTO analytics_recommendations (category, title, description, affected_employees, priority)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                category,
                title,
                description,
                affected,
                priority
            )
            .execute(pool)
            .await?;
        }
    }

    if seeded_metrics {
        sqlx::query(
            "UPDATE analytics_metadata SET updated_at = TIMESTAMPTZ '2025-12-31 10:00:00+00' WHERE company = 'OpsLab'",
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
