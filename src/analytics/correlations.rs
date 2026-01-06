///! Correlation Insights (#7)
///! Аналізує кореляції між показниками (sleep→mood, stress→concentration, day patterns)

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CorrelationInsight {
    pub correlation_type: String,
    pub strength: f64, // -1.0 to 1.0 (Pearson correlation coefficient)
    pub description: String,
    pub recommendation: String,
}

/// Аналізувати всі кореляції для користувача
pub async fn analyze_correlations(pool: &PgPool, user_id: Uuid) -> Result<Vec<CorrelationInsight>> {
    let mut insights = Vec::new();

    // 1. Sleep → Mood correlation
    if let Ok(sleep_mood) = calculate_sleep_mood_correlation(pool, user_id).await {
        if sleep_mood.abs() > 0.5 {
            // Strong correlation
            insights.push(CorrelationInsight {
                correlation_type: "sleep_mood".to_string(),
                strength: sleep_mood,
                description: format!(
                    "Твій сон {} пов'язаний з настроєм (r={:.2})",
                    if sleep_mood > 0.0 {
                        "сильно"
                    } else {
                        "негативно"
                    },
                    sleep_mood
                ),
                recommendation: if sleep_mood > 0.0 {
                    "💤 Якість сну напряму впливає на настрій. Пріоритизуй 7-8 годин щодня!".to_string()
                } else {
                    "🤔 Цікаво: твій сон не корелює з настроєм. Шукай інші фактори (stress, workload).".to_string()
                },
            });
        }
    }

    // 2. Stress → Concentration correlation
    if let Ok(stress_focus) = calculate_stress_concentration_correlation(pool, user_id).await {
        if stress_focus.abs() > 0.4 {
            insights.push(CorrelationInsight {
                correlation_type: "stress_concentration".to_string(),
                strength: stress_focus,
                description: format!(
                    "Стрес {} концентрацію (r={:.2})",
                    if stress_focus < 0.0 {
                        "знижує"
                    } else {
                        "підвищує"
                    },
                    stress_focus
                ),
                recommendation: if stress_focus < -0.5 {
                    "⚠️ Високий стрес руйнує концентрацію. Рекомендації: meditation, breaks кожні 90 хв, прогулянки.".to_string()
                } else {
                    "✅ Стрес не сильно впливає на концентрацію. Це добре!".to_string()
                },
            });
        }
    }

    // 3. Energy → Productivity correlation
    if let Ok(energy_prod) = calculate_energy_productivity_correlation(pool, user_id).await {
        if energy_prod > 0.5 {
            insights.push(CorrelationInsight {
                correlation_type: "energy_productivity".to_string(),
                strength: energy_prod,
                description: format!("Енергія сильно впливає на продуктивність (r={:.2})", energy_prod),
                recommendation: "⚡ Підтримуй енергію: якісний сон, healthy snacks, рухайся кожні 2 години!".to_string(),
            });
        }
    }

    // 4. Day of week patterns
    if let Ok((best_day, worst_day)) = find_best_worst_days(pool, user_id).await {
        insights.push(CorrelationInsight {
            correlation_type: "day_of_week".to_string(),
            strength: 1.0,
            description: format!(
                "Твій найкращий день: {}, найгірший: {}",
                day_name(best_day),
                day_name(worst_day)
            ),
            recommendation: format!(
                "📅 Плануй важливі завдання на {}. В {} - легші задачі та self-care.",
                day_name(best_day),
                day_name(worst_day)
            ),
        });
    }

    // 5. Workload → Burnout correlation
    if let Ok(workload_burnout) = calculate_workload_burnout_correlation(pool, user_id).await {
        if workload_burnout > 0.6 {
            insights.push(CorrelationInsight {
                correlation_type: "workload_burnout".to_string(),
                strength: workload_burnout,
                description: format!("Високе навантаження ⇒ burnout (r={:.2})", workload_burnout),
                recommendation: "🚨 Делегуй завдання! Говори з керівником про навантаження. Burnout небезпечний!".to_string(),
            });
        }
    }

    Ok(insights)
}

