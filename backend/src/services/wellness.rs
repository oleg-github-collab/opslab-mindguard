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

/// Generate an ultra-detailed, individualized daily wellness plan
/// based on the user's current metrics and personal goals.
/// The plan adapts to the specific combination of metrics to provide
/// targeted, actionable recommendations.
pub fn generate_daily_plan(metrics: Option<&Metrics>, goals: &GoalSettings) -> Vec<PlanItem> {
    let mut candidates: Vec<(i32, PlanItem)> = Vec::new();

    if let Some(m) = metrics {
        // ===== SLEEP RECOVERY =====
        if m.sleep_duration < 4.0 {
            // Critically low sleep
            candidates.push((
                99,
                PlanItem {
                    id: "sleep_emergency".to_string(),
                    title: "Сон: терміново".to_string(),
                    description: format!(
                        "Критично низький сон ({:.1}h). Сьогодні ляг на 1.5 год раніше. \
                        Без кофеїну після 14:00, без екранів за 90 хв до сну. \
                        Ціль: {} год.",
                        m.sleep_duration, goals.sleep_target
                    ),
                    category: "sleep".to_string(),
                    duration_minutes: Some(20),
                },
            ));
        } else if m.sleep_duration < goals.sleep_target as f64 || m.sleep_duration < 6.0 {
            candidates.push((
                95,
                PlanItem {
                    id: "sleep_reset".to_string(),
                    title: "Сон: відновлення".to_string(),
                    description: format!(
                        "Якість сну ({:.1}h) нижче цілі ({} год). \
                        Плануй розслаблення: без екранів 60 хв до сну, \
                        температура кімнати 18-20°C, дихання 4-7-8.",
                        m.sleep_duration, goals.sleep_target
                    ),
                    category: "sleep".to_string(),
                    duration_minutes: Some(15),
                },
            ));
        } else if m.sleep_duration >= 8.0 {
            // Good sleep - reinforce
            candidates.push((
                40,
                PlanItem {
                    id: "sleep_maintain".to_string(),
                    title: "Сон: підтримка".to_string(),
                    description: format!(
                        "Чудовий сон ({:.1}h)! Зберігай режим. \
                        Порада: прокинься в один час навіть на вихідних.",
                        m.sleep_duration
                    ),
                    category: "sleep".to_string(),
                    duration_minutes: Some(5),
                },
            ));
        }

        // ===== STRESS & BURNOUT MANAGEMENT =====
        if m.stress_level >= 30.0 || m.mbi_score >= 80.0 {
            // Critical stress/burnout
            candidates.push((
                98,
                PlanItem {
                    id: "stress_emergency".to_string(),
                    title: "Антистрес: терміново".to_string(),
                    description: format!(
                        "Рівень стресу критичний ({:.0}/40, вигорання {:.0}%). \
                        Негайно: 10 хв дихання box breathing (4-4-4-4). \
                        Скасуй або делегуй 1 некритичну задачу. \
                        Зверни увагу: тіло потребує паузу прямо зараз.",
                        m.stress_level, m.mbi_score
                    ),
                    category: "stress".to_string(),
                    duration_minutes: Some(15),
                },
            ));
        } else if m.stress_level >= 20.0 || m.mbi_score >= 60.0 {
            candidates.push((
                92,
                PlanItem {
                    id: "decompress_break".to_string(),
                    title: "Декомпресія".to_string(),
                    description: format!(
                        "Стрес підвищений ({:.0}/40). \
                        8-10 хв без екранів. Дихання 4-7-8 + легка розтяжка шиї та плечей. \
                        Уяви місце де тобі спокійно.",
                        m.stress_level
                    ),
                    category: "stress".to_string(),
                    duration_minutes: Some(10),
                },
            ));
        } else if m.stress_level >= 16.0 || m.mbi_score >= 50.0 {
            candidates.push((
                70,
                PlanItem {
                    id: "micro_pause".to_string(),
                    title: "Мікропаузи".to_string(),
                    description: format!(
                        "Помірний стрес ({:.0}/40). Зроби {} коротких пауз протягом дня. \
                        Кожна пауза: встань, 5 глибоких вдихів, випий води, подивись у вікно.",
                        m.stress_level, goals.break_target
                    ),
                    category: "breaks".to_string(),
                    duration_minutes: Some(3),
                },
            ));
        }

        // ===== WORK-LIFE BALANCE =====
        if m.work_life_balance < 3.0 {
            // Critical imbalance
            candidates.push((
                88,
                PlanItem {
                    id: "boundary_urgent".to_string(),
                    title: "Межі: терміново".to_string(),
                    description: format!(
                        "Баланс критично низький ({:.1}/10). \
                        Сьогодні обовʼязково: 1) визнач стоп-тайм і не працюй після нього, \
                        2) захисти 45 хв для себе (прогулянка, хобі, відпочинок), \
                        3) вимкни робочі нотифікації після стоп-тайму.",
                        m.work_life_balance
                    ),
                    category: "balance".to_string(),
                    duration_minutes: Some(45),
                },
            ));
        } else if m.work_life_balance < 5.0 {
            candidates.push((
                80,
                PlanItem {
                    id: "boundary".to_string(),
                    title: "Постав межу".to_string(),
                    description: format!(
                        "Work-Life Balance ({:.1}/10) потребує уваги. \
                        Визнач стоп-тайм і захисти 30 хв для відновлення. \
                        Правило: 1 годину перед сном — без роботи.",
                        m.work_life_balance
                    ),
                    category: "balance".to_string(),
                    duration_minutes: Some(30),
                },
            ));
        } else if m.work_life_balance >= 7.0 {
            candidates.push((
                35,
                PlanItem {
                    id: "balance_celebrate".to_string(),
                    title: "Баланс: супер".to_string(),
                    description: format!(
                        "Відмінний баланс ({:.1}/10)! Продовжуй. \
                        Порада: запиши що саме допомагає тримати баланс, \
                        щоб повторювати у складні дні.",
                        m.work_life_balance
                    ),
                    category: "balance".to_string(),
                    duration_minutes: Some(5),
                },
            ));
        }

        // ===== MOOD & EMOTIONAL WELL-BEING =====
        if m.who5_score < 40.0 {
            // Very low well-being
            candidates.push((
                90,
                PlanItem {
                    id: "mood_support".to_string(),
                    title: "Настрій: підтримка".to_string(),
                    description: format!(
                        "Благополуччя низьке ({:.0}/100). Сьогодні зроби щось приємне саме для себе: \
                        улюблена музика, прогулянка, чашка чаю у тиші. \
                        Напиши 3 речі за які вдячний сьогодні.",
                        m.who5_score
                    ),
                    category: "mood".to_string(),
                    duration_minutes: Some(15),
                },
            ));
        } else if m.who5_score < 60.0 {
            candidates.push((
                72,
                PlanItem {
                    id: "mood_boost".to_string(),
                    title: "Емоційне відновлення".to_string(),
                    description: format!(
                        "WHO-5 ({:.0}/100) — є простір для покращення. \
                        Зроби 1 річ що приносить радість. \
                        Поділись з кимось як себе почуваєш.",
                        m.who5_score
                    ),
                    category: "mood".to_string(),
                    duration_minutes: Some(10),
                },
            ));
        }

        // ===== FOCUS & PRODUCTIVITY =====
        if m.sleep_duration >= goals.sleep_target as f64 && m.stress_level < 16.0 {
            // Good conditions for deep work
            candidates.push((
                65,
                PlanItem {
                    id: "momentum".to_string(),
                    title: "Захисти фокус".to_string(),
                    description:
                        "Сон та стрес у нормі — ідеальний день для deep work! \
                        Обери 1 пріоритет і заблокуй 45 хв deep focus без мітингів. \
                        Вимкни нотифікації."
                            .to_string(),
                    category: "focus".to_string(),
                    duration_minutes: Some(45),
                },
            ));
        } else if m.who5_score >= 70.0 && m.stress_level < 20.0 {
            candidates.push((
                55,
                PlanItem {
                    id: "flow_state".to_string(),
                    title: "Потік".to_string(),
                    description:
                        "Показники гарні — використай цей стан! \
                        Обери найважливішу задачу і працюй 25 хв без перерв (Pomodoro). \
                        Після — 5 хв повна пауза."
                            .to_string(),
                    category: "focus".to_string(),
                    duration_minutes: Some(30),
                },
            ));
        }

        // ===== COMBINED RISK PATTERNS =====
        // High stress + poor sleep = burnout trajectory
        if m.stress_level >= 20.0 && m.sleep_duration < 6.0 {
            candidates.push((
                96,
                PlanItem {
                    id: "burnout_prevention".to_string(),
                    title: "Профілактика вигорання".to_string(),
                    description: format!(
                        "Комбінація стресу ({:.0}/40) та дефіциту сну ({:.1}h) — \
                        шлях до вигорання. Сьогодні: \
                        1) ніякої роботи після 19:00, \
                        2) лягти на 1 год раніше, \
                        3) делегувати або відкласти 1 задачу.",
                        m.stress_level, m.sleep_duration
                    ),
                    category: "recovery".to_string(),
                    duration_minutes: Some(20),
                },
            ));
        }

        // Low mood + low energy = emotional exhaustion
        if m.who5_score < 50.0 && m.phq9_score >= 10.0 {
            candidates.push((
                93,
                PlanItem {
                    id: "emotional_recharge".to_string(),
                    title: "Емоційна підзарядка".to_string(),
                    description:
                        "Настрій та енергія потребують відновлення. \
                        Рекомендація: \
                        1) Поговори з близькою людиною (10 хв), \
                        2) Зроби щось фізичне (прогулянка 15 хв), \
                        3) Подбай про базові потреби (їжа, вода, повітря)."
                            .to_string(),
                    category: "mood".to_string(),
                    duration_minutes: Some(25),
                },
            ));
        }

        // High workload + low energy = overload signal
        if m.work_life_balance < 4.0 && m.mbi_score >= 50.0 {
            candidates.push((
                85,
                PlanItem {
                    id: "workload_reset".to_string(),
                    title: "Перевантаження".to_string(),
                    description: format!(
                        "Навантаження ({:.1}/10 баланс, вигорання {:.0}%) зашкалює. \
                        Зроби інвентаризацію задач: запиши всі, \
                        відміть 1 яку можна делегувати або відкласти. \
                        Обговори з керівником пріоритети.",
                        m.work_life_balance, m.mbi_score
                    ),
                    category: "balance".to_string(),
                    duration_minutes: Some(15),
                },
            ));
        }
    }

    // ===== UNIVERSAL RECOMMENDATIONS =====
    candidates.push((
        75,
        PlanItem {
            id: "movement".to_string(),
            title: "Рух".to_string(),
            description: format!(
                "Рухайся {} хв (прогулянка, сходи, розтяжка). \
                Навіть 10 хв руху знижують стрес на 20% і покращують фокус.",
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
            description: "Випий 2 склянки води перед наступною задачею. \
                Зневоднення знижує когнітивні функції на 15%."
                .to_string(),
            category: "recovery".to_string(),
            duration_minutes: Some(5),
        },
    ));

    candidates.push((
        50,
        PlanItem {
            id: "gratitude".to_string(),
            title: "Вдячність".to_string(),
            description: "Запиши 3 речі за які вдячний сьогодні. \
                Практика вдячності знижує рівень кортизолу та покращує сон."
                .to_string(),
            category: "mood".to_string(),
            duration_minutes: Some(5),
        },
    ));

    candidates.push((
        45,
        PlanItem {
            id: "social_connect".to_string(),
            title: "Соціальний контакт".to_string(),
            description: "Напиши або подзвони комусь хто тобі небайдужий. \
                5 хв розмови покращують настрій на годину."
                .to_string(),
            category: "mood".to_string(),
            duration_minutes: Some(5),
        },
    ));

    // Sort by priority (highest first)
    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    // Select top items, avoid duplicates and same-category overflow
    let mut selected = Vec::new();
    let mut categories_used = std::collections::HashMap::new();
    let max_items = if metrics.map(|m| m.mbi_score >= 60.0 || m.stress_level >= 24.0).unwrap_or(false) {
        5 // More items for users in distress
    } else {
        3
    };

    for (_, item) in candidates {
        if selected.len() >= max_items {
            break;
        }
        if selected.iter().any(|i: &PlanItem| i.id == item.id) {
            continue;
        }
        // Limit 2 items per category to keep variety
        let cat_count = categories_used.entry(item.category.clone()).or_insert(0);
        if *cat_count >= 2 {
            continue;
        }
        *cat_count += 1;
        selected.push(item);
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
            .map(|m| format!(" ({m} хв)"))
            .unwrap_or_default();
        lines.push(format!("{}. {}{} — {}", idx + 1, item.title, duration, item.description));
    }
    lines.join("\n\n")
}
