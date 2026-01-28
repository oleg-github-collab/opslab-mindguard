use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckinFrequency {
    Daily,
    #[serde(
        rename = "every_3_days",
        alias = "every3_days",
        alias = "every3days",
        alias = "every-3-days",
        alias = "3days",
        alias = "3_days"
    )]
    Every3Days,
    Weekly,
}

impl CheckinFrequency {
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckinFrequency::Daily => "daily",
            CheckinFrequency::Every3Days => "every_3_days",
            CheckinFrequency::Weekly => "weekly",
        }
    }

    pub fn cadence_days(&self) -> i64 {
        match self {
            CheckinFrequency::Daily => 1,
            CheckinFrequency::Every3Days => 3,
            CheckinFrequency::Weekly => 7,
        }
    }
}

impl TryFrom<&str> for CheckinFrequency {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "daily" => Ok(CheckinFrequency::Daily),
            "every_3_days" | "every3_days" | "every3days" | "every-3-days" | "3days" | "3_days" => {
                Ok(CheckinFrequency::Every3Days)
            }
            "weekly" | "week" => Ok(CheckinFrequency::Weekly),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckinSchedule {
    pub due: bool,
    pub next_due_date: NaiveDate,
    pub days_until: i64,
    pub last_date: Option<NaiveDate>,
}

pub fn schedule_for(
    frequency: CheckinFrequency,
    last_date: Option<NaiveDate>,
    today: NaiveDate,
) -> CheckinSchedule {
    let cadence = frequency.cadence_days();
    let due = last_date
        .map(|last| (today - last).num_days() >= cadence)
        .unwrap_or(true);

    let next_due_date = if due {
        today
    } else if let Some(last) = last_date {
        last + Duration::days(cadence)
    } else {
        today
    };

    let days_until = (next_due_date - today).num_days();

    CheckinSchedule {
        due,
        next_due_date,
        days_until,
        last_date,
    }
}
