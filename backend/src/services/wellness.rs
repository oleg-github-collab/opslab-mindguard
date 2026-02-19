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

        // ===== POSITIVE REINFORCEMENT =====
        add_positive_reinforcement(&mut candidates, m);
    }

    // ===== UNIVERSAL & DAY-AWARE =====
    add_universal_items(&mut candidates, metrics, goals, weekday);

    // Sort by priority (highest first)
    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    // Determine max items based on severity
    let max_items = match metrics {
        Some(m) if m.mbi_score >= 70.0 || m.stress_level >= 28.0 || m.phq9_score >= 15.0 => 6,
        Some(m) if m.mbi_score >= 50.0 || m.stress_level >= 20.0 || m.phq9_score >= 10.0 => 5,
        Some(m) if m.who5_score < 50.0 || m.gad7_score >= 10.0 => 4,
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
    // Find which area needs most attention
    let mut worst = ("sleep", 0.0f64);
    // Normalize all to 0-1 where 1 is worst
    let sleep_bad = if m.sleep_duration < 8.0 { (8.0 - m.sleep_duration) / 8.0 } else { 0.0 };
    let stress_bad = m.stress_level / 40.0;
    let mood_bad = (100.0 - m.who5_score) / 100.0;
    let anxiety_bad = m.gad7_score / 21.0;
    let burnout_bad = m.mbi_score / 100.0;
    let balance_bad = (10.0 - m.work_life_balance) / 10.0;

    if sleep_bad > worst.1 { worst = ("sleep", sleep_bad); }
    if stress_bad > worst.1 { worst = ("stress", stress_bad); }
    if mood_bad > worst.1 { worst = ("mood", mood_bad); }
    if anxiety_bad > worst.1 { worst = ("anxiety", anxiety_bad); }
    if burnout_bad > worst.1 { worst = ("burnout", burnout_bad); }
    if balance_bad > worst.1 { worst = ("balance", balance_bad); }

    match worst.0 {
        "sleep" => "сон — ляг на 30 хв раніше сьогодні",
        "stress" => "стрес — зроби 5-хвилинну дихальну вправу",
        "mood" => "настрій — зроби 1 приємну дію для себе",
        "anxiety" => "тривога — спробуй техніку заземлення 5-4-3-2-1",
        "burnout" => "вигорання — делегуй або відклади 1 задачу",
        "balance" => "баланс — визнач стоп-тайм роботи сьогодні",
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
        _ => {} // Tue/Thu — no special day item
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
