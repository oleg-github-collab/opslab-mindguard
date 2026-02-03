mod analytics;
mod bot;
mod crypto;
mod db;
mod domain;
mod import_utils;
mod middleware;
mod services;
mod state;
mod web;
mod time_utils;

use crate::db::seed;
use crate::state::SharedState;
use axum::{
    body::Body,
    http::{header, Request, Response},
    middleware::{self as axum_middleware, Next},
    routing::get_service,
    Router,
};
use base64::{engine::general_purpose, Engine as _};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use chrono::NaiveDate;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{services::ServeDir, services::ServeFile, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Custom middleware to add cache-busting headers
async fn add_cache_headers(req: Request<Body>, next: Next) -> Response<Body> {
    let path = req.uri().path().to_string();
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    if path.starts_with("/static/") {
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    } else {
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store, no-cache, must-revalidate, proxy-revalidate"),
        );
        headers.insert(
            header::PRAGMA,
            header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            header::EXPIRES,
            header::HeaderValue::from_static("0"),
        );
    }
    headers.insert(
        header::HeaderName::from_static("x-mindguard-version"),
        header::HeaderValue::from_static("2026-01-12"),
    );

    response
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Application error: {:#}", e);
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    normalize_env_vars();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 OpsLab Mindguard starting...");
    tracing::info!(
        "Environment: {}",
        std::env::var("RAILWAY_ENVIRONMENT").unwrap_or_else(|_| "development".to_string())
    );

    let database_url = load_database_url()?;
    tracing::info!("Database URL configured");
    tracing::info!("Connecting to database...");
    let pool = connect_with_retry(&database_url).await?;
    tracing::info!("Database connection established");

    // Run database migrations
    tracing::info!("Running database migrations...");
    run_migrations(&pool).await?;
    tracing::info!("Database migrations completed");

    let (enc_key_source, enc_key_bytes) = load_key_material(&[
        "APP_ENC_KEY",
        "SESSION_KEY",
        "SESSION_KEY_BASE64",
        "SECRET_KEY",
    ])
    .ok_or_else(|| {
        anyhow::anyhow!(
            "APP_ENC_KEY missing or invalid (expected base64/hex 32 bytes)"
        )
    })?;
    if enc_key_source != "APP_ENC_KEY" {
        tracing::warn!(
            "APP_ENC_KEY missing; using {} as encryption key source",
            enc_key_source
        );
    }
    let crypto = Arc::new(crypto::Crypto::from_key_bytes(&enc_key_bytes)?);

    let (session_key_source, session_key) = load_key_material(&[
        "SESSION_KEY",
        "SESSION_KEY_BASE64",
        "APP_ENC_KEY",
        "SECRET_KEY",
    ])
    .ok_or_else(|| {
        anyhow::anyhow!(
            "SESSION_KEY missing or invalid (expected base64/hex 32 bytes)"
        )
    })?;
    if session_key_source != "SESSION_KEY" {
        tracing::warn!(
            "SESSION_KEY missing; using {} as session key source",
            session_key_source
        );
    }

    seed::seed_all(&pool, &crypto).await?;

    let openai_key = std::env::var("OPENAI_API_KEY").ok();
    if openai_key.is_none() {
        tracing::warn!("OPENAI_API_KEY not set; AI features disabled");
    }
    let ai = Arc::new(services::ai::AiService::new(openai_key, crypto.clone()));

    let poll_engine = domain::polling::PollEngine::new();
    let checkin_sessions = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let web_checkin_sessions =
        Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let shared: SharedState = Arc::new(state::AppState {
        pool,
        crypto,
        ai,
        poll_engine,
        session_key,
        checkin_sessions,
        web_checkin_sessions,
    });

    // Setup scheduler for daily check-ins and weekly summaries
    let scheduler = JobScheduler::new().await?;

    // #2 WOW Feature: Smart Reminders - timezone-aware reminders
    let shared_for_reminders = shared.clone();
    scheduler
        .add(Job::new_async("0 * * * * *", move |_uuid, _l| {
            let state = shared_for_reminders.clone();
            Box::pin(async move {
                let now = chrono::Utc::now();
                let candidates = match db::get_reminder_candidates(&state.pool).await {
                    Ok(list) => list,
                    Err(e) => {
                        tracing::error!("Failed to get reminder candidates: {}", e);
                        return;
                    }
                };

                let mut due_users: Vec<DueReminder> = Vec::new();

                for candidate in candidates {
                    if !candidate.notification_enabled {
                        continue;
                    }
                    if !candidate.onboarding_completed {
                        continue;
                    }

                    let (local_date, local_hour, local_minute) =
                        time_utils::local_components(&candidate.timezone, now);

                    let is_due_time = local_hour > candidate.reminder_hour
                        || (local_hour == candidate.reminder_hour
                            && local_minute >= candidate.reminder_minute);

                    if is_due_time
                    {
                        match db::mark_reminder_sent(&state.pool, candidate.user_id, local_date)
                            .await
                        {
                            Ok(true) => {
                                due_users.push(DueReminder {
                                    user_id: candidate.user_id,
                                    telegram_id: candidate.telegram_id,
                                    local_date,
                                });
                            }
                            Ok(false) => {}
                            Err(e) => {
                                tracing::error!(
                                    "Failed to mark reminder sent for {}: {}",
                                    candidate.user_id,
                                    e
                                );
                            }
                        }
                    }
                }

                if !due_users.is_empty() {
                    tracing::info!("Sending smart reminders to {} users", due_users.len());
                    if let Err(e) = send_smart_reminders(&state, due_users).await {
                        tracing::error!("Failed to send smart reminders: {}", e);
                    }
                }
            })
        })?)
        .await?;

    // #6 WOW Feature: Weekly Summary - Friday at 17:00
    let shared_for_weekly = shared.clone();
    scheduler
        .add(Job::new_async("0 0 17 * * FRI", move |_uuid, _l| {
            let state = shared_for_weekly.clone();
            Box::pin(async move {
                tracing::info!("Sending weekly summaries...");
                if let Err(e) = bot::weekly_summary::send_weekly_summaries(&state).await {
                    tracing::error!("Failed to send weekly summaries: {}", e);
                } else {
                    tracing::info!("Weekly summaries sent successfully!");
                }
            })
        })?)
        .await?;

    // Session cleanup - remove expired sessions every hour
    let shared_for_cleanup = shared.clone();
    scheduler
        .add(Job::new_async("0 0 * * * *", move |_uuid, _l| {
            let state = shared_for_cleanup.clone();
            Box::pin(async move {
                let mut sessions = state.checkin_sessions.write().await;
                let before_count = sessions.len();
                sessions.clear();
                if before_count > 0 {
                    tracing::info!("Cleaned up {} expired check-in sessions", before_count);
                }

                let mut web_sessions = state.web_checkin_sessions.write().await;
                let before_web = web_sessions.len();
                let now = chrono::Utc::now();
                web_sessions.retain(|_, session| session.expires_at > now);
                let removed_web = before_web.saturating_sub(web_sessions.len());
                if removed_web > 0 {
                    tracing::info!("Cleaned up {} expired web check-in sessions", removed_web);
                }
            })
        })?)
        .await?;

    scheduler.start().await?;
    tracing::info!("Scheduler started:");
    tracing::info!("  - Smart reminders: every minute (timezone-aware)");
    tracing::info!("  - Weekly summaries: Fridays 17:00");
    tracing::info!("  - Session cleanup: hourly");

    let shared_for_announcement = shared.clone();
    tokio::spawn(async move {
        if let Err(e) =
            bot::enhanced_handlers::send_web_checkin_rollout_announcement(&shared_for_announcement)
                .await
        {
            tracing::warn!("Web check-in rollout announcement failed: {}", e);
        }
    });

    let static_handler = ServeDir::new("static").not_found_service(ServeFile::new("index.html"));

    let app = Router::new()
        .merge(web::routes(shared.clone()))
        .merge(bot::enhanced_handlers::routes(shared.clone()))
        .nest_service("/static", get_service(ServeDir::new("static")))
        .fallback_service(get_service(static_handler))
        .layer(axum_middleware::from_fn(add_cache_headers))
        .layer(TraceLayer::new_for_http());

    let port = std::env::var("PORT")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            if is_production_like() {
                tracing::warn!("PORT not set; defaulting to 8080 for production");
                return "8080".to_string();
            }

            std::env::var("BIND_ADDR")
                .ok()
                .and_then(|addr| {
                    let trimmed = addr.trim().trim_matches('"').trim_matches('\'');
                    trimmed.rsplit_once(':').map(|(_, p)| p.to_string())
                })
                .unwrap_or_else(|| "3000".to_string())
        });
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("PORT env: {:?}", std::env::var("PORT"));
    tracing::info!("BIND_ADDR env: {:?}", std::env::var("BIND_ADDR"));
    tracing::info!("Binding to {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        tracing::error!("Failed to bind to {}: {}", addr, e);
        e
    })?;
    tracing::info!("✓ Server successfully started on {addr}");
    tracing::info!("✓ Health check endpoint: http://{}/health", addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

fn normalize_env_vars() {
    let keys = [
        "DATABASE_URL",
        "DATABASE_PRIVATE_URL",
        "POSTGRES_URL",
        "POSTGRESQL_URL",
        "APP_ENC_KEY",
        "SESSION_KEY",
        "SESSION_KEY_BASE64",
        "SECRET_KEY",
        "TELEGRAM_BOT_TOKEN",
        "BOT_USERNAME",
        "OPENAI_API_KEY",
        "APP_BASE_URL",
        "PUBLIC_BASE_URL",
        "BIND_ADDR",
        "PORT",
        "ADMIN_TELEGRAM_ID",
        "JANE_TELEGRAM_ID",
        "TELEGRAM_ADMIN_CHAT_ID",
        "TELEGRAM_JANE_CHAT_ID",
    ];

    for key in keys {
        if let Ok(raw) = std::env::var(key) {
            let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
            if trimmed != raw {
                std::env::set_var(key, trimmed);
                tracing::warn!("Env var {} contained quotes/whitespace; trimmed", key);
            }
        }
    }
}

fn load_database_url() -> anyhow::Result<String> {
    let candidates = ["DATABASE_URL", "DATABASE_PRIVATE_URL", "POSTGRES_URL", "POSTGRESQL_URL"];
    for key in candidates {
        if let Ok(raw) = std::env::var(key) {
            let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
            if !trimmed.is_empty() {
                if key != "DATABASE_URL" {
                    tracing::warn!("DATABASE_URL not set; using {} instead", key);
                }
                return Ok(trimmed.to_string());
            }
        }
    }
    Err(anyhow::anyhow!("DATABASE_URL missing"))
}

fn load_key_material(keys: &[&str]) -> Option<(String, Vec<u8>)> {
    for key in keys {
        if let Ok(raw) = std::env::var(key) {
            if let Some(bytes) = decode_key_material(&raw) {
                return Some(((*key).to_string(), bytes));
            }
            tracing::warn!("{} present but invalid; expected base64 or hex", key);
        }
    }
    None
}

fn decode_key_material(raw: &str) -> Option<Vec<u8>> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(bytes) = general_purpose::STANDARD.decode(trimmed) {
        if bytes.len() == 32 {
            return Some(bytes);
        }
    }
    decode_hex(trimmed).filter(|bytes| bytes.len() == 32)
}