/// Sleep → Mood Pearson correlation
async fn calculate_sleep_mood_correlation(pool: &PgPool, user_id: Uuid) -> Result<f64> {
    let result = sqlx::query!(
        r#"
        WITH daily_data AS (
            SELECT
                DATE(created_at) as day,
                AVG(CASE WHEN question_type = 'sleep' THEN value ELSE NULL END) as sleep,
                AVG(CASE WHEN question_type = 'mood' THEN value ELSE NULL END) as mood
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            HAVING
                AVG(CASE WHEN question_type = 'sleep' THEN value ELSE NULL END) IS NOT NULL
                AND AVG(CASE WHEN question_type = 'mood' THEN value ELSE NULL END) IS NOT NULL
        )
        SELECT
            CORR(sleep, mood) as "correlation"
        FROM daily_data
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.correlation.unwrap_or(0.0))
}

/// Stress → Concentration correlation (negative expected)
async fn calculate_stress_concentration_correlation(pool: &PgPool, user_id: Uuid) -> Result<f64> {
    let result = sqlx::query!(
        r#"
        WITH daily_data AS (
            SELECT
                DATE(created_at) as day,
                AVG(CASE WHEN question_type = 'stress' THEN value ELSE NULL END) as stress,
                AVG(CASE WHEN question_type IN ('focus', 'concentration') THEN value ELSE NULL END) as concentration
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            HAVING
                AVG(CASE WHEN question_type = 'stress' THEN value ELSE NULL END) IS NOT NULL
                AND AVG(CASE WHEN question_type IN ('focus', 'concentration') THEN value ELSE NULL END) IS NOT NULL
        )
        SELECT
            CORR(stress, concentration) as "correlation"
        FROM daily_data
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.correlation.unwrap_or(0.0))
}

/// Energy → Productivity (motivation) correlation
async fn calculate_energy_productivity_correlation(pool: &PgPool, user_id: Uuid) -> Result<f64> {
    let result = sqlx::query!(
        r#"
        WITH daily_data AS (
            SELECT
                DATE(created_at) as day,
                AVG(CASE WHEN question_type = 'energy' THEN value ELSE NULL END) as energy,
                AVG(CASE WHEN question_type = 'motivation' THEN value ELSE NULL END) as productivity
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            HAVING
                AVG(CASE WHEN question_type = 'energy' THEN value ELSE NULL END) IS NOT NULL
                AND AVG(CASE WHEN question_type = 'motivation' THEN value ELSE NULL END) IS NOT NULL
        )
        SELECT
            CORR(energy, productivity) as "correlation"
        FROM daily_data
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.correlation.unwrap_or(0.0))
}

/// Workload → Burnout correlation
async fn calculate_workload_burnout_correlation(pool: &PgPool, user_id: Uuid) -> Result<f64> {
    let result = sqlx::query!(
        r#"
        WITH daily_data AS (
            SELECT
                DATE(created_at) as day,
                AVG(CASE WHEN question_type = 'workload' THEN value ELSE NULL END) as workload,
                AVG(CASE WHEN question_type = 'stress' THEN value ELSE NULL END) as stress
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            HAVING
                AVG(CASE WHEN question_type = 'workload' THEN value ELSE NULL END) IS NOT NULL
                AND AVG(CASE WHEN question_type = 'stress' THEN value ELSE NULL END) IS NOT NULL
        )
        SELECT
            CORR(workload, stress) as "correlation"
        FROM daily_data
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.correlation.unwrap_or(0.0))
}

/// Знайти найкращий і найгірший день тижня
async fn find_best_worst_days(pool: &PgPool, user_id: Uuid) -> Result<(u32, u32)> {
    let result = sqlx::query!(
        r#"
        WITH day_averages AS (
            SELECT
                EXTRACT(DOW FROM created_at)::INT as dow,
                AVG(value) as avg_value
            FROM checkin_answers
            WHERE user_id = $1
              AND created_at >= NOW() - INTERVAL '60 days'
            GROUP BY dow
        )
        SELECT
            (SELECT dow FROM day_averages ORDER BY avg_value DESC LIMIT 1) as "best_day!",
            (SELECT dow FROM day_averages ORDER BY avg_value ASC LIMIT 1) as "worst_day!"
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok((result.best_day as u32, result.worst_day as u32))
}

fn day_name(dow: u32) -> &'static str {
    match dow {
        0 => "Неділя",
        1 => "Понеділок",
        2 => "Вівторок",
        3 => "Середа",
        4 => "Четвер",
        5 => "П'ятниця",
        6 => "Субота",
        _ => "Невідомо",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_names() {
        assert_eq!(day_name(0), "Неділя");
        assert_eq!(day_name(1), "Понеділок");
        assert_eq!(day_name(5), "П'ятниця");
    }
}
