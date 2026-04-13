use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{
        header::{RETRY_AFTER},
        HeaderValue, Request, StatusCode,
    },
    middleware::Next,
    response::Response,
    response::IntoResponse,
    Json,
};
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Simple in-memory IP rate limiter for Axum.
///
/// Keeps a rolling window of request timestamps per client IP:
/// - allow up to `max_requests` in `window`
/// - reject with HTTP 429 otherwise
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Inner>,
}

struct Inner {
    max_requests: usize,
    window: Duration,
    per_ip: Mutex<HashMap<IpAddr, VecDeque<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                max_requests: max_requests as usize,
                window,
                per_ip: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Returns `Some(retry_after)` when the IP is currently rate-limited.
    pub fn check_and_record(&self, ip: IpAddr) -> Option<Duration> {
        let now = Instant::now();
        let mut per_ip = self
            .inner
            .per_ip
            .lock()
            .expect("rate limiter per_ip mutex poisoned");

        let q = per_ip.entry(ip).or_insert_with(VecDeque::new);

        // Remove timestamps outside the rolling window.
        while let Some(&front) = q.front() {
            if now.duration_since(front) >= self.inner.window {
                q.pop_front();
            } else {
                break;
            }
        }

        if q.len() >= self.inner.max_requests {
            // We are at/over the limit; compute when the oldest timestamp will expire.
            let oldest = *q.front().expect("q len >= max implies front exists");
            let elapsed = now.duration_since(oldest);
            let retry_after = self
                .inner
                .window
                .checked_sub(elapsed)
                .unwrap_or_else(|| Duration::from_secs(0));
            return Some(retry_after);
        }

        q.push_back(now);
        None
    }
}

/// Axum middleware: rate limit all incoming requests by the client IP.
///
/// Note: this middleware uses `ConnectInfo<SocketAddr>` from request extensions.
/// If it is missing (e.g. some unit tests), it falls back to `0.0.0.0`.
pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    if let Some(retry_after) = limiter.check_and_record(ip) {
        let retry_secs = retry_after.as_secs();
        let mut res = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": "rate limit exceeded" })),
        )
            .into_response();

        // Retry-After is optional; only set when we can provide a positive value.
        if retry_secs > 0 {
            res.headers_mut().insert(
                RETRY_AFTER,
                HeaderValue::from_str(&retry_secs.to_string()).unwrap_or(HeaderValue::from_static("1")),
            );
        }
        return res;
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn rate_limiter_allows_max_then_denies_then_recovers() {
        let limiter = RateLimiter::new(2, Duration::from_millis(120));
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);

        assert_eq!(limiter.check_and_record(ip), None);
        assert_eq!(limiter.check_and_record(ip), None);
        assert!(limiter.check_and_record(ip).is_some());

        thread::sleep(Duration::from_millis(150));
        assert_eq!(limiter.check_and_record(ip), None);
    }
}

