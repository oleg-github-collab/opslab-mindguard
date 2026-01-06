///! Row Level Security (RLS) context middleware
///! Sets PostgreSQL session variables for RLS policies

use crate::db;
use crate::state::SharedState;
use crate::web::session::UserSession;
use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// Middleware to set RLS context for authenticated requests
/// Calls set_user_context(user_id, role) for every authenticated request
pub async fn set_rls_context(
    user_session: Option<UserSession>,
    State(state): State<SharedState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // If user is authenticated, set RLS context
    if let Some(UserSession(user_id)) = user_session {
        // Get user role from database
        let user = db::find_user_by_id(&state.pool, user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch user for RLS context: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(user) = user {
            // Set PostgreSQL session variables for RLS policies
            let role_str = format!("{:?}", user.role).to_uppercase();

            sqlx::query!(
                "SELECT set_user_context($1, $2)",
                user_id,
                role_str
            )
            .execute(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to set RLS context: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            tracing::debug!(
                "RLS context set: user_id={}, role={}",
                user_id,
                role_str
            );
        }
    }

    // Continue with request
    Ok(next.run(request).await)
}

/// Production-ready note:
/// This middleware should be applied BEFORE route handlers
/// Example in main.rs:
/// ```
/// let app = Router::new()
///     .route("/api/...", ...)
///     .layer(middleware::from_fn_with_state(state.clone(), rls::set_rls_context))
///     .with_state(state);
/// ```
