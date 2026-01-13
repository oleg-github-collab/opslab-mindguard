use crate::db;
use crate::bot::daily_checkin::Metrics;
use anyhow::Result;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSnapshot {
    pub days: i64,
    pub who5: f64,
    pub phq9: f64,
    pub gad7: f64,
    pub burnout: f64,
    pub stress: f64,
    pub sleep: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkDeltas {
    pub who5: f64,
    pub phq9: f64,
    pub gad7: f64,
    pub burnout: f64,
    pub stress: f64,
    pub sleep: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfBenchmark {
    pub current: BenchmarkSnapshot,
    pub previous: Option<BenchmarkSnapshot>,
    pub deltas: Option<BenchmarkDeltas>,
}

pub async fn build_self_benchmark(pool: &PgPool, user_id: Uuid) -> Result<Option<SelfBenchmark>> {
    let now = Utc::now();
    let current_start = now - Duration::days(7);
    let current_end = now;
    let prev_start = now - Duration::days(14);
    let prev_end = now - Duration::days(7);

    let current = db::calculate_user_metrics_for_period(pool, user_id, current_start, current_end)
        .await?;
    let Some(current_metrics) = current else {
        return Ok(None);
    };

    let previous = db::calculate_user_metrics_for_period(pool, user_id, prev_start, prev_end).await?;

    let current_snapshot = snapshot_from_metrics(&current_metrics, 7);
    let previous_snapshot = previous.as_ref().map(|m| snapshot_from_metrics(m, 7));

    let deltas = previous_snapshot.as_ref().map(|prev| BenchmarkDeltas {
        who5: current_snapshot.who5 - prev.who5,
        phq9: current_snapshot.phq9 - prev.phq9,
        gad7: current_snapshot.gad7 - prev.gad7,
        burnout: current_snapshot.burnout - prev.burnout,
        stress: current_snapshot.stress - prev.stress,
        sleep: current_snapshot.sleep - prev.sleep,
        balance: current_snapshot.balance - prev.balance,
    });

    Ok(Some(SelfBenchmark {
        current: current_snapshot,
        previous: previous_snapshot,
        deltas,
    }))
}

fn snapshot_from_metrics(metrics: &Metrics, days: i64) -> BenchmarkSnapshot {
    BenchmarkSnapshot {
        days,
        who5: metrics.who5_score,
        phq9: metrics.phq9_score,
        gad7: metrics.gad7_score,
        burnout: metrics.mbi_score,
        stress: metrics.stress_level,
        sleep: metrics.sleep_duration,
        balance: metrics.work_life_balance,
    }
}
