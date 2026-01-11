///! Simple in-memory rate limiter for anonymous endpoints
///! Production: use Redis or dedicated service like Cloudflare
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Check if request is allowed for given identifier (IP, user_id, etc.)
    pub async fn check(&self, identifier: &str) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.write().await;

        // Get or create request history for this identifier
        let history = requests
            .entry(identifier.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests outside the window
        history.retain(|&timestamp| now.duration_since(timestamp) < self.window);

        // Check if under limit
        if history.len() < self.max_requests {
            history.push(now);
            true
        } else {
            false
        }
    }

    /// Cleanup old entries (call periodically)
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut requests = self.requests.write().await;

        requests.retain(|_, history| {
            history.retain(|&timestamp| now.duration_since(timestamp) < self.window);
            !history.is_empty()
        });

        tracing::debug!(
            "Rate limiter cleanup: {} active identifiers",
            requests.len()
        );
    }
}

/// Middleware for IP-based rate limiting
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    limiter: axum::extract::State<RateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let ip = addr.ip().to_string();

    if !limiter.check(&ip).await {
        tracing::warn!("Rate limit exceeded for IP: {}", ip);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many requests. Please try again later.",
        )
            .into_response();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, 60); // 3 requests per 60 seconds

        // First 3 requests should pass
        assert!(limiter.check("test_ip").await);
        assert!(limiter.check("test_ip").await);
        assert!(limiter.check("test_ip").await);

        // 4th request should be blocked
        assert!(!limiter.check("test_ip").await);

        // Different IP should work
        assert!(limiter.check("other_ip").await);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let limiter = RateLimiter::new(5, 1); // 5 requests per 1 second

        limiter.check("ip1").await;
        limiter.check("ip2").await;

        tokio::time::sleep(Duration::from_secs(2)).await;
        limiter.cleanup().await;

        // After cleanup, old requests should be removed
        let requests = limiter.requests.read().await;
        assert_eq!(requests.len(), 0);
    }
}
