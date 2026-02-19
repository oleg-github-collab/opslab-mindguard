use crate::bot::daily_checkin::Metrics;
use crate::db::GoalSettings;
use chrono::Datelike;
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
/// based on the user's current metrics, personal goals, and day of week.
/// Uses multi-metric cross-analysis, severity tiers, and day-aware variety.
pub fn generate_daily_plan(metrics: Option<&Metrics>, goals: &GoalSettings) -> Vec<PlanItem> {
    let mut candidates: Vec<(i32, PlanItem)> = Vec::new();
    let weekday = chrono::Utc::now().weekday().num_days_from_monday(); // 0=Mon..6=Sun

    if let Some(m) = metrics {
        // ===== SLEEP: 5 tiers =====
        add_sleep_items(&mut candidates, m, goals);

        // ===== STRESS & BURNOUT: 5 tiers =====
        add_stress_items(&mut candidates, m, goals);

        // ===== ANXIETY (GAD-7): 4 tiers =====
        add_anxiety_items(&mut candidates, m);

        // ===== DEPRESSION (PHQ-9): 4 tiers =====
        add_depression_items(&mut candidates, m);

        // ===== WORK-LIFE BALANCE: 4 tiers =====
        add_balance_items(&mut candidates, m);

        // ===== MOOD & WELL-BEING: 4 tiers =====
        add_mood_items(&mut candidates, m);

        // ===== FOCUS & COGNITIVE LOAD =====
        add_focus_items(&mut candidates, m, goals);

        // ===== ENERGY MANAGEMENT =====
        add_energy_items(&mut candidates, m);

        // ===== COMBINED MULTI-METRIC PATTERNS =====
        add_combined_patterns(&mut candidates, m, goals);

        // ===== SLEEP QUALITY (separate from duration) =====
        add_sleep_quality_items(&mut candidates, m);

        // ===== MORNING ACTIVATION =====
        add_morning_routine(&mut candidates, m, goals);

        // ===== NUTRITION =====
        add_nutrition_items(&mut candidates, m);

        // ===== BODY CARE =====
        add_body_care_items(&mut candidates, m);

        // ===== SELF-COMPASSION =====
        add_self_compassion_items(&mut candidates, m);

        // ===== POSITIVE REINFORCEMENT =====
        add_positive_reinforcement(&mut candidates, m);
    }

    // ===== UNIVERSAL & DAY-AWARE =====
    add_universal_items(&mut candidates, metrics, goals, weekday);

    // Sort by priority (highest first)
    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    // Determine max items based on severity
    let max_items = match metrics {
        Some(m) if m.mbi_score >= 80.0 || m.stress_level >= 32.0
            || (m.phq9_score >= 15.0 && m.gad7_score >= 10.0) => 7,
        Some(m) if m.mbi_score >= 70.0 || m.stress_level >= 28.0 || m.phq9_score >= 15.0 => 6,
        Some(m) if m.mbi_score >= 50.0 || m.stress_level >= 20.0 || m.phq9_score >= 10.0 => 5,
        Some(m) if m.who5_score < 50.0 || m.gad7_score >= 10.0 => 5,
        _ => 4,
    };

    // Select top items with category diversity (max 2 per category)
    let mut selected = Vec::new();
    let mut categories_used = std::collections::HashMap::new();

    for (_, item) in candidates {
        if selected.len() >= max_items {
            break;
        }
        if selected.iter().any(|i: &PlanItem| i.id == item.id) {
            continue;
        }
        let cat_count = categories_used.entry(item.category.clone()).or_insert(0);
        if *cat_count >= 2 {
            continue;
        }
        *cat_count += 1;
        selected.push(item);
    }

    if selected.is_empty() {
        selected.push(PlanItem {
            id: "reset".into(),
            title: "Перезапуск".into(),
            description: "5 хв дихання, розтяжки та 1 маленька перемога.".into(),
            category: "recovery".into(),
            duration_minutes: Some(5),
        });
    }

    selected
}

