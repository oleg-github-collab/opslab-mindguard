use std::collections::{HashMap, HashSet};

pub struct ToxicitySignal {
    pub flagged: bool,
    pub severity: i16,
    pub themes: Vec<String>,
}

pub fn analyze_toxicity(text: &str) -> ToxicitySignal {
    let lowered = text.to_lowercase();
    let mut severity = 0;
    let mut themes = HashSet::new();

    let severe_keywords = [
        "harassment",
        "булінг",
        "буллинг",
        "приниження",
        "погроза",
        "threat",
        "hate",
        "ненавист",
        "discrimination",
        "дискримінац",
        "abuse",
        "агрес",
        "насилля",
    ];
    if severe_keywords.iter().any(|k| lowered.contains(k)) {
        severity += 3;
        themes.insert("harassment".to_string());
    }

    let conflict_keywords = [
        "конфлікт",
        "конфликт",
        "токсич",
        "токсичн",
        "ігнор",
        "boycott",
        "саботаж",
        "скандал",
    ];
    if conflict_keywords.iter().any(|k| lowered.contains(k)) {
        severity += 2;
        themes.insert("conflict".to_string());
    }

    let burnout_keywords = [
        "burnout",
        "вигоран",
        "виснажен",
        "знесилен",
        "депрес",
        "тривог",
        "панік",
    ];
    if burnout_keywords.iter().any(|k| lowered.contains(k)) {
        severity += 1;
        themes.insert("burnout".to_string());
    }

    let workload_keywords = ["перевантаж", "переработ", "overtime", "дедлайн"];
    if workload_keywords.iter().any(|k| lowered.contains(k)) {
        themes.insert("workload".to_string());
    }

    let management_keywords = ["менеджмент", "керівництв", "лідер", "hr", "управл"];
    if management_keywords.iter().any(|k| lowered.contains(k)) {
        themes.insert("management".to_string());
    }

    let process_keywords = ["процес", "регламент", "бюрократ", "хаос", "quality", "якість"];
    if process_keywords.iter().any(|k| lowered.contains(k)) {
        themes.insert("process".to_string());
    }

    let flagged = severity >= 3;
    ToxicitySignal {
        flagged,
        severity,
        themes: themes.into_iter().collect(),
    }
}

pub fn extract_keywords(texts: &[String], limit: usize) -> Vec<String> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    let stop = stop_words();

    for text in texts {
        let normalized = text
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect::<String>();

        for token in normalized.split_whitespace() {
            if token.len() < 4 || stop.contains(token) {
                continue;
            }
            *freq.entry(token.to_string()).or_insert(0) += 1;
        }
    }

    let mut pairs: Vec<(String, usize)> = freq.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    pairs.into_iter().take(limit).map(|p| p.0).collect()
}

fn stop_words() -> HashSet<&'static str> {
    [
        "і",
        "в",
        "на",
        "та",
        "що",
        "як",
        "але",
        "це",
        "ми",
        "ви",
        "вони",
        "про",
        "для",
        "мене",
        "тому",
        "коли",
        "тут",
        "там",
        "цей",
        "ця",
        "це",
        "так",
        "not",
        "with",
        "this",
        "that",
        "your",
        "from",
    ]
    .into_iter()
    .collect()
}
