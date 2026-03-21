//! Per-request UUID and shared observability state in request extensions.

use http::header::{HeaderName, HeaderValue};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

/// HTTP header for correlation (`X-Request-ID`).
pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// UUID v4 attached to every request.
#[derive(Clone, Copy, Debug)]
pub struct RequestId(pub Uuid);

/// Mutable fields filled by handlers; read when the response body completes.
#[derive(Debug)]
pub struct ObservabilityFields {
    pub request_id: Uuid,
    pub path: String,
    pub start: Instant,
    pub function_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    /// `success` or `error` when set by handler (e.g. SSE error with HTTP 200).
    pub handler_status: Option<String>,
    pub error_type: Option<String>,
}

impl ObservabilityFields {
    pub fn new(request_id: Uuid, path: String) -> Self {
        Self {
            request_id,
            path,
            start: Instant::now(),
            function_id: None,
            provider: None,
            model: None,
            handler_status: None,
            error_type: None,
        }
    }
}

pub type ObservabilityShared = Arc<Mutex<ObservabilityFields>>;

/// Header value for `X-Request-ID`.
pub fn request_id_header_value(id: Uuid) -> HeaderValue {
    HeaderValue::from_str(&id.to_string()).expect("uuid is valid header")
}