fn decode_hex(raw: &str) -> Option<Vec<u8>> {
    if raw.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    let bytes = raw.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = hex_value(chunk[0])?;
        let lo = hex_value(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn is_production_like() -> bool {
    if let Ok(val) = std::env::var("PRODUCTION") {
        let lower = val.trim().to_lowercase();
        if matches!(lower.as_str(), "true" | "1" | "yes") {
            return true;
        }
    }
    std::env::var("RAILWAY_ENVIRONMENT").is_ok()
        || std::env::var("RAILWAY_PROJECT_ID").is_ok()
}

async fn connect_with_retry(database_url: &str) -> anyhow::Result<PgPool> {
    let timeout_secs = std::env::var("DB_CONNECT_TIMEOUT_SECONDS")
        .ok()
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(180);
    let start = Instant::now();
    let mut attempt = 0u64;

    loop {
        attempt += 1;
        match PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                if attempt > 1 {
                    tracing::info!("Database connected after {} attempts", attempt);
                }
                return Ok(pool);
            }
            Err(e) => {
                let elapsed = start.elapsed();
                if elapsed >= Duration::from_secs(timeout_secs) {
                    tracing::error!(
                        "Failed to connect to database after {}s: {}",
                        timeout_secs,
                        e
                    );
                    return Err(e.into());
                }
                let backoff = Duration::from_secs(attempt.min(10));
                tracing::warn!(
                    "Database connect attempt {} failed: {}. Retrying in {}s",
                    attempt,
                    e,
                    backoff.as_secs()
                );
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let migrator = sqlx::migrate!("./migrations");
    match migrator.run(pool).await {
        Ok(_) => Ok(()),
        Err(err) => {
            if let sqlx::migrate::MigrateError::VersionMismatch(version) = err {
                tracing::error!("Migration {} checksum mismatch detected", version);
                if allow_migration_repair() {
                    tracing::warn!("Repairing migration checksum for version {}", version);
                    repair_migration_checksum(pool, &migrator, version).await?;
                    migrator.run(pool).await.map_err(|e| {
                        tracing::error!("Failed to run migrations after repair: {}", e);
                        e
                    })?;
                    Ok(())
                } else {
                    tracing::error!(
                        "Set ALLOW_MIGRATION_REPAIR=1 to sync the checksum for version {}",
                        version
                    );
                    Err(err.into())
                }
            } else {
                tracing::error!("Failed to run database migrations: {}", err);
                Err(err.into())
            }
        }
    }
}

fn allow_migration_repair() -> bool {
    if let Ok(val) = std::env::var("ALLOW_MIGRATION_REPAIR") {
        let lower = val.trim().to_lowercase();
        return matches!(lower.as_str(), "true" | "1" | "yes");
    }

    std::env::var("RAILWAY_ENVIRONMENT").is_ok()
        || std::env::var("RAILWAY_PROJECT_ID").is_ok()
}

async fn repair_migration_checksum(
    pool: &PgPool,
    migrator: &sqlx::migrate::Migrator,
    version: i64,
) -> anyhow::Result<()> {
    let migration = migrator
        .iter()
        .find(|m| m.version == version)
        .ok_or_else(|| anyhow::anyhow!("Migration {} not found", version))?;

    let result = sqlx::query(
        r#"
        UPDATE _sqlx_migrations
        SET checksum = $1
        WHERE version = $2
        "#,
    )
    .bind(migration.checksum.as_ref())
    .bind(version)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        tracing::warn!("No migration row updated for version {}", version);
    }

    Ok(())
}

/// #2 WOW Feature: Smart Reminders - надсилає чекіни користувачам у їхній вибраний час
struct DueReminder {
    user_id: Uuid,
    telegram_id: i64,
    local_date: NaiveDate,
}

async fn send_smart_reminders(state: &SharedState, users: Vec<DueReminder>) -> anyhow::Result<()> {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")?;
    let bot = teloxide::Bot::new(bot_token);

    let mut success_count = 0;
    let mut error_count = 0;

    for due in users {
        let chat_id = teloxide::types::ChatId(due.telegram_id);

        match bot::enhanced_handlers::start_daily_checkin(&bot, state, chat_id, due.user_id).await {
            Ok(_) => {
                success_count += 1;
                tracing::debug!(
                    "Sent smart reminder to user {} (telegram: {})",
                    due.user_id,
                    due.telegram_id
                );
            }
            Err(e) => {
                error_count += 1;
                tracing::error!(
                    "Failed to send smart reminder to user {} (telegram: {}): {}",
                    due.user_id,
                    due.telegram_id,
                    e
                );
                if let Err(clear_err) =
                    db::clear_reminder_sent(&state.pool, due.user_id, due.local_date).await
                {
                    tracing::warn!(
                        "Failed to clear reminder marker for {}: {}",
                        due.user_id,
                        clear_err
                    );
                }
            }
        }

        // Rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(35)).await;
    }

    if success_count > 0 || error_count > 0 {
        tracing::info!(
            "Smart reminders sent: {} successful, {} failed",
            success_count,
            error_count
        );
    }

    Ok(())
}

/// Надсилає щоденні чекіни всім користувачам (окрім адмінів)
async fn send_daily_checkins_to_all(state: &SharedState) -> anyhow::Result<()> {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")?;
    let bot = teloxide::Bot::new(bot_token);

    // Отримати всіх користувачів з telegram_id (окрім ADMIN ролі)
    let rows = sqlx::query(
        r#"
        SELECT id, telegram_id
        FROM users
        WHERE telegram_id IS NOT NULL
          AND role != 'ADMIN'
          AND is_active = true
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    tracing::info!("Broadcasting daily check-ins to {} users", rows.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for row in rows {
        let user_id: Uuid = row.try_get("id")?;
        let telegram_id: Option<i64> = row.try_get("telegram_id")?;
        if let Some(telegram_id) = telegram_id {
            let chat_id = teloxide::types::ChatId(telegram_id);

            match bot::enhanced_handlers::start_daily_checkin(&bot, state, chat_id, user_id).await {
                Ok(_) => {
                    success_count += 1;
                    tracing::debug!(
                        "Sent check-in to user {} (telegram: {})",
                        user_id,
                        telegram_id
                    );
                }
                Err(e) => {
                    error_count += 1;
                    tracing::error!(
                        "Failed to send check-in to user {} (telegram: {}): {}",
                        user_id,
                        telegram_id,
                        e
                    );
                }
            }

            // Rate limiting - 30 messages per second max (Telegram limit)
            tokio::time::sleep(tokio::time::Duration::from_millis(35)).await;
        }
    }

    tracing::info!(
        "Daily check-in broadcast finished: {} successful, {} failed",
        success_count,
        error_count
    );

    Ok(())
}
