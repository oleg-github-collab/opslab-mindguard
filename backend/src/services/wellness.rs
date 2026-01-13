use crate::bot::daily_checkin::Metrics;
use crate::db::GoalSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub duration_minutes: Option<i16>,
}

pub fn generate_daily_plan(metrics: Option<&Metrics>, goals: &GoalSettings) -> Vec<PlanItem> {
    let mut candidates: Vec<(i32, PlanItem)> = Vec::new();

    if let Some(m) = metrics {
        if m.sleep_duration < goals.sleep_target as f64 || m.sleep_duration < 6.0 {
            candidates.push((
                95,
                PlanItem {
                    id: "sleep_reset".to_string(),
                    title: "Сон".to_string(),
                    description: format!(
                        "Плануй розслаблення: без екранів 60 хв до сну. Ціль: {} год.",
                        goals.sleep_target
                    ),
                    category: "sleep".to_string(),
                    duration_minutes: Some(15),
                },
            ));
        }

        if m.stress_level >= 20.0 || m.mbi_score >= 60.0 {
            candidates.push((
                92,
                PlanItem {
                    id: "decompress_break".to_string(),
                    title: "Декомпресія".to_string(),
                    description: "8-10 хв без екранів. Дихання 4-7-8 + розтяжка."
                        .to_string(),
                    category: "stress".to_string(),
                    duration_minutes: Some(10),
                },
            ));
        }

        if m.work_life_balance < 5.0 {
            candidates.push((
                80,
                PlanItem {
                    id: "boundary".to_string(),
                    title: "Постав межу".to_string(),
                    description:
                        "Визнач стоп-тайм і захисти 30 хв для відновлення.".to_string(),
                    category: "balance".to_string(),
                    duration_minutes: Some(30),
                },
            ));
        }

        if m.stress_level >= 16.0 || m.mbi_score >= 50.0 {
            candidates.push((
                70,
                PlanItem {
                    id: "micro_pause".to_string(),
                    title: "Мікропаузи".to_string(),
                    description: format!(
                        "Зроби {} коротких пауз. Встань, подихай, випий води.",
                        goals.break_target
                    ),
                    category: "breaks".to_string(),
                    duration_minutes: Some(3),
                },
            ));
        }

        if m.sleep_duration >= goals.sleep_target as f64 && m.stress_level < 16.0 {
            candidates.push((
                65,
                PlanItem {
                    id: "momentum".to_string(),
                    title: "Захисти фокус".to_string(),
                    description:
                        "Обери 1 пріоритет і заблокуй 45 хв deep focus без мітингів."
                            .to_string(),
                    category: "focus".to_string(),
                    duration_minutes: Some(45),
                },
            ));
        }
    }

    candidates.push((
        75,
        PlanItem {
            id: "movement".to_string(),
            title: "Рух".to_string(),
            description: format!(
                "Рухайся {} хв (прогулянка, сходи, розтяжка).",
                goals.move_target
            ),
            category: "movement".to_string(),
            duration_minutes: Some(goals.move_target),
        },
    ));

    candidates.push((
        60,
        PlanItem {
            id: "hydrate".to_string(),
            title: "Вода".to_string(),
            description: "Випий 2 склянки води перед наступною задачею.".to_string(),
            category: "recovery".to_string(),
            duration_minutes: Some(5),
        },
    ));

    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let mut selected = Vec::new();
    for (_, item) in candidates {
        if selected.len() >= 3 {
            break;
        }
        if !selected.iter().any(|i: &PlanItem| i.id == item.id) {
            selected.push(item);
        }
    }

    if selected.is_empty() {
        selected.push(PlanItem {
            id: "reset".to_string(),
                    title: "Перезапуск".to_string(),
            description: "5 хв дихання, розтяжки та 1 маленька перемога.".to_string(),
            category: "recovery".to_string(),
            duration_minutes: Some(5),
        });
    }

    selected
}

pub fn plan_to_text(items: &[PlanItem]) -> String {
    let mut lines = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let duration = item
            .duration_minutes
            .map(|m| format!(" ({m} min)"))
            .unwrap_or_default();
        lines.push(format!("{}. {}{} - {}", idx + 1, item.title, duration, item.description));
    }
    lines.join("\n")
}