// ─────────────────────────────────────────────────────────
// SLEEP
// ─────────────────────────────────────────────────────────
fn add_sleep_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics, goals: &GoalSettings) {
    let target = goals.sleep_target as f64;

    if m.sleep_duration < 4.0 {
        c.push((99, PlanItem {
            id: "sleep_emergency".into(),
            title: "Сон: терміново".into(),
            description: format!(
                "Критично мало сну ({:.1}h при цілі {}h). Організм у режимі виживання. \
                Сьогодні: 1) ляг на 2 год раніше, 2) жодного кофеїну після 13:00, \
                3) за 90 хв до сну: теплий душ, вимкни екрани, вправа 4-7-8. \
                Відклади все що можна на завтра — сон зараз пріоритет №1.",
                m.sleep_duration, goals.sleep_target
            ),
            category: "sleep".into(),
            duration_minutes: Some(20),
        }));
    } else if m.sleep_duration < 5.5 {
        c.push((96, PlanItem {
            id: "sleep_deficit".into(),
            title: "Сон: дефіцит".into(),
            description: format!(
                "Серйозний дефіцит сну ({:.1}h / {}h ціль). Ризик помилок зростає на 70%. \
                План на вечір: стоп-тайм роботи о 20:00. За 60 хв до сну — режим без екранів. \
                Температура 18-19°C. Дихання 4-7-8 перед сном (4с вдих, 7с затримка, 8с видих).",
                m.sleep_duration, goals.sleep_target
            ),
            category: "sleep".into(),
            duration_minutes: Some(15),
        }));
    } else if m.sleep_duration < target || m.sleep_duration < 6.5 {
        c.push((88, PlanItem {
            id: "sleep_improve".into(),
            title: "Сон: покращення".into(),
            description: format!(
                "Сон ({:.1}h) нижче цілі ({:.0}h). \
                Поступово зсувай час на 15-20 хв раніше. \
                Без кофеїну після 15:00, без важкої їжі за 3 год до сну. \
                Вечірній ритуал: книга або спокійна музика замість телефону.",
                m.sleep_duration, target
            ),
            category: "sleep".into(),
            duration_minutes: Some(10),
        }));
    } else if m.sleep_duration >= target && m.sleep_duration < 8.0 {
        c.push((30, PlanItem {
            id: "sleep_good".into(),
            title: "Сон: на рівні".into(),
            description: format!(
                "Сон {:.1}h — ціль досягнуто! Щоб закріпити: \
                вставай в один час навіть на вихідних (±30 хв). \
                Сонячне світло протягом 15 хв після пробудження допомагає циркадному ритму.",
                m.sleep_duration
            ),
            category: "sleep".into(),
            duration_minutes: Some(5),
        }));
    } else if m.sleep_duration >= 8.0 {
        c.push((25, PlanItem {
            id: "sleep_excellent".into(),
            title: "Сон: чудово".into(),
            description: format!(
                "Відмінний сон ({:.1}h)! Ти у зоні максимальної відновлення. \
                Порада дня: запиши що допомогло (час відходу, ритуал, температура) — \
                це твій особистий рецепт якісного сну.",
                m.sleep_duration
            ),
            category: "sleep".into(),
            duration_minutes: Some(3),
        }));
    }

    // Sleep hygiene tip when sleep is mediocre
    if m.sleep_duration >= 5.0 && m.sleep_duration < 7.0 && m.stress_level >= 16.0 {
        c.push((78, PlanItem {
            id: "sleep_hygiene_stress".into(),
            title: "Вечірня деактивація".into(),
            description: format!(
                "При стресі ({:.0}/40) і нестачі сну ({:.1}h) мозок не може \"вимкнутись\". \
                Створи \"буферну зону\": за 45 хв до сну запиши 3 думки що тривожать \
                на папір (brain dump). Це дозволить мозку відпустити контроль.",
                m.stress_level, m.sleep_duration
            ),
            category: "sleep".into(),
            duration_minutes: Some(10),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// STRESS & BURNOUT
// ─────────────────────────────────────────────────────────
fn add_stress_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics, goals: &GoalSettings) {
    if m.stress_level >= 32.0 || m.mbi_score >= 85.0 {
        c.push((99, PlanItem {
            id: "stress_crisis".into(),
            title: "Стрес: критична точка".into(),
            description: format!(
                "Стрес {:.0}/40, вигорання {:.0}%. Це зона ризику для здоров'я. \
                Негайні дії: 1) box breathing 5 хв (4-4-4-4 рахунок), \
                2) скасуй або перенеси 1-2 некритичні зустрічі, \
                3) вийди на свіже повітря на 10 хв. \
                Якщо це продовжується — поговори з керівником або HR.",
                m.stress_level, m.mbi_score
            ),
            category: "stress".into(),
            duration_minutes: Some(15),
        }));
    } else if m.stress_level >= 24.0 || m.mbi_score >= 65.0 {
        c.push((94, PlanItem {
            id: "stress_high".into(),
            title: "Антистрес: активний".into(),
            description: format!(
                "Стрес підвищений ({:.0}/40). Потрібна активна декомпресія: \
                10 хв дихальної вправи (вдих 4с через ніс → затримка 7с → повільний видих 8с ротом). \
                Потім: розтяжка шиї (нахили голови) і плечей (кругові рухи). \
                Увечері: 20 хв без жодних екранів.",
                m.stress_level
            ),
            category: "stress".into(),
            duration_minutes: Some(15),
        }));
    } else if m.stress_level >= 16.0 || m.mbi_score >= 50.0 {
        c.push((78, PlanItem {
            id: "stress_moderate".into(),
            title: "Управління стресом".into(),
            description: format!(
                "Стрес помірний ({:.0}/40). Заплануй {} мікропауз по 3-5 хв сьогодні. \
                Кожна пауза: встань → 5 глибоких вдихів → попий води → подивись у вікно. \
                Після обіду — коротка прогулянка навколо будинку.",
                m.stress_level, goals.break_target
            ),
            category: "stress".into(),
            duration_minutes: Some(5),
        }));
    } else if m.stress_level >= 10.0 {
        c.push((50, PlanItem {
            id: "stress_light".into(),
            title: "Легкий стрес-менеджмент".into(),
            description: format!(
                "Стрес у нормі ({:.0}/40), але профілактика важлива. \
                Зроби 1 річ для себе: улюблена музика, кава в тиші, \
                5-хвилинна розтяжка. Маленькі ритуали утримують баланс.",
                m.stress_level
            ),
            category: "stress".into(),
            duration_minutes: Some(5),
        }));
    }

    // Burnout-specific when high but stress moderate
    if m.mbi_score >= 55.0 && m.stress_level < 20.0 {
        c.push((82, PlanItem {
            id: "burnout_creeping".into(),
            title: "Приховане вигорання".into(),
            description: format!(
                "Вигорання {:.0}% при помірному стресі — класичне \"тихе вигорання\". \
                Стрес може не відчуватись, але тіло виснажене. \
                Сьогодні: 1) перевір чи є задоволення від роботи, \
                2) визнач 1 задачу яка дратує найбільше → делегуй або відклади, \
                3) згадай що тебе колись надихало в роботі.",
                m.mbi_score
            ),
            category: "recovery".into(),
            duration_minutes: Some(10),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// ANXIETY (GAD-7)
// ─────────────────────────────────────────────────────────
fn add_anxiety_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    if m.gad7_score >= 15.0 {
        c.push((95, PlanItem {
            id: "anxiety_severe".into(),
            title: "Тривога: зниження".into(),
            description: format!(
                "GAD-7 = {:.0}/21 — виражена тривожність. \
                Техніка заземлення \"5-4-3-2-1\": назви 5 речей що бачиш, \
                4 що чуєш, 3 що відчуваєш дотиком, 2 що нюхаєш, 1 що смакуєш. \
                Це повертає увагу в \"тут і зараз\" і знижує тривогу за 3-5 хв. \
                Якщо тривога заважає працювати — зверніся до психолога.",
                m.gad7_score
            ),
            category: "anxiety".into(),
            duration_minutes: Some(10),
        }));
    } else if m.gad7_score >= 10.0 {
        c.push((83, PlanItem {
            id: "anxiety_moderate".into(),
            title: "Антитривожна практика".into(),
            description: format!(
                "Тривожність підвищена ({:.0}/21). Спробуй \"worry time\": \
                визнач 10 хв сьогодні щоб записати ВСІ тривоги на папір. \
                Поділи на 2 колонки: \"можу вплинути\" і \"не можу вплинути\". \
                Для першої — запиши 1 конкретний крок. Другу — відпусти свідомо.",
                m.gad7_score
            ),
            category: "anxiety".into(),
            duration_minutes: Some(10),
        }));
    } else if m.gad7_score >= 5.0 {
        c.push((55, PlanItem {
            id: "anxiety_light".into(),
            title: "Профілактика тривоги".into(),
            description: format!(
                "Легка тривожність ({:.0}/21). Гарна практика: \
                3 хв діафрагмального дихання (рука на животі, \
                вдих — живіт піднімається, видих — опускається). \
                Це активує парасимпатичну систему і знижує кортизол.",
                m.gad7_score
            ),
            category: "anxiety".into(),
            duration_minutes: Some(5),
        }));
    }

    // Progressive Muscle Relaxation when anxiety + physical tension
    if m.gad7_score >= 8.0 && m.stress_level >= 16.0 {
        c.push((81, PlanItem {
            id: "pmr_anxiety".into(),
            title: "Прогресивна м'язова релаксація".into(),
            description: format!(
                "Тривога ({:.0}/21) + стрес ({:.0}/40) = тіло затиснуте. \
                Прогресивна м'язова релаксація (Джекобсон): \
                напружуй групу м'язів на 5 сек → розслаб на 15 сек. \
                Послідовність: кулаки → передпліччя → плечі → чоло → щелепа → шия → живіт → ноги. \
                Весь цикл 10-15 хв. Це знижує кортизол і м'язову напругу одночасно.",
                m.gad7_score, m.stress_level
            ),
            category: "anxiety".into(),
            duration_minutes: Some(15),
        }));
    }

    // Cognitive defusion for moderate+ anxiety
    if m.gad7_score >= 10.0 && m.phq9_score < 15.0 {
        c.push((77, PlanItem {
            id: "cognitive_defusion".into(),
            title: "Когнітивна дефузія".into(),
            description: format!(
                "Тривога ({:.0}/21) часто тримається на \"клейких\" думках. \
                Техніка дефузії: коли з'являється тривожна думка, \
                додай перед нею \"я помічаю, що думаю...\". \
                Напр.: не \"все піде погано\", а \"я помічаю думку що все піде погано\". \
                Це створює дистанцію між тобою і думкою. Спробуй з 3 думками зараз.",
                m.gad7_score
            ),
            category: "anxiety".into(),
            duration_minutes: Some(5),
        }));
    }

    // Butterfly hug for acute anxiety
    if m.gad7_score >= 12.0 {
        c.push((84, PlanItem {
            id: "butterfly_hug".into(),
            title: "Техніка \"метелик\"".into(),
            description:
                "Швидка техніка зниження тривоги (EMDR-based): \
                схрести руки на грудях, кінчики пальців на плечах. \
                По черзі постукуй лівою і правою рукою (як крила метелика) \
                протягом 2-3 хв, дихаючи повільно. \
                Білатеральна стимуляція знижує активацію мигдалини за 60-90 сек."
                    .into(),
            category: "anxiety".into(),
            duration_minutes: Some(5),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// DEPRESSION (PHQ-9)
// ─────────────────────────────────────────────────────────
fn add_depression_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    if m.phq9_score >= 15.0 {
        c.push((94, PlanItem {
            id: "depression_support".into(),
            title: "Емоційна підтримка".into(),
            description: format!(
                "PHQ-9 = {:.0}/27 — сигнал що потрібна допомога. \
                Сьогодні: 1) не ізолюйся — напиши або подзвони 1 близькій людині, \
                2) вийди на вулицю хоча б на 10 хв (сонячне світло допомагає), \
                3) зроби 1 маленьку конкретну річ (помити чашку, прибрати стіл). \
                Маленькі дії розбивають порочне коло. Ти не один.",
                m.phq9_score
            ),
            category: "mood".into(),
            duration_minutes: Some(15),
        }));
    } else if m.phq9_score >= 10.0 {
        c.push((84, PlanItem {
            id: "depression_activation".into(),
            title: "Поведінкова активація".into(),
            description: format!(
                "Індикатор настрою ({:.0}/27) потребує уваги. \
                Техніка поведінкової активації: заплануй 1 приємну дію \
                (навіть якщо не хочеться). Це може бути: улюблена їжа, \
                коротка прогулянка, епізод серіалу, розмова з другом. \
                Дія створює мотивацію, а не навпаки.",
                m.phq9_score
            ),
            category: "mood".into(),
            duration_minutes: Some(15),
        }));
    } else if m.phq9_score >= 5.0 {
        c.push((58, PlanItem {
            id: "mood_nurture".into(),
            title: "Підживлення настрою".into(),
            description: format!(
                "Настрій трохи знижений ({:.0}/27). \
                Інвестуй 10 хв у щось що приносить радість: \
                музика, творчість, прогулянка або розмова. \
                Запиши 3 моменти дня за які вдячний — це перемикає фокус мозку.",
                m.phq9_score
            ),
            category: "mood".into(),
            duration_minutes: Some(10),
        }));
    }

    // Self-compassion break for depression + burnout
    if m.phq9_score >= 10.0 && m.mbi_score >= 40.0 {
        c.push((86, PlanItem {
            id: "self_compassion_break".into(),
            title: "Пауза самоспівчуття".into(),
            description: format!(
                "PHQ-9 ({:.0}) + вигорання ({:.0}%) — ти надто суворий до себе. \
                Вправа Крістін Нефф (3 кроки, 2 хв): \
                1) \"Мені зараз важко\" (визнай страждання), \
                2) \"Інші люди теж переживають подібне\" (спільність), \
                3) \"Нехай я буду добрим до себе\" (самодоброта). \
                Поклади руку на серце. Скажи це вголос або подумки.",
                m.phq9_score, m.mbi_score
            ),
            category: "mood".into(),
            duration_minutes: Some(5),
        }));
    }

    // Behavioral scheduling: mastery + pleasure
    if m.phq9_score >= 8.0 {
        c.push((73, PlanItem {
            id: "mastery_pleasure".into(),
            title: "Задоволення + майстерність".into(),
            description: format!(
                "При зниженому настрої ({:.0}/27) мозок \"забуває\" що приносить радість. \
                Заплануй на сьогодні: 1 дію задоволення (Pleasure) — \
                щось приємне без мети (кава, музика, прогулянка), \
                і 1 дію майстерності (Mastery) — маленьке досягнення \
                (прибрати стіл, закрити 1 задачу, приготувати їжу). \
                Комбінація P+M — основа поведінкової активації в КПТ.",
                m.phq9_score
            ),
            category: "mood".into(),
            duration_minutes: Some(15),
        }));
    }

    // Anti-rumination walk for moderate depression
    if m.phq9_score >= 10.0 && m.who5_score < 55.0 {
        c.push((79, PlanItem {
            id: "anti_rumination_walk".into(),
            title: "Антируминаційна прогулянка".into(),
            description:
                "Руминація (прокручування негативних думок) — головний підтримувач депресії. \
                Техніка: вийди на 15-хв прогулянку і СВІДОМО фокусуйся на зовнішньому: \
                рахуй червоні машини, помічай форми хмар, слухай звуки. \
                Кожного разу коли думки повертаються всередину — м'яко \
                перенаправляй увагу назовні. Це розриває цикл руминації."
                    .into(),
            category: "mood".into(),
            duration_minutes: Some(15),
        }));
    }

    // Accomplishment journaling
    if m.phq9_score >= 7.0 && m.phq9_score < 15.0 {
        c.push((60, PlanItem {
            id: "accomplishment_journal".into(),
            title: "Журнал досягнень".into(),
            description:
                "При зниженому настрої мозок ігнорує позитивне. Щовечора запиши: \
                1) Що я зробив сьогодні (навіть дрібниці: встав, поїв, відповів на лист), \
                2) Що було складно але я впорався, \
                3) Що хочу зробити завтра (1 маленька річ). \
                Через тиждень перечитай — побачиш прогрес якого не помічав."
                    .into(),
            category: "mood".into(),
            duration_minutes: Some(5),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// WORK-LIFE BALANCE
// ─────────────────────────────────────────────────────────
fn add_balance_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    if m.work_life_balance < 2.5 {
        c.push((92, PlanItem {
            id: "balance_crisis".into(),
            title: "Баланс: червона зона".into(),
            description: format!(
                "Баланс критичний ({:.1}/10). Робота поглинає все. \
                Екстрені заходи: 1) визнач стоп-тайм (напр. 19:00) і НЕ працюй після, \
                2) захисти 1 годину для себе — прогулянка, хобі, нічого не роблення, \
                3) вимкни робочі нотифікації на телефоні після стоп-тайму, \
                4) напиши керівнику про навантаження якщо це продовжується.",
                m.work_life_balance
            ),
            category: "balance".into(),
            duration_minutes: Some(60),
        }));
    } else if m.work_life_balance < 4.0 {
        c.push((82, PlanItem {
            id: "balance_restore".into(),
            title: "Відновлення балансу".into(),
            description: format!(
                "Work-Life Balance ({:.1}/10) вимагає корекції. \
                Візьми 30 хв \"священного часу\" — лише для себе, без почуття провини. \
                Правило: 1 год перед сном = 0 роботи. \
                Запитай себе: яка задача мене найбільше висмоктує? Чи можна її делегувати?",
                m.work_life_balance
            ),
            category: "balance".into(),
            duration_minutes: Some(30),
        }));
    } else if m.work_life_balance < 6.0 {
        c.push((55, PlanItem {
            id: "balance_tune".into(),
            title: "Тонка настройка балансу".into(),
            description: format!(
                "Баланс ({:.1}/10) можна покращити. \
                Спробуй правило \"3-3-3\": 3 год фокусної роботи, \
                3 год помірної, 3 год повністю поза роботою. \
                Увечері — запитай себе: чи був у мене час лише для себе?",
                m.work_life_balance
            ),
            category: "balance".into(),
            duration_minutes: Some(10),
        }));
    } else if m.work_life_balance >= 7.5 {
        c.push((28, PlanItem {
            id: "balance_celebrate".into(),
            title: "Баланс: тримаєш!".into(),
            description: format!(
                "Баланс {:.1}/10 — це рідкість і заслуга! \
                Запиши рецепт: що саме допомагає? (Стоп-тайм? Хобі? Спорт?) \
                Ці знання знадобляться коли буде складно.",
                m.work_life_balance
            ),
            category: "balance".into(),
            duration_minutes: Some(5),
        }));
    }

    // Boundary communication script
    if m.work_life_balance < 3.5 && m.stress_level >= 16.0 {
        c.push((85, PlanItem {
            id: "boundary_script".into(),
            title: "Скрипт кордонів".into(),
            description: format!(
                "Баланс ({:.1}/10) при стресі ({:.0}/40) = тобі потрібно сказати \"ні\". \
                Готові фрази: \
                \"Я зараз не зможу взяти це — у мене [X] в пріоритеті\", \
                \"Давай повернемось до цього наступного тижня?\", \
                \"Мені потрібно перевірити своє навантаження перед тим як відповісти\". \
                Обери 1 задачу сьогодні і встанови кордон. \"Ні\" — це повне речення.",
                m.work_life_balance, m.stress_level
            ),
            category: "balance".into(),
            duration_minutes: Some(5),
        }));
    }

    // Values alignment check
    if m.work_life_balance < 5.0 && m.mbi_score >= 45.0 {
        c.push((70, PlanItem {
            id: "values_check".into(),
            title: "Перевірка цінностей".into(),
            description: format!(
                "Дисбаланс ({:.1}/10) + вигорання ({:.0}%) часто означає: \
                ти живеш за чужими пріоритетами. \
                Запитай себе: \"Якби я мав лише 3 робочі години на день — \
                що б я зробив?\" Відповідь покаже твої справжні пріоритети. \
                Якщо реальний день не схожий на цю відповідь — час щось змінити.",
                m.work_life_balance, m.mbi_score
            ),
            category: "balance".into(),
            duration_minutes: Some(10),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// MOOD & WELL-BEING (WHO-5)
// ─────────────────────────────────────────────────────────
fn add_mood_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    if m.who5_score < 30.0 {
        c.push((93, PlanItem {
            id: "wellbeing_crisis".into(),
            title: "Благополуччя: увага".into(),
            description: format!(
                "WHO-5 = {:.0}/100. Це сигнал що потрібна підтримка. \
                Прямо зараз: 1) подбай про базове — поїж, випий води, впусти повітря, \
                2) зроби 1 дуже маленьку приємну річ (улюблена пісня, теплий напій), \
                3) поговори з кимось. Якщо це стан тривалий — зверніся до спеціаліста. \
                Просити допомогу — це ознака сили, не слабкості.",
                m.who5_score
            ),
            category: "mood".into(),
            duration_minutes: Some(15),
        }));
    } else if m.who5_score < 50.0 {
        c.push((85, PlanItem {
            id: "wellbeing_low".into(),
            title: "Підтримка настрою".into(),
            description: format!(
                "Благополуччя знижене ({:.0}/100). \
                Сьогодні зроби щось ЛИШЕ для себе: \
                прогулянка на свіжому повітрі (навіть 10 хв), \
                улюблена музика, розмова з другом. \
                Запиши 3 речі за які вдячний — це змінює хімію мозку.",
                m.who5_score
            ),
            category: "mood".into(),
            duration_minutes: Some(15),
        }));
    } else if m.who5_score < 65.0 {
        c.push((62, PlanItem {
            id: "wellbeing_moderate".into(),
            title: "Емоційний заряд".into(),
            description: format!(
                "WHO-5 ({:.0}/100) — є куди рости. Дослідження показують: \
                20 хв на природі знижують кортизол на 20%. \
                Вийди сьогодні на вулицю без навушників — послухай звуки навколо. \
                Або зроби 1 творчу дію: малюнок, нотатка, фото чогось гарного.",
                m.who5_score
            ),
            category: "mood".into(),
            duration_minutes: Some(20),
        }));
    } else if m.who5_score >= 80.0 {
        c.push((22, PlanItem {
            id: "wellbeing_great".into(),
            title: "Настрій: сяйво".into(),
            description: format!(
                "Благополуччя {:.0}/100 — це чудово! \
                Порада: поділись позитивом — напиши комусь щось приємне. \
                Дослідження: допомога іншим підвищує власний рівень щастя на 30%.",
                m.who5_score
            ),
            category: "mood".into(),
            duration_minutes: Some(5),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// FOCUS & COGNITIVE LOAD
// ─────────────────────────────────────────────────────────
fn add_focus_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics, goals: &GoalSettings) {
    let good_sleep = m.sleep_duration >= goals.sleep_target as f64;
    let low_stress = m.stress_level < 16.0;
    let moderate_stress = m.stress_level < 24.0;

    if good_sleep && low_stress && m.who5_score >= 60.0 {
        c.push((68, PlanItem {
            id: "deep_work_ideal".into(),
            title: "Ідеальний день для фокусу".into(),
            description:
                "Сон, стрес і настрій — все в нормі. Це рідкісна комбінація! \
                Обери найважливішу задачу і заблокуй 60-90 хв deep work: \
                телефон у режим \"не турбувати\", закрий всі вкладки окрім 1, \
                жодних мітингів. Після — 15 хв повна пауза (прогулянка, розтяжка)."
                    .into(),
            category: "focus".into(),
            duration_minutes: Some(90),
        }));
    } else if good_sleep && moderate_stress {
        c.push((58, PlanItem {
            id: "pomodoro_day".into(),
            title: "Pomodoro-фокус".into(),
            description:
                "Є ресурс для продуктивності. Використай Pomodoro: \
                25 хв роботи → 5 хв пауза → повторити 3-4 рази. \
                Під час паузи: встань, потягнися, попий води. \
                Після 4 циклів — довша пауза 15-20 хв."
                    .into(),
            category: "focus".into(),
            duration_minutes: Some(30),
        }));
    } else if m.stress_level >= 20.0 && m.work_life_balance < 5.0 {
        c.push((76, PlanItem {
            id: "cognitive_overload".into(),
            title: "Когнітивне перевантаження".into(),
            description: format!(
                "Стрес ({:.0}/40) + низький баланс ({:.1}/10) = мозок перегрітий. \
                Зроби \"brain dump\": витягни ВСЕ з голови на папір за 5 хв. \
                Потім обери лише 3 задачі на сьогодні. Решту — свідомо відклади. \
                Мультитаскінг знижує продуктивність на 40%.",
                m.stress_level, m.work_life_balance
            ),
            category: "focus".into(),
            duration_minutes: Some(10),
        }));
    }

    // Digital detox when overstimulated
    if m.stress_level >= 16.0 && m.who5_score < 60.0 {
        c.push((64, PlanItem {
            id: "digital_detox".into(),
            title: "Цифрова пауза".into(),
            description:
                "Інформаційне перенавантаження підвищує тривогу. \
                Виділи 30 хв без будь-яких екранів: без телефону, ноутбука, ТВ. \
                Замість цього: паперова книга, прогулянка, розмова, \
                або просто тиша. Мозку потрібна \"перезавантаження\" без стимулів."
                    .into(),
            category: "focus".into(),
            duration_minutes: Some(30),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// ENERGY MANAGEMENT
// ─────────────────────────────────────────────────────────
fn add_energy_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    // Low WHO-5 often correlates with low energy
    let low_energy = m.who5_score < 50.0 && m.phq9_score >= 8.0;
    let very_low_energy = m.who5_score < 35.0 && m.phq9_score >= 12.0;

    if very_low_energy {
        c.push((87, PlanItem {
            id: "energy_critical".into(),
            title: "Енергія: базові потреби".into(),
            description:
                "Енергія дуже низька. Фокус на базовому: \
                1) Їжа: з'їж щось поживне прямо зараз (білок + складні вуглеводи), \
                2) Вода: 2 склянки води, \
                3) Повітря: відкрий вікно або вийди на 5 хв, \
                4) Рух: 5 хв легкої розтяжки. \
                Не вимагай від себе багато сьогодні — відновлення це теж робота."
                    .into(),
            category: "energy".into(),
            duration_minutes: Some(10),
        }));
    } else if low_energy {
        c.push((72, PlanItem {
            id: "energy_boost".into(),
            title: "Підйом енергії".into(),
            description:
                "Енергія знижена. Швидкий буст: \
                1) 10 присідань або підйомів на носочки (кров до мозку), \
                2) 2 склянки прохолодної води, \
                3) 5 хв на свіжому повітрі. \
                Уникай солодкого — дасть пік і потім провал. \
                Краще: горіхи, фрукти, чай."
                    .into(),
            category: "energy".into(),
            duration_minutes: Some(10),
        }));
    }

    // Afternoon slump prevention
    if m.sleep_duration < 7.0 && m.stress_level >= 12.0 {
        c.push((52, PlanItem {
            id: "afternoon_slump".into(),
            title: "Антиспад після обіду".into(),
            description:
                "При дефіциті сну та стресі після 14:00 буде провал енергії. \
                Підготуйся: о 13:30-14:00 зроби 5-хвилинну прогулянку, \
                випий води, з'їж легкий перекус (горіхи, банан). \
                Уникай важкого обіду — він посилить сонливість."
                    .into(),
            category: "energy".into(),
            duration_minutes: Some(10),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// COMBINED MULTI-METRIC PATTERNS
// ─────────────────────────────────────────────────────────
fn add_combined_patterns(c: &mut Vec<(i32, PlanItem)>, m: &Metrics, goals: &GoalSettings) {
    // High stress + poor sleep = burnout trajectory
    if m.stress_level >= 20.0 && m.sleep_duration < 6.0 {
        c.push((97, PlanItem {
            id: "burnout_trajectory".into(),
            title: "Профілактика вигорання".into(),
            description: format!(
                "Стрес ({:.0}/40) + дефіцит сну ({:.1}h) = прямий шлях до вигорання. \
                Екстрений план: 1) жодної роботи після 19:00 сьогодні, \
                2) лягти мінімум на 1 год раніше, \
                3) делегувати або відкласти 1 задачу, \
                4) завтра вранці — оцінити чи стан покращився.",
                m.stress_level, m.sleep_duration
            ),
            category: "recovery".into(),
            duration_minutes: Some(15),
        }));
    }

    // Low mood + high anxiety = anxious-depressive state
    if m.who5_score < 50.0 && m.gad7_score >= 10.0 {
        c.push((91, PlanItem {
            id: "anxious_depressive".into(),
            title: "Тривога + настрій".into(),
            description: format!(
                "Низький настрій ({:.0}/100) і тривожність ({:.0}/21) підсилюють одне одного. \
                Техніка \"Лист собі\": напиши собі як другу. Що б ти порадив? \
                Потім: 10 хв ходьби (ритмічний рух знижує і тривогу, і пригніченість). \
                Якщо цей стан тримається більше тижня — варто поговорити з психологом.",
                m.who5_score, m.gad7_score
            ),
            category: "mood".into(),
            duration_minutes: Some(15),
        }));
    }

    // Low mood + low energy = emotional exhaustion
    if m.who5_score < 45.0 && m.phq9_score >= 12.0 && m.mbi_score < 60.0 {
        c.push((89, PlanItem {
            id: "emotional_exhaustion".into(),
            title: "Емоційне виснаження".into(),
            description:
                "Настрій та енергія на мінімумі. Не вимагай від себе подвигів. \
                Сьогодні дозволь собі: 1) поговорити з близькою людиною (10 хв), \
                2) щось фізичне (прогулянка 15 хв — навіть якщо не хочеться), \
                3) подбати про тіло (їжа, вода, свіже повітря). \
                Маленькі дії накопичуються."
                    .into(),
            category: "mood".into(),
            duration_minutes: Some(25),
        }));
    }

    // High workload + high burnout = overload
    if m.work_life_balance < 4.0 && m.mbi_score >= 55.0 {
        c.push((86, PlanItem {
            id: "overload_signal".into(),
            title: "Перевантаження".into(),
            description: format!(
                "Низький баланс ({:.1}/10) при вигоранні {:.0}%. \
                Потрібна інвентаризація: 1) запиши ВСІ задачі, \
                2) відміть 1 яку можна делегувати, \
                3) відміть 1 яку можна відкласти на тиждень, \
                4) обговори з керівником пріоритети. \
                Працювати більше ≠ зробити більше.",
                m.work_life_balance, m.mbi_score
            ),
            category: "balance".into(),
            duration_minutes: Some(15),
        }));
    }

    // Stress + anxiety = hyperarousal
    if m.stress_level >= 20.0 && m.gad7_score >= 10.0 {
        c.push((90, PlanItem {
            id: "hyperarousal".into(),
            title: "Нервова система перезбуджена".into(),
            description: format!(
                "Стрес ({:.0}/40) + тривога ({:.0}/21) = нервова система \"на взводі\". \
                Вагусна стимуляція за 2 хв: 1) довгий повільний видих (6-8 сек), \
                2) умийся холодною водою, 3) затримай дихання на 10 сек. \
                Це активує блукаючий нерв і знижує \"fight-or-flight\" відповідь.",
                m.stress_level, m.gad7_score
            ),
            category: "anxiety".into(),
            duration_minutes: Some(5),
        }));
    }

    // Good sleep + high stress = recovery potential
    if m.sleep_duration >= goals.sleep_target as f64 && m.stress_level >= 20.0 {
        c.push((74, PlanItem {
            id: "recovery_potential".into(),
            title: "Потенціал відновлення".into(),
            description: format!(
                "Сон в нормі ({:.1}h) — тіло відновлюється, але стрес ({:.0}/40) високий. \
                Сьогодні сфокусуйся на зниженні стресу: \
                замість надмірної продуктивності — якісні паузи. \
                Через кожні 45 хв роботи — 10 хв повного відключення.",
                m.sleep_duration, m.stress_level
            ),
            category: "recovery".into(),
            duration_minutes: Some(10),
        }));
    }

    // Sleep + anxiety = racing thoughts at night
    if m.sleep_duration < 6.5 && m.gad7_score >= 8.0 {
        c.push((88, PlanItem {
            id: "racing_thoughts_sleep".into(),
            title: "Нічні нав'язливі думки".into(),
            description: format!(
                "Тривога ({:.0}/21) + дефіцит сну ({:.1}h) = думки не дають заснути. \
                Техніка \"відкладених тривог\": за 2 год до сну запиши всі тривоги \
                на аркуш + для кожної: \"розберуся завтра о [конкретний час]\". \
                Поклади аркуш далеко від ліжка. Якщо думка повертається — \
                скажи собі: \"це вже записано, мозок може відпочити\".",
                m.gad7_score, m.sleep_duration
            ),
            category: "sleep".into(),
            duration_minutes: Some(10),
        }));
    }

    // Depression + poor balance = work-driven emptiness
    if m.phq9_score >= 8.0 && m.work_life_balance < 4.0 && m.mbi_score >= 40.0 {
        c.push((87, PlanItem {
            id: "work_driven_emptiness".into(),
            title: "Робота без сенсу".into(),
            description: format!(
                "Знижений настрій ({:.0}/27) + дисбаланс ({:.1}/10) + вигорання ({:.0}%) = \
                робота поглинає, але не наповнює. \
                Сьогодні: 1) Згадай момент коли робота приносила задоволення — що тоді було інакше? \
                2) Визнач 1 задачу яка відповідає твоїм цінностям і зроби її ПЕРШОЮ. \
                3) Після роботи — 30 хв на щось поза роботою що має для тебе значення.",
                m.phq9_score, m.work_life_balance, m.mbi_score
            ),
            category: "recovery".into(),
            duration_minutes: Some(30),
        }));
    }

    // Good metrics = optimization mode
    if m.who5_score >= 65.0 && m.stress_level < 16.0 && m.sleep_duration >= 7.0
        && m.gad7_score < 7.0 && m.phq9_score < 5.0
    {
        c.push((45, PlanItem {
            id: "optimization_mode".into(),
            title: "Режим оптимізації".into(),
            description: format!(
                "Сон ({:.1}h), настрій ({:.0}), стрес ({:.0}) — все в нормі! \
                Рідкісний день для зростання. Спробуй щось нове: \
                холодний душ 30 сек (вагусна стимуляція), \
                нову фізичну активність (йога, біг, плавання), \
                або 15 хв медитації. В хороші дні формуються звички \
                які тримають в погані.",
                m.sleep_duration, m.who5_score, m.stress_level
            ),
            category: "energy".into(),
            duration_minutes: Some(15),
        }));
    }

    // Stress + depression (без тривоги) = анестезований стрес
    if m.stress_level >= 20.0 && m.phq9_score >= 10.0 && m.gad7_score < 8.0 {
        c.push((88, PlanItem {
            id: "numbed_stress".into(),
            title: "Стрес без тривоги".into(),
            description: format!(
                "Стрес ({:.0}/40) + знижений настрій ({:.0}/27) без вираженої тривоги — \
                це може бути \"анестезована\" реакція: тіло під навантаженням, \
                але емоції пригнічені. Це небезпечніше ніж тривога, бо непомітне. \
                Сьогодні: 1) Перевір тілесні сигнали (головний біль? напруга в спині? \
                стиснуті щелепи?), 2) 10 хв легкої розтяжки з увагою до відчуттів, \
                3) Запиши \"як я НАСПРАВДІ себе почуваю?\"",
                m.stress_level, m.phq9_score
            ),
            category: "recovery".into(),
            duration_minutes: Some(15),
        }));
    }

    // All metrics bad = crisis
    if m.who5_score < 40.0 && m.stress_level >= 24.0 && m.sleep_duration < 5.5 {
        c.push((100, PlanItem {
            id: "multi_crisis".into(),
            title: "Комплексне відновлення".into(),
            description: format!(
                "Сон ({:.1}h), стрес ({:.0}/40) і настрій ({:.0}/100) — \
                все потребує уваги. Пріоритет: ЛИШЕ базові потреби. \
                1) Поїж щось поживне, \
                2) Випий 2 склянки води, \
                3) 5 хв на свіжому повітрі, \
                4) Сьогодні ляг якомога раніше. \
                Не вимагай від себе нічого іншого. Відновлення — це робота.",
                m.sleep_duration, m.stress_level, m.who5_score
            ),
            category: "recovery".into(),
            duration_minutes: Some(10),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// POSITIVE REINFORCEMENT
// ─────────────────────────────────────────────────────────
fn add_positive_reinforcement(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    let good_metrics_count = [
        m.who5_score >= 70.0,
        m.stress_level < 12.0,
        m.sleep_duration >= 7.5,
        m.mbi_score < 30.0,
        m.gad7_score < 5.0,
        m.phq9_score < 5.0,
        m.work_life_balance >= 7.0,
    ]
    .iter()
    .filter(|&&v| v)
    .count();

    if good_metrics_count >= 5 {
        c.push((35, PlanItem {
            id: "all_green".into(),
            title: "Все добре!".into(),
            description: format!(
                "{} з 7 показників у зеленій зоні. Ти піклуєшся про себе — і це видно! \
                Порада: запиши свій \"рецепт гарного дня\" — \
                що ти робив вчора і сьогодні що привело до цього стану? \
                Ці нотатки стануть в пригоді у складні часи.",
                good_metrics_count
            ),
            category: "motivation".into(),
            duration_minutes: Some(5),
        }));
    } else if good_metrics_count >= 3 {
        c.push((30, PlanItem {
            id: "partial_green".into(),
            title: "Є прогрес".into(),
            description: format!(
                "{} з 7 показників у нормі. Є база для покращення решти. \
                Сфокусуйся на найслабшому: \
                {}. \
                Маленький крок сьогодні → великий результат за тиждень.",
                good_metrics_count,
                weakest_area_tip(m)
            ),
            category: "motivation".into(),
            duration_minutes: Some(5),
        }));
    }
}

fn weakest_area_tip(m: &Metrics) -> &'static str {
    let mut worst = ("sleep", 0.0f64);
    let sleep_bad = if m.sleep_duration < 8.0 { (8.0 - m.sleep_duration) / 8.0 } else { 0.0 };
    let stress_bad = m.stress_level / 40.0;
    let mood_bad = (100.0 - m.who5_score) / 100.0;
    let anxiety_bad = m.gad7_score / 21.0;
    let burnout_bad = m.mbi_score / 100.0;
    let balance_bad = (10.0 - m.work_life_balance) / 10.0;
    let depression_bad = m.phq9_score / 27.0;

    if sleep_bad > worst.1 { worst = ("sleep", sleep_bad); }
    if stress_bad > worst.1 { worst = ("stress", stress_bad); }
    if mood_bad > worst.1 { worst = ("mood", mood_bad); }
    if anxiety_bad > worst.1 { worst = ("anxiety", anxiety_bad); }
    if burnout_bad > worst.1 { worst = ("burnout", burnout_bad); }
    if balance_bad > worst.1 { worst = ("balance", balance_bad); }
    if depression_bad > worst.1 { worst = ("depression", depression_bad); }

    match worst.0 {
        "sleep" => "сон — ляг на 30 хв раніше і вимкни екрани за годину до сну",
        "stress" => "стрес — 5 хв box breathing (4-4-4-4) і скасуй 1 необов'язкову справу",
        "mood" => "настрій — вийди на 15 хв прогулянку і зроби 1 приємну дію для себе",
        "anxiety" => "тривога — техніка заземлення 5-4-3-2-1 і запиши тривоги на папір",
        "burnout" => "вигорання — делегуй 1 задачу і захисти 30 хв лише для себе",
        "balance" => "баланс — визнач стоп-тайм роботи і вимкни робочі нотифікації після нього",
        "depression" => "настрій — зроби 1 маленьку приємну річ і поговори з близькою людиною",
        _ => "подбай про себе — ти цього вартий",
    }
}

// ─────────────────────────────────────────────────────────
// UNIVERSAL & DAY-AWARE ITEMS
// ─────────────────────────────────────────────────────────
fn add_universal_items(
    c: &mut Vec<(i32, PlanItem)>,
    metrics: Option<&Metrics>,
    goals: &GoalSettings,
    weekday: u32,
) {
    // Movement — always relevant, priority varies
    let move_priority = match metrics {
        Some(m) if m.stress_level >= 20.0 => 80, // urgent when stressed
        Some(m) if m.who5_score < 50.0 => 75,    // important for mood
        _ => 60,
    };
    let move_desc = match metrics {
        Some(m) if m.stress_level >= 20.0 => format!(
            "При стресі ({:.0}/40) рух — найефективніший антидот. \
            {} хв: швидка ходьба, сходи, розтяжка. \
            Рух знижує кортизол на 20% і вивільняє ендорфіни.",
            m.stress_level, goals.move_target
        ),
        Some(m) if m.who5_score < 50.0 => format!(
            "Рух покращує настрій через вивільнення серотоніну. \
            {} хв будь-якої активності: прогулянка, розтяжка, танці під музику. \
            Навіть 10 хв допоможуть.",
            goals.move_target
        ),
        _ => format!(
            "Рухайся {} хв сьогодні: прогулянка, сходи, розтяжка. \
            Регулярний рух знижує ризик вигорання на 30% і покращує сон.",
            goals.move_target
        ),
    };
    c.push((move_priority, PlanItem {
        id: "movement".into(),
        title: "Рух".into(),
        description: move_desc,
        category: "movement".into(),
        duration_minutes: Some(goals.move_target),
    }));

    // Specific exercise type based on state
    match metrics {
        Some(m) if m.gad7_score >= 10.0 && m.stress_level >= 16.0 => {
            c.push((69, PlanItem {
                id: "exercise_type".into(),
                title: "Ритмічний рух".into(),
                description: format!(
                    "При тривозі ({:.0}/21) + стресі ({:.0}/40) найкраще працює \
                    РИТМІЧНА активність: швидка ходьба, біг, плавання, велосипед. \
                    Ритмічний рух синхронізує півкулі мозку і знижує кортизол \
                    ефективніше ніж хаотичний рух. 20-30 хв помірної інтенсивності \
                    (можеш говорити, але з зусиллям).",
                    m.gad7_score, m.stress_level
                ),
                category: "movement".into(),
                duration_minutes: Some(25),
            }));
        }
        Some(m) if m.phq9_score >= 8.0 && m.who5_score < 55.0 => {
            c.push((66, PlanItem {
                id: "exercise_type".into(),
                title: "Силова активність".into(),
                description:
                    "При зниженому настрої силові вправи ефективніші за кардіо. \
                    Навіть без обладнання: присідання, віджимання, планка. \
                    3 підходи по 10 повторень = 10 хв. \
                    Силове навантаження підвищує тестостерон і BDNF (фактор росту нейронів), \
                    що покращує настрій і самооцінку. Бонус: відчуття контролю над тілом."
                        .into(),
                category: "movement".into(),
                duration_minutes: Some(15),
            }));
        }
        Some(m) if m.sleep_duration < 6.0 || m.mbi_score >= 60.0 => {
            c.push((58, PlanItem {
                id: "exercise_type".into(),
                title: "М'яка розтяжка".into(),
                description:
                    "При виснаженні інтенсивне тренування зашкодить. \
                    Сьогодні: 15 хв м'якої розтяжки або йоги. \
                    Фокус на: розтяжка стегон (поза голуба), \
                    розкриття грудної клітки (руки за спиною), \
                    нахил вперед (розтяжка задньої поверхні ніг). \
                    Повільне дихання під час розтяжки = подвійний ефект розслаблення."
                        .into(),
                category: "movement".into(),
                duration_minutes: Some(15),
            }));
        }
        _ => {}
    }

    // Hydration
    c.push((48, PlanItem {
        id: "hydrate".into(),
        title: "Вода".into(),
        description:
            "Випий 2 склянки води прямо зараз. \
            Зневоднення навіть на 1-2% знижує когнітивні функції на 15%. \
            Порада: постав пляшку води на робочий стіл як нагадування."
                .into(),
        category: "recovery".into(),
        duration_minutes: Some(3),
    }));

    // Day-specific recommendations
    match weekday {
        0 => {
            // Monday — week planning
            c.push((56, PlanItem {
                id: "monday_planning".into(),
                title: "Планування тижня".into(),
                description:
                    "Понеділок — час визначити пріоритети. \
                    Запиши 3 головні цілі на тиждень. \
                    Для кожної — 1 конкретний перший крок. \
                    Це знижує тривогу і дає відчуття контролю."
                        .into(),
                category: "focus".into(),
                duration_minutes: Some(10),
            }));
        }
        2 => {
            // Wednesday — midweek check
            c.push((45, PlanItem {
                id: "midweek_check".into(),
                title: "Середина тижня".into(),
                description:
                    "Середа — час для перевірки. Подивись на 3 цілі тижня: \
                    чи є прогрес? Чи потрібно щось змінити? \
                    Якщо ти позаду — це нормально. Скоригуй і рухайся далі. \
                    Дай собі 5 хв тиші для рефлексії."
                        .into(),
                category: "focus".into(),
                duration_minutes: Some(5),
            }));
        }
        4 => {
            // Friday — week closure
            c.push((55, PlanItem {
                id: "friday_closure".into(),
                title: "Закриття тижня".into(),
                description:
                    "П'ятниця — час завершити тиждень правильно. \
                    1) Запиши 3 досягнення цього тижня (навіть маленькі), \
                    2) Зафіксуй що залишилось на наступний тиждень, \
                    3) \"Закрий\" робочий мозок — дозволь собі вихідні."
                        .into(),
                category: "balance".into(),
                duration_minutes: Some(10),
            }));
        }
        5 | 6 => {
            // Weekend — recovery & joy
            c.push((55, PlanItem {
                id: "weekend_recovery".into(),
                title: "Відновлення на вихідних".into(),
                description:
                    "Вихідні — час для справжнього відновлення. \
                    Не плануй багато. Зроби 1 річ для тіла (спорт, прогулянка) \
                    і 1 для душі (хобі, друзі, природа). \
                    Правило: мінімум роботи, максимум того що наповнює."
                        .into(),
                category: "balance".into(),
                duration_minutes: Some(30),
            }));
        }
        1 => {
            // Tuesday — energy management
            c.push((46, PlanItem {
                id: "tuesday_energy".into(),
                title: "Вівторок: енергоменеджмент".into(),
                description:
                    "Вівторок — зазвичай найпродуктивніший день тижня. \
                    Використай це: постав найскладнішу задачу на першу половину дня. \
                    Після обіду — рутинні справи. Захисти свою пікову енергію \
                    від непотрібних мітингів і дрібних задач."
                        .into(),
                category: "focus".into(),
                duration_minutes: Some(5),
            }));
        }
        3 => {
            // Thursday — social + reflection
            c.push((46, PlanItem {
                id: "thursday_connect".into(),
                title: "Четвер: зв'язки".into(),
                description:
                    "Четвер — гарний день для соціальних зв'язків. \
                    Напиши колезі подяку за щось конкретне, \
                    або запропонуй спільний обід. Соціальна підтримка \
                    на роботі знижує ризик вигорання на 40%. \
                    Також: рефлексія — чого ти навчився цього тижня?"
                        .into(),
                category: "social".into(),
                duration_minutes: Some(10),
            }));
        }
        _ => {}
    }

    // Gratitude — personalized based on state
    let gratitude_desc = match metrics {
        Some(m) if m.who5_score < 50.0 =>
            "Коли настрій низький, практика вдячності особливо важлива. \
            Запиши 3 КОНКРЕТНІ речі за які вдячний (не абстрактні). \
            Наприклад: \"тепла кава вранці\", \"повідомлення від друга\". \
            Дослідження: 21 день практики змінює нейронні зв'язки мозку."
                .to_string(),
        Some(m) if m.stress_level >= 16.0 =>
            "При стресі мозок фокусується на загрозах. Практика вдячності \
            перемикає його. Запиши 3 хороші речі сьогодні — \
            це знижує кортизол і покращує сон."
                .to_string(),
        _ =>
            "Запиши 3 речі за які вдячний сьогодні. \
            Регулярна практика вдячності знижує рівень кортизолу на 23% \
            і покращує якість сну."
                .to_string(),
    };
    c.push((42, PlanItem {
        id: "gratitude".into(),
        title: "Вдячність".into(),
        description: gratitude_desc,
        category: "mindfulness".into(),
        duration_minutes: Some(5),
    }));

    // Social connection — personalized
    let social_desc = match metrics {
        Some(m) if m.who5_score < 45.0 || m.phq9_score >= 10.0 =>
            "Ізоляція погіршує настрій. Навіть 5 хв розмови з близькою людиною \
            вивільняють окситоцин і покращують самопочуття. \
            Напиши або подзвони комусь прямо зараз. Не чекай \"кращого моменту\"."
                .to_string(),
        _ =>
            "Напиши або подзвони комусь хто тобі небайдужий. \
            Соціальні зв'язки — найсильніший предиктор благополуччя. \
            5 хв теплої розмови покращують настрій на годину."
                .to_string(),
    };
    c.push((40, PlanItem {
        id: "social_connect".into(),
        title: "Соціальний контакт".into(),
        description: social_desc,
        category: "social".into(),
        duration_minutes: Some(5),
    }));

    // Mindful breathing — always available, priority varies
    let breathing_priority = match metrics {
        Some(m) if m.gad7_score >= 10.0 || m.stress_level >= 20.0 => 70,
        _ => 38,
    };
    let breathing_desc = match metrics {
        Some(m) if m.gad7_score >= 10.0 =>
            "При тривожності ({:.0}/21) дихання — найшвидший інструмент. \
            Вправа \"4-7-8\": вдих 4 сек → затримка 7 сек → видих 8 сек. \
            3 цикли. Це активує парасимпатику і знижує тривогу за 60 секунд."
                .to_string(),
        Some(m) if m.stress_level >= 20.0 => format!(
            "При стресі ({:.0}/40) дихання знижує пульс за 90 секунд. \
            Box breathing: вдих 4с → затримка 4с → видих 4с → затримка 4с. \
            5 циклів. Використовують навіть Navy SEALs.",
            m.stress_level
        ),
        _ =>
            "3 хвилини свідомого дихання: повільний вдих на 4 рахунки, \
            повільний видих на 6. Це знижує кортизол і покращує фокус. \
            Можна робити прямо за робочим столом."
                .to_string(),
    };
    c.push((breathing_priority, PlanItem {
        id: "mindful_breathing".into(),
        title: "Дихальна вправа".into(),
        description: breathing_desc,
        category: "mindfulness".into(),
        duration_minutes: Some(5),
    }));

    // Nature prescription — calibrated to mood
    let nature_priority = match metrics {
        Some(m) if m.who5_score < 45.0 => 74,
        Some(m) if m.stress_level >= 20.0 => 70,
        _ => 42,
    };
    let nature_desc = match metrics {
        Some(m) if m.who5_score < 45.0 => format!(
            "\"Зелена рецептура\": при низькому настрої ({:.0}/100) \
            20 хв серед зелені знижують кортизол на 20% і покращують настрій на 3 год. \
            Не потрібен ліс — парк, двір з деревами, навіть балкон з рослинами. \
            Ключ: без телефону, з увагою до природи (запахи, звуки, кольори).",
            m.who5_score
        ),
        Some(m) if m.stress_level >= 20.0 => format!(
            "Природа знижує стрес ({:.0}/40) ефективніше ніж медитація в приміщенні. \
            \"Синрін-йоку\" (лісове купання): 15-20 хв серед дерев, повільна ходьба, \
            глибоке дихання. Навіть міський парк працює. Без навушників.",
            m.stress_level
        ),
        _ =>
            "Вийди на 15 хв на свіже повітря сьогодні. Дослідження: 120 хв на природі \
            на тиждень (17 хв/день) оптимальні для здоров'я і благополуччя. \
            Без телефону — просто будь присутнім."
                .to_string(),
    };
    c.push((nature_priority, PlanItem {
        id: "nature_rx".into(),
        title: "Природа як ліки".into(),
        description: nature_desc,
        category: "movement".into(),
        duration_minutes: Some(20),
    }));

    // Micro-habits — tiny stackable habits
    if metrics.map(|m| m.mbi_score >= 40.0 || m.phq9_score >= 8.0).unwrap_or(false) {
        c.push((52, PlanItem {
            id: "micro_habits".into(),
            title: "Мікро-звички".into(),
            description:
                "Коли енергії мало, великі зміни не працюють. Стратегія мікро-звичок: \
                прив'яжи нову дію до існуючої. Приклади: \
                \"Після того як налью каву → 3 глибоких вдихи\", \
                \"Після того як сяду за комп'ютер → запишу 1 пріоритет\", \
                \"Після того як вимию руки → розтяжка шиї 10 сек\". \
                Обери 1 мікро-звичку на сьогодні. Через 21 день це стане автоматичним."
                    .into(),
            category: "motivation".into(),
            duration_minutes: Some(2),
        }));
    }

    // Evening wind-down
    if metrics.map(|m| m.sleep_duration < 7.0 || m.stress_level >= 16.0).unwrap_or(false) {
        c.push((62, PlanItem {
            id: "evening_routine".into(),
            title: "Вечірній ритуал".into(),
            description:
                "За 60 хв до сну почни \"вечірню деактивацію\": \
                1) Запиши 3 думки що крутяться в голові (brain dump), \
                2) Вимкни яскраве світло (використай тепле/жовте), \
                3) Без телефону/ноутбука — книга або спокійна музика. \
                Ритуал сигналізує мозку: час спати."
                    .into(),
            category: "sleep".into(),
            duration_minutes: Some(15),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// SLEEP QUALITY (separate from duration)
// ─────────────────────────────────────────────────────────
fn add_sleep_quality_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    let sq = m.sleep_quality();
    // sleep_quality falls back to duration, so only add specific items when we have real quality data
    // or when there's a notable discrepancy between duration and quality
    if m.sleep_quality.is_some() {
        let quality = m.sleep_quality.unwrap();
        if quality < 4.0 && m.sleep_duration >= 6.0 {
            // Long sleep but poor quality
            c.push((85, PlanItem {
                id: "sleep_quality_poor".into(),
                title: "Якість сну: увага".into(),
                description: format!(
                    "Ти спиш достатньо ({:.1}h), але якість сну низька ({:.1}/10). \
                    Можливі причини: часті пробудження, апное, стрес. \
                    Перевір: 1) Температура спальні 18-20°C? \
                    2) Чи не п'єш алкоголь перед сном (руйнує REM-фазу)? \
                    3) Чи є шум/світло? (беруші + маска для сну можуть змінити все). \
                    4) Якщо хропеш — зверніся до лікаря щодо апное.",
                    m.sleep_duration, quality
                ),
                category: "sleep".into(),
                duration_minutes: Some(10),
            }));
        } else if quality < 5.5 {
            c.push((68, PlanItem {
                id: "sleep_quality_mediocre".into(),
                title: "Покращення якості сну".into(),
                description: format!(
                    "Якість сну ({:.1}/10) можна покращити. \
                    Три ключі до глибокого сну: \
                    1) Регулярність — лягай і вставай в один час (±30 хв), \
                    2) Прохолода — зниж температуру в спальні, \
                    3) Темрява — жодного світла (навіть LED індикаторів). \
                    Найважливіше: стоп-екрани за 60 хв до сну (синє світло \
                    блокує мелатонін на 90 хв).",
                    quality
                ),
                category: "sleep".into(),
                duration_minutes: Some(10),
            }));
        } else if quality >= 8.0 {
            c.push((20, PlanItem {
                id: "sleep_quality_great".into(),
                title: "Якість сну: відмінно".into(),
                description: format!(
                    "Якість сну {:.1}/10 — ти робиш все правильно! \
                    Твій організм якісно відновлюється. \
                    Запиши свій вечірній ритуал — це твій рецепт.",
                    quality
                ),
                category: "sleep".into(),
                duration_minutes: Some(3),
            }));
        }
    }

    // Morning sunlight when sleep is suboptimal
    if sq < 6.0 || m.sleep_duration < 6.5 {
        c.push((65, PlanItem {
            id: "morning_sunlight".into(),
            title: "Ранкове сонце".into(),
            description:
                "Перших 30 хв після пробудження вийди на сонячне світло (або яскраве штучне). \
                10-15 хв сонця вранці: 1) зсувають циркадний ритм для кращого засинання ввечері, \
                2) підвищують кортизол вранці (що нормально і корисно), \
                3) збільшують вироблення серотоніну вдень і мелатоніну вночі. \
                Це найпотужніший безкоштовний інструмент покращення сну."
                    .into(),
            category: "sleep".into(),
            duration_minutes: Some(15),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// MORNING ACTIVATION ROUTINE
// ─────────────────────────────────────────────────────────
fn add_morning_routine(c: &mut Vec<(i32, PlanItem)>, m: &Metrics, goals: &GoalSettings) {
    let poor_recovery = m.sleep_duration < 6.0 || m.who5_score < 45.0;
    let moderate_state = m.sleep_duration >= 6.0 && m.who5_score >= 45.0 && m.stress_level < 24.0;

    if poor_recovery {
        c.push((76, PlanItem {
            id: "morning_gentle".into(),
            title: "М'який ранковий старт".into(),
            description: format!(
                "Після поганого сну ({:.1}h) або при низькому настрої ({:.0}/100) \
                не кидайся одразу в роботу. Ранковий протокол відновлення (15 хв): \
                1) Склянка теплої води з лимоном, \
                2) 3 хв повільного дихання (вдих 4 — видих 6), \
                3) 5 хв легкої розтяжки (шия, плечі, спина), \
                4) Запиши 1 намір на день (не задачу, а намір: \"бути терплячим\", \"берегти енергію\"). \
                Повільний старт = стабільніший день.",
                m.sleep_duration, m.who5_score
            ),
            category: "energy".into(),
            duration_minutes: Some(15),
        }));
    } else if moderate_state {
        c.push((50, PlanItem {
            id: "morning_activate".into(),
            title: "Ранкова активація".into(),
            description: format!(
                "Стан у нормі — використай ранок для заряду. Протокол (10 хв): \
                1) Холодне вмивання обличчя (активація вагусного нерва), \
                2) {} присідань або віджимань (запуск кровообігу), \
                3) Склянка води, \
                4) 2 хв візуалізації: уяви що сьогоднішній день пройшов ідеально — \
                що ти зробив? Як почувався? Це програмує мозок на успіх.",
                goals.move_target.min(15)
            ),
            category: "energy".into(),
            duration_minutes: Some(10),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// NUTRITION
// ─────────────────────────────────────────────────────────
fn add_nutrition_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    // Stress-specific nutrition
    if m.stress_level >= 20.0 {
        c.push((66, PlanItem {
            id: "nutrition_stress".into(),
            title: "Їжа проти стресу".into(),
            description: format!(
                "При стресі ({:.0}/40) організм витрачає більше магнію і вітаміну B. \
                Антистрес-їжа сьогодні: \
                темний шоколад (70%+, 1-2 квадратики — магній + ендорфіни), \
                банан (триптофан → серотонін), горіхи (омега-3, магній), \
                зелений чай (L-теанін = спокій без сонливості). \
                Уникай: цукор, фастфуд, надмір кофеїну (макс 2 чашки до 14:00).",
                m.stress_level
            ),
            category: "nutrition".into(),
            duration_minutes: Some(5),
        }));
    }

    // Depression-specific nutrition
    if m.phq9_score >= 8.0 {
        c.push((63, PlanItem {
            id: "nutrition_mood".into(),
            title: "Їжа для настрою".into(),
            description:
                "Серотонін на 95% виробляється в кишечнику. Для його підтримки: \
                жирна риба (лосось, скумбрія — омега-3 знижує запалення мозку), \
                ферментовані продукти (кефір, квашена капуста — мікробіом), \
                яйця (холін + вітамін D), листова зелень (фолат). \
                Дослідження SMILES: середземноморська дієта за 12 тижнів \
                зменшила симптоми депресії у 32% учасників."
                    .into(),
            category: "nutrition".into(),
            duration_minutes: Some(5),
        }));
    }

    // Low energy nutrition
    if m.who5_score < 50.0 && m.sleep_duration < 7.0 {
        c.push((60, PlanItem {
            id: "nutrition_energy".into(),
            title: "Їжа для енергії".into(),
            description: format!(
                "При втомі ({:.0}/100 WHO-5) уникай \"швидких фіксів\" (цукор, енергетики). \
                Стабільна енергія: вівсянка/каша (повільні вуглеводи), \
                білок на сніданок (яйця, сир — тримає рівень цукру), \
                горіхи на перекус (не чіпси/печиво). \
                Маленькі прийоми їжі кожні 3-4 год замість 2 великих. \
                Зневоднення імітує втому — мінімум 8 склянок води.",
                m.who5_score
            ),
            category: "nutrition".into(),
            duration_minutes: Some(5),
        }));
    }

    // Anxiety-specific nutrition
    if m.gad7_score >= 10.0 {
        c.push((58, PlanItem {
            id: "nutrition_anxiety".into(),
            title: "Їжа проти тривоги".into(),
            description: format!(
                "Тривога ({:.0}/21) реагує на харчування. \
                Зниж: кофеїн (макс 1 чашка вранці), цукор, алкоголь, ультраперероблену їжу. \
                Додай: магній (гарбузове насіння, мигдаль, шпинат), \
                L-теанін (зелений чай), пробіотики (кефір, йогурт). \
                Кишечник-мозок зв'язок: 70% імунної системи і більшість \
                нейромедіаторів спокою виробляються в кишечнику.",
                m.gad7_score
            ),
            category: "nutrition".into(),
            duration_minutes: Some(5),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// BODY CARE
// ─────────────────────────────────────────────────────────
fn add_body_care_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    // Posture + tension when stressed
    if m.stress_level >= 16.0 {
        c.push((59, PlanItem {
            id: "posture_check".into(),
            title: "Постура і затиски".into(),
            description: format!(
                "При стресі ({:.0}/40) тіло затискається непомітно. \
                Скан тіла прямо зараз (60 сек): \
                Чоло нахмурене? → розслаб. \
                Щелепи стиснуті? → відкрий рот на 2 см, подихай. \
                Плечі підняті? → опусти, зроби 3 кругових рухи. \
                Живіт затиснутий? → поклади руку, подихай животом. \
                Повторюй цей скан кожні 2 години — це запобігає головним болям і болю в спині.",
                m.stress_level
            ),
            category: "body".into(),
            duration_minutes: Some(3),
        }));
    }

    // Eye care for screen workers
    if m.work_life_balance < 6.0 || m.stress_level >= 12.0 {
        c.push((44, PlanItem {
            id: "eye_care_2020".into(),
            title: "Правило 20-20-20".into(),
            description:
                "Кожні 20 хв роботи за екраном: дивись 20 секунд \
                на об'єкт на відстані 20 футів (6 метрів). \
                Додатково: 1) кожну годину закривай очі на 30 сек, \
                2) свідомо моргай (при екрані ми моргаємо в 3 рази рідше), \
                3) ввечері увімкни нічний режим (тепле світло) на всіх пристроях."
                    .into(),
            category: "body".into(),
            duration_minutes: Some(2),
        }));
    }

    // Neck and shoulder routine when burnout indicators present
    if m.mbi_score >= 45.0 || (m.stress_level >= 20.0 && m.sleep_duration < 7.0) {
        c.push((62, PlanItem {
            id: "neck_shoulders".into(),
            title: "Розтяжка шиї і плечей".into(),
            description:
                "Стрес накопичується в шиї і плечах. Мікрорутина (3 хв): \
                1) Нахили голови вліво-вправо (затримка 15 сек кожний), \
                2) Підборіддя до грудей, потім потилицю назад (5 разів), \
                3) Кругові рухи плечима вперед і назад (по 10), \
                4) Зведи лопатки разом, затримай 5 сек (5 разів). \
                Роби цю рутину після кожної години сидячої роботи."
                    .into(),
            category: "body".into(),
            duration_minutes: Some(3),
        }));
    }

    // Cold exposure for vagal tone
    if m.gad7_score >= 8.0 || (m.stress_level >= 20.0 && m.who5_score < 60.0) {
        c.push((56, PlanItem {
            id: "cold_exposure".into(),
            title: "Холодна стимуляція".into(),
            description: format!(
                "Вагусна стимуляція холодом — швидкий спосіб знизити тривогу \
                і стрес. Обери 1 варіант: \
                1) Умий обличчя холодною водою (30 сек — \"dive reflex\"), \
                2) Холодний компрес на задню частину шиї (2 хв), \
                3) Холодний душ останні 30 сек (просунутий рівень). \
                Холод активує парасимпатику і знижує пульс за 60 сек. \
                Протипоказання: серцево-судинні захворювання."
            ),
            category: "body".into(),
            duration_minutes: Some(3),
        }));
    }
}

// ─────────────────────────────────────────────────────────
// SELF-COMPASSION (for depression + burnout overlap)
// ─────────────────────────────────────────────────────────
fn add_self_compassion_items(c: &mut Vec<(i32, PlanItem)>, m: &Metrics) {
    // Perfectionism trap: high burnout but moderate other metrics
    if m.mbi_score >= 55.0 && m.stress_level < 24.0 && m.gad7_score < 12.0 {
        c.push((72, PlanItem {
            id: "perfectionism_release".into(),
            title: "Відпусти перфекціонізм".into(),
            description: format!(
                "Вигорання ({:.0}%) без екстремального стресу — часто ознака перфекціонізму. \
                Вправа \"достатньо добре\": для 3 задач сьогодні свідомо обери рівень \"80%\" \
                замість \"ідеально\". Запитай: \"Чи зміниться щось суттєве від різниці \
                між 80% і 100%?\" Зазвичай ні. Звільнена енергія = твоє відновлення.",
                m.mbi_score
            ),
            category: "recovery".into(),
            duration_minutes: Some(5),
        }));
    }

    // Inner critic when depression + self-criticism pattern
    if m.phq9_score >= 10.0 && m.who5_score < 50.0 {
        c.push((80, PlanItem {
            id: "inner_critic".into(),
            title: "Внутрішній критик".into(),
            description:
                "Коли настрій низький, внутрішній критик стає голоснішим. \
                Вправа \"друг\": коли ловиш себе на самокритиці — запитай: \
                \"Чи сказав би я це близькому другу в такій ситуації?\" \
                Якщо ні — перефразуй. Замість \"я нікчема\" → \
                \"мені зараз важко, і це нормально\". \
                Самоспівчуття — не слабкість, а навичка яка тренується."
                    .into(),
            category: "mood".into(),
            duration_minutes: Some(5),
        }));
    }

    // Body scan for combined stress + sleep issues
    if m.stress_level >= 16.0 && m.sleep_duration < 7.0 {
        c.push((67, PlanItem {
            id: "body_scan".into(),
            title: "Сканування тіла".into(),
            description:
                "Повільний body scan перед сном (10 хв): \
                ляж зручно, закрий очі. Подумки пройди від макушки до пальців ніг, \
                затримуючись на кожній частині тіла на 3-5 вдихів: \
                голова → чоло → очі → щелепа → шия → плечі → руки → \
                груди → живіт → стегна → коліна → стопи. \
                На кожному видиху уяви як напруга витікає з цієї частини. \
                Це одна з найефективніших технік засинання (Jon Kabat-Zinn)."
                    .into(),
            category: "sleep".into(),
            duration_minutes: Some(10),
        }));
    }
}

pub fn plan_to_text(items: &[PlanItem]) -> String {
    let mut lines = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let duration = item
            .duration_minutes
            .map(|m| format!(" ({m} хв)"))
            .unwrap_or_default();
        lines.push(format!(
            "{}. {}{} — {}",
            idx + 1,
            item.title,
            duration,
            item.description
        ));
    }
    lines.join("\n\n")
}
