use crate::db;
use crate::domain::models::UserRole;
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::Serialize;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricBlock {
    who5: f64,
    phq9: f64,
    gad7: f64,
    mbi: f64,
    sleep_duration: f64,
    sleep_quality: f64,
    work_life_balance: f64,
    stress_level: f64,
}

impl MetricBlock {
    fn zero() -> Self {
        Self {
            who5: 0.0,
            phq9: 0.0,
            gad7: 0.0,
            mbi: 0.0,
            sleep_duration: 0.0,
            sleep_quality: 0.0,
            work_life_balance: 0.0,
            stress_level: 0.0,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmployeeMonthMetrics {
    user_id: Uuid,
    name: String,
    #[serde(flatten)]
    metrics: MetricBlock,
    has_data: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamMonthData {
    year: i32,
    month: u32,
    averages: MetricBlock,
    employees: Vec<EmployeeMonthMetrics>,
    participants: i64,
    responses: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamDataResponse {
    company: String,
    last_updated: DateTime<Utc>,
    metadata: AnalyticsMetadata,
    months: Vec<TeamMonthData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmployeeProfile {
    user_id: Uuid,
    name: String,
    email: Option<String>,
    metrics: MetricBlock,
    history: HashMap<String, MetricBlock>,
    risk_level: String,
    notes: String,
    has_data: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsMetadata {
    assessment_tools: Vec<String>,
    update_frequency: String,
    next_assessment: Option<NaiveDate>,
    participation_rate: String,
    data_collection_period: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndustryBenchmarks {
    tech_sector: MetricBlock,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Alert {
    severity: String,
    employee: String,
    message: String,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Recommendation {
    category: String,
    title: String,
    description: String,
    affected_employees: Vec<String>,
    priority: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmployeeDataResponse {
    company: String,
    last_updated: DateTime<Utc>,
    employees: Vec<EmployeeProfile>,
    team_averages: HashMap<String, MetricBlock>,
    industry_benchmarks: IndustryBenchmarks,
    alerts: Vec<Alert>,
    recommendations: Vec<Recommendation>,
    metadata: AnalyticsMetadata,
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/team-data", get(team_data))
        .route("/employee-data", get(employee_data))
        .with_state(state)
}

async fn team_data(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<TeamDataResponse>, StatusCode> {
    let requester = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find user {} in team_data: {}", user_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !requester.is_active {
        return Err(StatusCode::FORBIDDEN);
    }

    let include_all = matches!(requester.role, UserRole::Admin | UserRole::Founder);
    tracing::debug!("team_data: user_id={}, include_all={}", user_id, include_all);

    let users = match fetch_users(&state, user_id, include_all).await {
        Ok(users) => users,
        Err(e) => {
            tracing::error!("Failed to fetch users: {:?}", e);
            Vec::new()
        }
    };
    tracing::debug!("team_data: fetched {} users", users.len());

    let analytics = match build_analytics(&state, &users).await {
        Ok(analytics) => analytics,
        Err(e) => {
            tracing::error!("Failed to build analytics: {:?}", e);
            AnalyticsData::empty()
        }
    };
    tracing::debug!("team_data: built analytics with {} months", analytics.months.len());

    let (company, metadata, meta_updated_at) = match load_metadata(&state).await {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Failed to load metadata: {:?}", e);
            default_metadata()
        }
    };
    let last_updated = resolve_last_updated(&analytics.month_keys, meta_updated_at);

    Ok(Json(TeamDataResponse {
        company,
        last_updated,
        metadata,
        months: analytics.months,
    }))
}

async fn employee_data(
    UserSession(user_id): UserSession,
    State(state): State<SharedState>,
) -> Result<Json<EmployeeDataResponse>, StatusCode> {
    let requester = db::find_user_by_id(&state.pool, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !requester.is_active {
        return Err(StatusCode::FORBIDDEN);
    }

    let include_all = matches!(requester.role, UserRole::Admin | UserRole::Founder);
    let users = match fetch_users(&state, user_id, include_all).await {
        Ok(users) => users,
        Err(e) => {
            tracing::error!("Failed to fetch users: {:?}", e);
            Vec::new()
        }
    };
    let analytics = match build_analytics(&state, &users).await {
        Ok(analytics) => analytics,
        Err(e) => {
            tracing::error!("Failed to build analytics: {:?}", e);
            AnalyticsData::empty()
        }
    };

    let latest_key = analytics
        .month_keys
        .last()
        .cloned()
        .unwrap_or_else(|| analytics.month_keys.first().cloned().unwrap_or_default());

    let mut team_averages = analytics.team_averages.clone();
    if let Some(latest_avg) = analytics.team_averages.get(&latest_key).cloned() {
        team_averages.insert("current".to_string(), latest_avg);
    }

    let mut employees = Vec::new();
    let mut alerts = Vec::new();
    let mut recommendations = Vec::new();

    for user in &users {
        let history = analytics
            .user_history
            .get(&user.id)
            .cloned()
            .unwrap_or_default();

        let metrics = latest_user_metrics(&history, &analytics.month_keys)
            .unwrap_or_else(MetricBlock::zero);
        let has_data = !history.is_empty();
        let risk_level = if has_data {
            calculate_employee_risk(&metrics)
        } else {
            "low"
        };

        let notes = build_notes(&metrics, &history, &analytics.month_keys, risk_level);

        employees.push(EmployeeProfile {
            user_id: user.id,
            name: user.name.clone(),
            email: if include_all { Some(user.email.clone()) } else { None },
            metrics,
            history,
            risk_level: risk_level.to_string(),
            notes,
            has_data,
        });
    }

    let (company, metadata, meta_updated_at) = match load_metadata(&state).await {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Failed to load metadata: {:?}", e);
            default_metadata()
        }
    };
    let last_updated = resolve_last_updated(&analytics.month_keys, meta_updated_at);
    let industry_benchmarks = load_industry_benchmarks(&state).await?;

    if include_all {
        alerts = load_alerts(&state, &users, &analytics).await?;
        recommendations = load_recommendations(&state, &employees, &analytics).await?;
    }

    Ok(Json(EmployeeDataResponse {
        company,
        last_updated,
        employees,
        team_averages,
        industry_benchmarks,
        alerts,
        recommendations,
        metadata,
    }))
}

struct AnalyticsData {
    months: Vec<TeamMonthData>,
    team_averages: HashMap<String, MetricBlock>,
    user_history: HashMap<Uuid, HashMap<String, MetricBlock>>,
    month_keys: Vec<String>,
}

impl AnalyticsData {
    fn empty() -> Self {
        Self {
            months: Vec::new(),
            team_averages: HashMap::new(),
            user_history: HashMap::new(),
            month_keys: Vec::new(),
        }
    }
}

struct UserInfo {
    id: Uuid,
    name: String,
    email: String,
}

async fn fetch_users(
    state: &SharedState,
    requester_id: Uuid,
    include_all: bool,
) -> Result<Vec<UserInfo>, StatusCode> {
    let users = if include_all {
        sqlx::query(
            r#"
            SELECT id, email, enc_name
            FROM users
            WHERE role != 'ADMIN'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query(
            r#"
            SELECT id, email, enc_name
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(requester_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let mut out = Vec::new();
    for row in users {
        let id: Uuid = row.try_get("id").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let email: String = row.try_get("email").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let enc_name: String = row.try_get("enc_name").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let name = state
            .crypto
            .decrypt_str(&enc_name)
            .unwrap_or_else(|_| "User".to_string());
        out.push(UserInfo { id, name, email });
    }
    Ok(out)
}

async fn build_analytics(
    state: &SharedState,
    users: &[UserInfo],
) -> Result<AnalyticsData, StatusCode> {
    if users.is_empty() {
        return Ok(AnalyticsData {
            months: Vec::new(),
            team_averages: HashMap::new(),
            user_history: HashMap::new(),
            month_keys: Vec::new(),
        });
    }

    let user_ids: Vec<Uuid> = users.iter().map(|u| u.id).collect();

    #[derive(sqlx::FromRow)]
    struct MonthRow {
        user_id: Uuid,
        year: i32,
        month: i32,
        mood_avg: Option<f64>,
        energy_avg: Option<f64>,
        wellbeing_avg: Option<f64>,
        motivation_avg: Option<f64>,
        focus_avg: Option<f64>,
        stress_avg: Option<f64>,
        sleep_avg: Option<f64>,
        workload_avg: Option<f64>,
        answers_count: i64,
    }

    let rows: Vec<MonthRow> = match sqlx::query_as(
        r#"
        SELECT
            user_id,
            EXTRACT(YEAR FROM created_at)::int AS year,
            EXTRACT(MONTH FROM created_at)::int AS month,
            AVG(CASE WHEN question_type = 'mood' THEN value END)::double precision AS mood_avg,
            AVG(CASE WHEN question_type = 'energy' THEN value END)::double precision AS energy_avg,
            AVG(CASE WHEN question_type = 'wellbeing' THEN value END)::double precision AS wellbeing_avg,
            AVG(CASE WHEN question_type = 'motivation' THEN value END)::double precision AS motivation_avg,
            AVG(CASE WHEN question_type = 'focus' THEN value END)::double precision AS focus_avg,
            AVG(CASE WHEN question_type = 'stress' THEN value END)::double precision AS stress_avg,
            AVG(CASE WHEN question_type = 'sleep' THEN value END)::double precision AS sleep_avg,
            AVG(CASE WHEN question_type = 'workload' THEN value END)::double precision AS workload_avg,
            COUNT(*)::bigint AS answers_count
        FROM checkin_answers
        WHERE user_id = ANY($1)
        GROUP BY 1, 2, 3
        ORDER BY 2, 3
        "#,
    )
    .bind(&user_ids)
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to load analytics rows: {}", e);
            Vec::new()
        }
    };

    let mut user_history: HashMap<Uuid, HashMap<String, MetricBlock>> = HashMap::new();
    let mut month_keys_set: HashSet<String> = HashSet::new();
    let mut month_participants: HashMap<String, i64> = HashMap::new();
    let mut month_responses: HashMap<String, i64> = HashMap::new();
    let mut user_months: HashSet<(Uuid, String)> = HashSet::new();

    for row in rows {
        let key = format!("{}-{:02}", row.year, row.month);
        let metrics = compute_metrics(
            row.mood_avg,
            row.energy_avg,
            row.wellbeing_avg,
            row.motivation_avg,
            row.focus_avg,
            row.stress_avg,
            row.sleep_avg,
            row.workload_avg,
        );

        user_history
            .entry(row.user_id)
            .or_default()
            .insert(key.clone(), metrics);
        month_keys_set.insert(key.clone());

        *month_responses.entry(key.clone()).or_insert(0) += row.answers_count;
        if user_months.insert((row.user_id, key.clone())) {
            *month_participants.entry(key).or_insert(0) += 1;
        }
    }

    let overrides = match db::get_monthly_metric_overrides(&state.pool, &user_ids).await {
        Ok(overrides) => overrides,
        Err(e) => {
            tracing::error!("Failed to load monthly metric overrides: {}", e);
            Vec::new()
        }
    };
    for row in overrides {
        let key = format!("{}-{:02}", row.period.year(), row.period.month());
        let metrics = MetricBlock {
            who5: row.who5,
            phq9: row.phq9,
            gad7: row.gad7,
            mbi: row.mbi,
            sleep_duration: row.sleep_duration,
            sleep_quality: row.sleep_quality,
            work_life_balance: row.work_life_balance,
            stress_level: row.stress_level,
        };
        let entry = user_history.entry(row.user_id).or_default();
        let had_data = entry.contains_key(&key);
        entry.insert(key.clone(), metrics);
        month_keys_set.insert(key.clone());
        if !had_data {
            *month_participants.entry(key).or_insert(0) += 1;
        }
    }

    let mut month_keys: Vec<String> = month_keys_set.into_iter().collect();
    month_keys.sort_by(|a, b| {
        let (ay, am) = parse_month_key(a);
        let (by, bm) = parse_month_key(b);
        ay.cmp(&by).then(am.cmp(&bm))
    });

    if !month_keys.is_empty() {
        let (start_year, start_month) = parse_month_key(&month_keys[0]);
        let (end_year, end_month) = parse_month_key(month_keys.last().unwrap());
        let mut expanded = Vec::new();
        let mut year = start_year;
        let mut month = start_month;
        loop {
            expanded.push(format!("{year}-{month:02}"));
            if year == end_year && month == end_month {
                break;
            }
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
        month_keys = expanded;
    }

    let mut months_output = Vec::new();
    let mut team_averages = HashMap::new();

    for key in &month_keys {
        let (year, month) = parse_month_key(key);
        let mut employees = Vec::new();
        let mut sum = MetricBlock::zero();
        let mut count = 0.0f64;

        for user in users {
            let metrics = user_history
                .get(&user.id)
                .and_then(|h| h.get(key))
                .cloned()
                .unwrap_or_else(MetricBlock::zero);

            let has_data = user_history
                .get(&user.id)
                .and_then(|h| h.get(key))
                .is_some();

            if has_data {
                sum = add_metrics(&sum, &metrics);
                count += 1.0;
            }

            employees.push(EmployeeMonthMetrics {
                user_id: user.id,
                name: user.name.clone(),
                metrics,
                has_data,
            });
        }

        let averages = if count > 0.0 {
            divide_metrics(&sum, count)
        } else {
            MetricBlock::zero()
        };

        team_averages.insert(key.clone(), averages.clone());
        let participants = *month_participants.get(key).unwrap_or(&0);
        let responses = *month_responses.get(key).unwrap_or(&0);

        months_output.push(TeamMonthData {
            year,
            month: month as u32,
            averages,
            employees,
            participants,
            responses,
        });
    }

    Ok(AnalyticsData {
        months: months_output,
        team_averages,
        user_history,
        month_keys,
    })
}

fn parse_month_key(key: &str) -> (i32, i32) {
    let mut parts = key.split('-');
    let year = parts.next().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    let month = parts.next().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    (year, month)
}

fn compute_metrics(
    mood_avg: Option<f64>,
    energy_avg: Option<f64>,
    wellbeing_avg: Option<f64>,
    motivation_avg: Option<f64>,
    focus_avg: Option<f64>,
    stress_avg: Option<f64>,
    sleep_avg: Option<f64>,
    workload_avg: Option<f64>,
) -> MetricBlock {
    fn avg(values: &[Option<f64>]) -> Option<f64> {
        let mut sum = 0.0;
        let mut count = 0.0;
        for value in values {
            if let Some(v) = value {
                sum += v;
                count += 1.0;
            }
        }
        if count > 0.0 {
            Some(sum / count)
        } else {
            None
        }
    }

    let who5 = avg(&[mood_avg, energy_avg, wellbeing_avg])
        .map(|v| (v * 10.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    let phq9 = avg(&[
        mood_avg.map(|v| 10.0 - v),
        energy_avg.map(|v| 10.0 - v),
        motivation_avg.map(|v| 10.0 - v),
    ])
    .map(|v| (v * 2.7).clamp(0.0, 27.0))
    .unwrap_or(0.0);

    let gad7 = avg(&[stress_avg, focus_avg.map(|v| 10.0 - v)])
        .map(|v| (v * 2.1).clamp(0.0, 21.0))
        .unwrap_or(0.0);

    let mbi = avg(&[
        stress_avg,
        workload_avg,
        energy_avg.map(|v| 10.0 - v),
        motivation_avg.map(|v| 10.0 - v),
    ])
    .map(|v| (v * 10.0).clamp(0.0, 100.0))
    .unwrap_or(0.0);

    let sleep = sleep_avg.unwrap_or(0.0);
    let stress_level = stress_avg.map(|v| (v * 4.0).clamp(0.0, 40.0)).unwrap_or(0.0);
    let work_life_balance = workload_avg
        .map(|v| (10.0 - v).clamp(0.0, 10.0))
        .unwrap_or(0.0);

    MetricBlock {
        who5,
        phq9,
        gad7,
        mbi,
        sleep_duration: sleep,
        sleep_quality: sleep,
        work_life_balance,
        stress_level,
    }
}

fn add_metrics(a: &MetricBlock, b: &MetricBlock) -> MetricBlock {
    MetricBlock {
        who5: a.who5 + b.who5,
        phq9: a.phq9 + b.phq9,
        gad7: a.gad7 + b.gad7,
        mbi: a.mbi + b.mbi,
        sleep_duration: a.sleep_duration + b.sleep_duration,
        sleep_quality: a.sleep_quality + b.sleep_quality,
        work_life_balance: a.work_life_balance + b.work_life_balance,
        stress_level: a.stress_level + b.stress_level,
    }
}

fn divide_metrics(a: &MetricBlock, denom: f64) -> MetricBlock {
    MetricBlock {
        who5: a.who5 / denom,
        phq9: a.phq9 / denom,
        gad7: a.gad7 / denom,
        mbi: a.mbi / denom,
        sleep_duration: a.sleep_duration / denom,
        sleep_quality: a.sleep_quality / denom,
        work_life_balance: a.work_life_balance / denom,
        stress_level: a.stress_level / denom,
    }
}

fn calculate_employee_risk(metrics: &MetricBlock) -> &'static str {
    if metrics.who5 < 28.0
        || metrics.phq9 > 15.0
        || metrics.gad7 > 15.0
        || metrics.mbi >= 70.0
        || metrics.stress_level >= 30.0
    {
        return "critical";
    }
    if metrics.who5 < 50.0 || metrics.phq9 > 10.0 || metrics.gad7 > 10.0 || metrics.mbi > 40.0 {
        return "high";
    }
    if metrics.mbi > 30.0 || metrics.stress_level > 20.0 {
        return "medium";
    }
    "low"
}

fn latest_user_metrics(
    history: &HashMap<String, MetricBlock>,
    month_keys: &[String],
) -> Option<MetricBlock> {
    if let Some(last_key) = month_keys.last() {
        if let Some(metrics) = history.get(last_key) {
            return Some(metrics.clone());
        }
    }
    for key in month_keys.iter().rev() {
        if let Some(metrics) = history.get(key) {
            return Some(metrics.clone());
        }
    }
    None
}

fn build_notes(
    metrics: &MetricBlock,
    history: &HashMap<String, MetricBlock>,
    month_keys: &[String],
    risk_level: &str,
) -> String {
    if history.is_empty() {
        return "Дані поки що відсутні. Додайте чекін, щоб побачити аналітику.".to_string();
    }

    let latest = month_keys.last().and_then(|k| history.get(k));
    let prev = if month_keys.len() >= 2 {
        history.get(&month_keys[month_keys.len() - 2])
    } else {
        None
    };

    let who5_delta = match (latest, prev) {
        (Some(latest), Some(prev)) => latest.who5 - prev.who5,
        _ => 0.0,
    };

    match risk_level {
        "critical" => format!(
            "Критичний стан. WHO-5: {:.1}, PHQ-9: {:.1}, GAD-7: {:.1}, MBI: {:.1}%. Потрібна термінова підтримка.",
            metrics.who5, metrics.phq9, metrics.gad7, metrics.mbi
        ),
        "high" => format!(
            "Підвищений ризик. WHO-5: {:.1}, PHQ-9: {:.1}, стрес: {:.1}/40. Рекомендується 1:1 і зниження навантаження.",
            metrics.who5, metrics.phq9, metrics.stress_level
        ),
        _ => {
            if who5_delta >= 10.0 {
                format!(
                    "Позитивна динаміка WHO-5 +{:.1}. Продовжуйте підтримувати баланс.",
                    who5_delta
                )
            } else if who5_delta <= -10.0 {
                format!(
                    "Негативна динаміка WHO-5 {:.1}. Варто перевірити фактори стресу.",
                    who5_delta
                )
            } else {
                "Стабільні показники. Продовжуйте регулярні чекіни.".to_string()
            }
        }
    }
}

async fn load_metadata(
    state: &SharedState,
) -> Result<(String, AnalyticsMetadata, Option<DateTime<Utc>>), StatusCode> {
    let row = db::get_analytics_metadata(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(row) = row {
        let db::AnalyticsMetadataRow {
            company,
            data_collection_period,
            update_frequency,
            next_assessment,
            participation_rate,
            assessment_tools,
            updated_at,
        } = row;
        let metadata = AnalyticsMetadata {
            assessment_tools,
            update_frequency: update_frequency.unwrap_or_else(|| "monthly".to_string()),
            next_assessment,
            participation_rate: participation_rate.unwrap_or_default(),
            data_collection_period: data_collection_period.unwrap_or_default(),
        };
        Ok((company, metadata, Some(updated_at)))
    } else {
        Ok(default_metadata())
    }
}

fn default_metadata() -> (String, AnalyticsMetadata, Option<DateTime<Utc>>) {
    (
        "OpsLab".to_string(),
        AnalyticsMetadata {
            assessment_tools: vec![
                "WHO-5 Well-Being Index (0-100)".to_string(),
                "PHQ-9 Patient Health Questionnaire (0-27)".to_string(),
                "GAD-7 Generalized Anxiety Disorder (0-21)".to_string(),
                "MBI Maslach Burnout Inventory (0-100%)".to_string(),
            ],
            update_frequency: "monthly".to_string(),
            next_assessment: None,
            participation_rate: "—".to_string(),
            data_collection_period: String::new(),
        },
        None,
    )
}

fn resolve_last_updated(month_keys: &[String], meta_updated_at: Option<DateTime<Utc>>) -> DateTime<Utc> {
    if let Some(last_key) = month_keys.last() {
        let (year, month) = parse_month_key(last_key);
        if let Some(date) = month_end_date(year, month) {
            if let Some(datetime) = date.and_hms_opt(0, 0, 0) {
                return DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc);
            }
        }
    }
    meta_updated_at.unwrap_or_else(Utc::now)
}

fn month_end_date(year: i32, month: i32) -> Option<NaiveDate> {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1)?
    };
    Some(next - chrono::Duration::days(1))
}

async fn load_industry_benchmarks(state: &SharedState) -> Result<IndustryBenchmarks, StatusCode> {
    let rows = db::get_industry_benchmarks(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut tech = MetricBlock {
        who5: 58.0,
        phq9: 8.0,
        gad7: 7.0,
        mbi: 30.0,
        sleep_duration: 6.5,
        sleep_quality: 0.0,
        work_life_balance: 5.5,
        stress_level: 7.0,
    };

    if let Some(row) = rows
        .into_iter()
        .find(|row| row.sector.to_lowercase() == "tech")
    {
        tech.who5 = row.who5.unwrap_or(tech.who5);
        tech.phq9 = row.phq9.unwrap_or(tech.phq9);
        tech.gad7 = row.gad7.unwrap_or(tech.gad7);
        tech.mbi = row.mbi.unwrap_or(tech.mbi);
        tech.sleep_duration = row.sleep_duration.unwrap_or(tech.sleep_duration);
        tech.work_life_balance = row.work_life_balance.unwrap_or(tech.work_life_balance);
        tech.stress_level = row.stress_level.unwrap_or(tech.stress_level);
    }

    Ok(IndustryBenchmarks { tech_sector: tech })
}

async fn load_alerts(
    state: &SharedState,
    users: &[UserInfo],
    analytics: &AnalyticsData,
) -> Result<Vec<Alert>, StatusCode> {
    let rows = db::get_analytics_alerts(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !rows.is_empty() {
        return Ok(rows
            .into_iter()
            .map(|row| Alert {
                severity: row.severity,
                employee: row.employee_name,
                message: row.message,
                timestamp: row.timestamp,
            })
            .collect());
    }

    let mut alerts = Vec::new();
    let latest_key = analytics.month_keys.last().cloned();
    let prev_key = if analytics.month_keys.len() >= 2 {
        Some(analytics.month_keys[analytics.month_keys.len() - 2].clone())
    } else {
        analytics.month_keys.first().cloned()
    };

    for user in users {
        let history = analytics
            .user_history
            .get(&user.id)
            .cloned()
            .unwrap_or_default();
        if history.is_empty() {
            continue;
        }
        let metrics = latest_user_metrics(&history, &analytics.month_keys)
            .unwrap_or_else(MetricBlock::zero);
        let risk_level = calculate_employee_risk(&metrics);

        if risk_level == "critical" {
            alerts.push(Alert {
                severity: "critical".to_string(),
                employee: user.name.clone(),
                message: format!(
                    "Критичний стан! WHO-5 {:.1}, PHQ-9 {:.1}, GAD-7 {:.1}, MBI {:.1}%",
                    metrics.who5, metrics.phq9, metrics.gad7, metrics.mbi
                ),
                timestamp: Utc::now(),
            });
            continue;
        }

        if let (Some(latest_key), Some(prev_key)) = (latest_key.as_ref(), prev_key.as_ref()) {
            if let (Some(latest), Some(prev)) = (history.get(latest_key), history.get(prev_key)) {
                let who5_delta = latest.who5 - prev.who5;
                let mbi_delta = latest.mbi - prev.mbi;
                if who5_delta <= -20.0 {
                    alerts.push(Alert {
                        severity: "critical".to_string(),
                        employee: user.name.clone(),
                        message: format!(
                            "Різке погіршення WHO-5 ({:.1}). Потрібна термінова інтервенція.",
                            who5_delta
                        ),
                        timestamp: Utc::now(),
                    });
                } else if who5_delta >= 10.0 || mbi_delta <= -15.0 {
                    alerts.push(Alert {
                        severity: "positive".to_string(),
                        employee: user.name.clone(),
                        message: format!(
                            "Відновлення: WHO-5 {:+.1}, MBI {:+.1}. Позитивна динаміка!",
                            who5_delta, mbi_delta
                        ),
                        timestamp: Utc::now(),
                    });
                }
            }
        }
    }

    Ok(alerts)
}

async fn load_recommendations(
    state: &SharedState,
    employees: &[EmployeeProfile],
    analytics: &AnalyticsData,
) -> Result<Vec<Recommendation>, StatusCode> {
    let rows = db::get_analytics_recommendations(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !rows.is_empty() {
        return Ok(rows
            .into_iter()
            .map(|row| Recommendation {
                category: row.category,
                title: row.title,
                description: row.description,
                affected_employees: row.affected_employees,
                priority: row.priority,
            })
            .collect());
    }

    let mut recommendations = Vec::new();
    let critical: Vec<String> = employees
        .iter()
        .filter(|e| e.risk_level == "critical")
        .map(|e| e.name.clone())
        .collect();
    let high: Vec<String> = employees
        .iter()
        .filter(|e| e.risk_level == "high")
        .map(|e| e.name.clone())
        .collect();

    if !critical.is_empty() {
        recommendations.push(Recommendation {
            category: "emergency".to_string(),
            title: "Термінова психологічна підтримка".to_string(),
            description: "Критичні показники у частини команди. Необхідні швидкі 1:1 та доступ до психолога."
                .to_string(),
            affected_employees: critical.clone(),
            priority: "critical".to_string(),
        });
    }

    if !high.is_empty() {
        recommendations.push(Recommendation {
            category: "urgent".to_string(),
            title: "Перерозподіл навантаження".to_string(),
            description: "Перевірити обсяг задач, зменшити навантаження та дати відновлення."
                .to_string(),
            affected_employees: high.clone(),
            priority: "high".to_string(),
        });
    }

    let mut improvement = Vec::new();
    let latest_key = analytics.month_keys.last().cloned();
    let prev_key = if analytics.month_keys.len() >= 2 {
        Some(analytics.month_keys[analytics.month_keys.len() - 2].clone())
    } else {
        analytics.month_keys.first().cloned()
    };
    for emp in employees {
        if let (Some(latest_key), Some(prev_key)) = (latest_key.as_ref(), prev_key.as_ref()) {
            if let (Some(latest), Some(prev)) =
                (emp.history.get(latest_key), emp.history.get(prev_key))
            {
                if latest.who5 - prev.who5 >= 10.0 || prev.mbi - latest.mbi >= 15.0 {
                    improvement.push(emp.name.clone());
                }
            }
        }
    }

    if !improvement.is_empty() {
        recommendations.push(Recommendation {
            category: "positive".to_string(),
            title: "Аналіз практик відновлення".to_string(),
            description: "Вивчити фактори успіху сильних позитивних змін і масштабувати на команду."
                .to_string(),
            affected_employees: improvement,
            priority: "high".to_string(),
        });
    }

    if let Some(latest_key) = analytics.month_keys.last() {
        if let Some(avg) = analytics.team_averages.get(latest_key) {
            if avg.mbi > 40.0 {
                recommendations.push(Recommendation {
                    category: "general".to_string(),
                    title: "Організаційні зміни".to_string(),
                    description: "Командний рівень вигорання залишається високим — потрібні системні зміни в процесах."
                        .to_string(),
                    affected_employees: vec!["all".to_string()],
                    priority: "critical".to_string(),
                });
            }
        }
    }

    Ok(recommendations)
}
