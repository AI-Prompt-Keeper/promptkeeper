//! Request ID + inflight + completion metrics/logging on body end.

use crate::observability::metrics::label_or_unknown;
use crate::observability::request_id::{ObservabilityFields, ObservabilityShared, RequestId, X_REQUEST_ID};
use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::header::HeaderValue;
use http_body::Body as HttpBody;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use uuid::Uuid;

/// Panic-safe inflight decrement (gauge) when dropped after the response body completes.
pub struct InflightGuard;

impl Drop for InflightGuard {
    fn drop(&mut self) {
        metrics::gauge!("prke_inflight_requests").decrement(1.0);
    }
}

pub struct InstrumentedBody<B> {
    inner: Pin<Box<B>>,
    meta: ObservabilityShared,
    status: http::StatusCode,
    guard: Option<InflightGuard>,
    recorded: bool,
}

impl<B> InstrumentedBody<B> {
    pub fn new(inner: B, meta: ObservabilityShared, status: http::StatusCode, guard: InflightGuard) -> Self {
        Self {
            inner: Box::pin(inner),
            meta,
            status,
            guard: Some(guard),
            recorded: false,
        }
    }

    fn record_once(&mut self) {
        if self.recorded {
            return;
        }
        self.recorded = true;

        let duration_ms = {
            let g = match self.meta.lock() {
                Ok(x) => x,
                Err(p) => p.into_inner(),
            };
            g.start.elapsed().as_secs_f64() * 1000.0
        };

        let (request_id, path, function_id, provider, model, handler_status, error_type) = {
            let g = match self.meta.lock() {
                Ok(x) => x,
                Err(p) => p.into_inner(),
            };
            (
                g.request_id,
                g.path.clone(),
                g.function_id.clone(),
                g.provider.clone(),
                g.model.clone(),
                g.handler_status.clone(),
                g.error_type.clone(),
            )
        };

        let status_label = if self.status.is_server_error() || self.status.is_client_error() {
            "error"
        } else if let Some(ref h) = handler_status {
            if h == "error" {
                "error"
            } else {
                "success"
            }
        } else {
            "success"
        };

        let endpoint = path.clone();
        let prov = label_or_unknown(provider.as_deref());
        let mdl = label_or_unknown(model.as_deref());

        metrics::counter!(
            "prke_requests_total",
            "endpoint" => endpoint.clone(),
            "status" => status_label,
            "provider" => prov.clone(),
            "model" => mdl.clone(),
        )
        .increment(1);

        metrics::histogram!(
            "prke_request_duration_millis",
            "endpoint" => endpoint,
            "status" => status_label,
            "provider" => prov,
            "model" => mdl,
        )
        .record(duration_ms);

        let log_status = if status_label == "success" { "success" } else { "error" };
        tracing::info!(
            request_id = %request_id,
            function_id = %function_id.as_deref().unwrap_or(""),
            provider = %provider.as_deref().unwrap_or("unknown"),
            model = %model.as_deref().unwrap_or("unknown"),
            latency_ms = (duration_ms as u64),
            status = log_status,
            error_type = %error_type.as_deref().unwrap_or(""),
            "request completed"
        );

        drop(self.guard.take());
    }
}

impl<B> Drop for InstrumentedBody<B> {
    fn drop(&mut self) {
        if !self.recorded {
            self.record_once();
        }
        drop(self.guard.take());
    }
}

impl<B> HttpBody for InstrumentedBody<B>
where
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Send,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let poll = self.as_mut().get_mut().inner.as_mut().poll_frame(cx);
        match &poll {
            Poll::Ready(None) => {
                self.as_mut().get_mut().record_once();
            }
            Poll::Ready(Some(Err(_))) => {
                let slf = self.as_mut().get_mut();
                if let Ok(mut g) = slf.meta.lock() {
                    g.handler_status = Some("error".into());
                    if g.error_type.is_none() {
                        g.error_type = Some("unknown".into());
                    }
                }
            }
            _ => {}
        }
        poll
    }
}

/// Axum middleware: UUID, extensions, inflight gauge, response header, instrumented body.
pub async fn observability_middleware(mut req: Request, next: Next) -> Response {
    let id = Uuid::new_v4();
    let path = req.uri().path().to_string();
    let span = tracing::info_span!("request", request_id = %id, path = %path);
    let _e = span.enter();

    let state: ObservabilityShared = Arc::new(Mutex::new(ObservabilityFields::new(id, path)));
    req.extensions_mut().insert(RequestId(id));
    req.extensions_mut().insert(Arc::clone(&state));

    metrics::gauge!("prke_inflight_requests").increment(1.0);
    let guard = InflightGuard;

    let mut res = next.run(req).await;

    res.headers_mut()
        .insert(X_REQUEST_ID, HeaderValue::from_str(&id.to_string()).expect("uuid"));

    let status = res.status();
    let (parts, body) = res.into_parts();
    let wrapped = InstrumentedBody::new(body, state, status, guard);
    Response::from_parts(parts, Body::new(wrapped))
}
