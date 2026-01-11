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
use chrono::Timelike;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{services::ServeDir, services::ServeFile, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Custom middleware to add cache-busting headers
async fn add_cache_headers(req: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

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

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL missing");
    tracing::info!("Database URL configured");
    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to database: {}", e);
            e
        })?;
    tracing::info!("Database connection established");

    // Run database migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to run database migrations: {}", e);
            e
        })?;
    tracing::info!("Database migrations completed");

    let crypto = Arc::new(crypto::Crypto::from_env()?);
    let session_key_b64 = std::env::var("SESSION_KEY")
        .or_else(|_| std::env::var("APP_ENC_KEY"))
        .expect("SESSION_KEY missing");
    let session_key = general_purpose::STANDARD
        .decode(session_key_b64)
        .expect("SESSION_KEY must be base64");

    seed::seed_all(&pool, &crypto).await?;

    let ai = Arc::new(services::ai::AiService::new(
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY missing"),
        crypto.clone(),
    ));

    let poll_engine = domain::polling::PollEngine::new();
    let checkin_sessions = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let shared: SharedState = Arc::new(state::AppState {
        pool,
        crypto,
        ai,
        poll_engine,
        session_key,
        checkin_sessions,
    });

    // Setup scheduler for daily check-ins and weekly summaries
    let scheduler = JobScheduler::new().await?;

    // #2 WOW Feature: Smart Reminders - every minute check for users
    let shared_for_reminders = shared.clone();
    scheduler
        .add(Job::new_async("0 * * * * *", move |_uuid, _l| {
            let state = shared_for_reminders.clone();
            Box::pin(async move {
                let now = chrono::Utc::now();
                let hour = now.hour() as i16;
                let minute = (now.minute() as i16 / 15) * 15; // Round to 0, 15, 30, 45

                // Тільки на початку кожної 15-хвилинки
                if now.minute() as i16 % 15 == 0 {
                    match db::get_users_for_reminder_time(&state.pool, hour, minute).await {
                        Ok(users) => {
                            if !users.is_empty() {
                                tracing::info!(
                                    "Sending smart reminders to {} users at {:02}:{:02}",
                                    users.len(),
                                    hour,
                                    minute
                                );
                                if let Err(e) = send_smart_reminders(&state, users).await {
                                    tracing::error!("Failed to send smart reminders: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to get users for reminder time: {}", e);
                        }
                    }
                }
            })
        })?)
        .await?;

    // Legacy: Daily check-ins at 10:00 AM for users without custom time (fallback)
    let shared_for_scheduler = shared.clone();
    scheduler
        .add(Job::new_async("0 0 10 * * *", move |_uuid, _l| {
            let state = shared_for_scheduler.clone();
            Box::pin(async move {
                tracing::info!("Starting default 10:00 AM check-in broadcast...");
                if let Err(e) = send_daily_checkins_to_all(&state).await {
                    tracing::error!("Failed to send daily check-ins: {}", e);
                } else {
                    tracing::info!("Daily check-in broadcast completed successfully");
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
            })
        })?)
        .await?;

    scheduler.start().await?;
    tracing::info!("Scheduler started:");
    tracing::info!("  - Smart reminders: every 15 min");
    tracing::info!("  - Default check-ins: 10:00 AM daily");
    tracing::info!("  - Weekly summaries: Fridays 17:00");
    tracing::info!("  - Session cleanup: hourly");

    let static_handler = ServeDir::new("static").not_found_service(ServeFile::new("index.html"));

    let app = Router::new()
        .merge(web::routes(shared.clone()))
        .merge(bot::enhanced_handlers::routes(shared.clone()))
        .nest_service("/static", get_service(ServeDir::new("static")))
        .fallback_service(get_service(static_handler))
        .layer(axum_middleware::from_fn(add_cache_headers))
        .layer(TraceLayer::new_for_http());

    // Railway sets PORT automatically, prefer it over BIND_ADDR
    let port = std::env::var("PORT").unwrap_or_else(|_| {
        std::env::var("BIND_ADDR")
            .ok()
            .and_then(|addr| addr.split(':').nth(1).map(|p| p.to_string()))
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
    axum::serve(listener, app).await?;
    Ok(())
}

/// #2 WOW Feature: Smart Reminders - надсилає чекіни користувачам у їхній вибраний час
async fn send_smart_reminders(
    state: &SharedState,
    users: Vec<(uuid::Uuid, i64)>,
) -> anyhow::Result<()> {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")?;
    let bot = teloxide::Bot::new(bot_token);

    let mut success_count = 0;
    let mut error_count = 0;

    for (user_id, telegram_id) in users {
        let chat_id = teloxide::types::ChatId(telegram_id);

        match bot::enhanced_handlers::start_daily_checkin(&bot, state, chat_id, user_id).await {
            Ok(_) => {
                success_count += 1;
                tracing::debug!(
                    "Sent smart reminder to user {} (telegram: {})",
                    user_id,
                    telegram_id
                );
            }
            Err(e) => {
                error_count += 1;
                tracing::error!(
                    "Failed to send smart reminder to user {} (telegram: {}): {}",
                    user_id,
                    telegram_id,
                    e
                );
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
    let users = sqlx::query!(
        r#"
        SELECT id, telegram_id
        FROM users
        WHERE telegram_id IS NOT NULL
          AND role != 'ADMIN'
        "#
    )
    .fetch_all(&state.pool)
    .await?;

    tracing::info!("Broadcasting daily check-ins to {} users", users.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for user in users {
        if let Some(telegram_id) = user.telegram_id {
            let chat_id = teloxide::types::ChatId(telegram_id);

            match bot::enhanced_handlers::start_daily_checkin(&bot, state, chat_id, user.id).await {
                Ok(_) => {
                    success_count += 1;
                    tracing::debug!(
                        "Sent check-in to user {} (telegram: {})",
                        user.id,
                        telegram_id
                    );
                }
                Err(e) => {
                    error_count += 1;
                    tracing::error!(
                        "Failed to send check-in to user {} (telegram: {}): {}",
                        user.id,
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
