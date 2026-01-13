use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalDeltas {
    pub stress: Option<f64>,
    pub mood: Option<f64>,
    pub energy: Option<f64>,
    pub sleep: Option<f64>,
    pub workload: Option<f64>,
    pub burnout_slope: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarlySignal {
    pub level: String,
    pub score: f64,
    pub confidence: f64,
    pub indicators: Vec<String>,
    pub deltas: SignalDeltas,
}

#[derive(Debug, Clone)]
struct DailyRow {
    day: NaiveDate,
    stress: Option<f64>,
    mood: Option<f64>,
    energy: Option<f64>,
    sleep: Option<f64>,
    workload: Option<f64>,
}

pub async fn detect_early_signal(pool: &PgPool, user_id: Uuid) -> Result<Option<EarlySignal>> {
    let rows = sqlx::query(
        r#"
        SELECT
            DATE(created_at) as day,
            AVG(CASE WHEN question_type = 'stress' THEN value END) as stress,
            AVG(CASE WHEN question_type = 'mood' THEN value END) as mood,
            AVG(CASE WHEN question_type = 'energy' THEN value END) as energy,
            AVG(CASE WHEN question_type = 'sleep' THEN value END) as sleep,
            AVG(CASE WHEN question_type = 'workload' THEN value END) as workload
        FROM checkin_answers
        WHERE user_id = $1
          AND created_at >= NOW() - INTERVAL '28 days'
        GROUP BY DATE(created_at)
        ORDER BY day ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut daily = Vec::new();
    for row in rows {
        daily.push(DailyRow {
            day: row.try_get("day")?,
            stress: row.try_get("stress")?,
            mood: row.try_get("mood")?,
            energy: row.try_get("energy")?,
            sleep: row.try_get("sleep")?,
            workload: row.try_get("workload")?,
        });
    }

    if daily.len() < 10 {
        return Ok(None);
    }

    let recent_len = 7.min(daily.len());
    let baseline_len = daily.len().saturating_sub(recent_len).min(21);
    if baseline_len < 5 {
        return Ok(None);
    }

    let recent = &daily[daily.len() - recent_len..];
    let baseline = &daily[daily.len() - recent_len - baseline_len..daily.len() - recent_len];

    let avg = |vals: &[Option<f64>]| -> Option<f64> {
        let mut sum = 0.0;
        let mut count = 0.0;
        for v in vals.iter().copied().flatten() {
            sum += v;
            count += 1.0;
        }
        if count < 3.0 {
            None
        } else {
            Some(sum / count)
        }
    };

    let delta = |recent: Option<f64>, baseline: Option<f64>| -> Option<f64> {
        match (recent, baseline) {
            (Some(r), Some(b)) => Some(r - b),
            _ => None,
        }
    };

    let stress_delta = delta(
        avg(&recent.iter().map(|d| d.stress).collect::<Vec<_>>()),
        avg(&baseline.iter().map(|d| d.stress).collect::<Vec<_>>()),
    );
    let mood_delta = delta(
        avg(&recent.iter().map(|d| d.mood).collect::<Vec<_>>()),
        avg(&baseline.iter().map(|d| d.mood).collect::<Vec<_>>()),
    );
    let energy_delta = delta(
        avg(&recent.iter().map(|d| d.energy).collect::<Vec<_>>()),
        avg(&baseline.iter().map(|d| d.energy).collect::<Vec<_>>()),
    );
    let sleep_delta = delta(
        avg(&recent.iter().map(|d| d.sleep).collect::<Vec<_>>()),
        avg(&baseline.iter().map(|d| d.sleep).collect::<Vec<_>>()),
    );
    let workload_delta = delta(
        avg(&recent.iter().map(|d| d.workload).collect::<Vec<_>>()),
        avg(&baseline.iter().map(|d| d.workload).collect::<Vec<_>>()),
    );

    let burnout_slope = burnout_trend_slope(&daily);

    let mut score = 0.0;
    let mut indicators = Vec::new();

    if let Some(delta) = stress_delta {
        if delta >= 1.5 {
            score += 2.0;
            indicators.push("stress spike vs baseline".to_string());
        } else if delta >= 1.0 {
            score += 1.0;
            indicators.push("stress rising".to_string());
        }
    }

    if let Some(delta) = mood_delta {
        if delta <= -1.2 {
            score += 2.0;
            indicators.push("mood drop".to_string());
        } else if delta <= -0.8 {
            score += 1.0;
            indicators.push("mood drifting down".to_string());
        }
    }

    if let Some(delta) = energy_delta {
        if delta <= -1.2 {
            score += 1.0;
            indicators.push("energy decrease".to_string());
        }
    }

    if let Some(delta) = sleep_delta {
        if delta <= -0.8 {
            score += 1.0;
            indicators.push("sleep quality drop".to_string());
        }
    }

    if let Some(delta) = workload_delta {
        if delta >= 1.5 {
            score += 1.0;
            indicators.push("workload spike".to_string());
        }
    }

    if let Some(slope) = burnout_slope {
        if slope >= 0.12 {
            score += 2.0;
            indicators.push("burnout slope rising".to_string());
        } else if slope >= 0.08 {
            score += 1.0;
            indicators.push("burnout slope uptick".to_string());
        }
    }

    if score < 3.0 || indicators.len() < 2 {
        return Ok(None);
    }

    let confidence = ((daily.len() as f64) / 21.0).clamp(0.35, 1.0);
    let level = if score >= 6.0 {
        "critical"
    } else if score >= 4.0 {
        "alert"
    } else {
        "watch"
    };

    Ok(Some(EarlySignal {
        level: level.to_string(),
        score,
        confidence,
        indicators,
        deltas: SignalDeltas {
            stress: stress_delta,
            mood: mood_delta,
            energy: energy_delta,
            sleep: sleep_delta,
            workload: workload_delta,
            burnout_slope,
        },
    }))
}

fn burnout_trend_slope(daily: &[DailyRow]) -> Option<f64> {
    let mut points = Vec::new();
    for (idx, row) in daily.iter().enumerate() {
        let stress = row.stress?;
        let workload = row.workload?;
        let energy = row.energy?;
        let mood = row.mood?;
        let burnout = (stress + workload + (10.0 - energy) + (10.0 - mood)) / 4.0;
        points.push((idx as f64, burnout));
    }

    if points.len() < 6 {
        return None;
    }

    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
    let sum_x2: f64 = points.iter().map(|(x, _)| x * x).sum();
    let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-6 {
        return None;
    }

    Some((n * sum_xy - sum_x * sum_y) / denom)
}
