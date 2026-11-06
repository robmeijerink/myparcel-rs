use std::fmt;

use serde::Deserialize;
use serde_json::Value;

use crate::shipments::response::Price;
use crate::types::ShipmentId;

/// Result type used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

const MAX_BODY_CHARS: usize = 500;

/// Errors returned by the client.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API answered with a non-success status.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// The request could not be sent or the response could not be read.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// The response body did not have the expected shape.
    #[error("failed to decode response: {source}")]
    Decode {
        /// Underlying deserialization error.
        source: serde_json::Error,
        /// The (possibly truncated) response body. It may contain personal data, so
        /// think twice before logging it.
        body: String,
    },
    /// A value was rejected before anything was sent.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    /// The API asked for a payment instead of returning the requested labels.
    #[error("the account has to pay before labels can be printed")]
    PaymentRequired(Vec<PaymentInstruction>),
    /// The API holds back the labels of some shipments because they are unpaid or blocked.
    #[error(
        "labels are held back: {} unpaid and {} blocked shipment(s)",
        .0.unpaid.len(),
        .0.blocked.len()
    )]
    LabelsHeldBack(HeldBackShipments),
    /// The API answered successfully, but not with what was asked for.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(&'static str),
}

impl Error {
    /// Returns the API error, if this is one.
    pub fn as_api(&self) -> Option<&ApiError> {
        match self {
            Error::Api(error) => Some(error),
            _ => None,
        }
    }

    /// Whether the API key was missing or invalid.
    pub fn is_unauthorized(&self) -> bool {
        self.as_api().is_some_and(|error| error.status == 401)
    }

    /// Whether the requested object does not exist.
    pub fn is_not_found(&self) -> bool {
        self.as_api().is_some_and(|error| error.status == 404)
    }
}

/// A value that was rejected because it does not satisfy the API's constraints.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid {field}: {message}")]
#[non_exhaustive]
pub struct ValidationError {
    /// What was being validated.
    pub field: &'static str,
    /// Why the value was rejected.
    pub message: String,
}

impl ValidationError {
    pub(crate) fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

/// What a shipment costs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ShipmentCharge {
    /// The shipment.
    pub shipment_id: ShipmentId,
    /// The price without VAT.
    pub price: Price,
    /// The price with VAT.
    pub price_after_vat: Price,
}

/// The shipments whose labels the API does not release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HeldBackShipments {
    /// Shipments that have to be paid first.
    pub unpaid: Vec<ShipmentCharge>,
    /// Shipments that are blocked.
    pub blocked: Vec<ShipmentCharge>,
}

/// A payment the account has to complete before labels are released.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PaymentInstruction {
    /// Identifier of the payment.
    pub id: String,
    /// Hash of the payment.
    pub hash: String,
    /// Where the payment can be completed, when the API supplies a link.
    pub payment_url: Option<String>,
    /// Whether the payment was already completed.
    #[serde(deserialize_with = "crate::serde_util::int_bool")]
    pub paid: bool,
}

/// An error response from the MyParcel API.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ApiError {
    /// HTTP status code.
    pub status: u16,
    /// Human readable message.
    pub message: String,
    /// Correlation id, useful when contacting MyParcel.
    pub request_id: Option<String>,
    /// Per-field or per-item problems reported by the API.
    pub details: Vec<ApiErrorDetail>,
}

/// One problem listed in an API error response.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ApiErrorDetail {
    /// Numeric or URN error code.
    pub code: Option<String>,
    /// Short summary.
    pub title: Option<String>,
    /// Longer explanation.
    pub message: Option<String>,
    /// JSON pointer of the offending field, when known.
    pub pointer: Option<String>,
}

impl ApiError {
    pub(crate) fn from_response(status: u16, body: &[u8]) -> Self {
        let value: Option<Value> = serde_json::from_slice(body).ok();
        let Some(value) = value.filter(Value::is_object) else {
            return Self {
                status,
                message: truncate(&String::from_utf8_lossy(body)),
                request_id: None,
                details: Vec::new(),
            };
        };

        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let message = text("message")
            .or_else(|| text("detail"))
            .or_else(|| text("title"))
            .unwrap_or_default();
        let details = match value.get("errors") {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(ApiErrorDetail::from_value)
                .collect(),
            Some(Value::Object(map)) => map
                .values()
                .filter_map(ApiErrorDetail::from_value)
                .collect(),
            _ => Vec::new(),
        };

        Self {
            status,
            message,
            request_id: text("request_id"),
            details,
        }
    }
}

impl ApiErrorDetail {
    fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let text = |key: &str| {
            object.get(key).and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
        };
        Some(Self {
            code: text("code").or_else(|| text("type")),
            title: text("title"),
            message: text("message").or_else(|| text("detail")),
            pointer: text("instance"),
        })
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "myparcel api error (status {}", self.status)?;
        if let Some(request_id) = &self.request_id {
            write!(f, ", request {request_id}")?;
        }
        write!(f, "): {}", self.message)?;
        for detail in &self.details {
            if let Some(message) = detail.message.as_ref().or(detail.title.as_ref()) {
                write!(f, "; {message}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

pub(crate) fn truncate(text: &str) -> String {
    text.chars().take(MAX_BODY_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_envelope() {
        let body = br#"{"message":"Invalid","request_id":"abc","status_code":422,"errors":[{"code":3212,"title":"T","message":"bad street"}]}"#;
        let error = ApiError::from_response(422, body);
        assert_eq!(error.message, "Invalid");
        assert_eq!(error.request_id.as_deref(), Some("abc"));
        assert_eq!(error.details[0].code.as_deref(), Some("3212"));
        assert_eq!(
            error.to_string(),
            "myparcel api error (status 422, request abc): Invalid; bad street"
        );
    }

    #[test]
    fn parses_problem_details() {
        let body = br#"{"type":"urn:x","title":"Bad","status":400,"detail":"nope","errors":[{"type":"urn:y","title":"f","detail":"d","instance":"/a"}]}"#;
        let error = ApiError::from_response(400, body);
        assert_eq!(error.message, "nope");
        assert_eq!(error.details[0].pointer.as_deref(), Some("/a"));
    }

    #[test]
    fn keeps_object_shaped_errors() {
        let body = br#"{"message":"m","errors":{"a":{"code":1,"message":"x"}}}"#;
        assert_eq!(ApiError::from_response(400, body).details.len(), 1);
    }

    #[test]
    fn falls_back_to_truncated_raw_body() {
        let body = "x".repeat(2000);
        let error = ApiError::from_response(502, body.as_bytes());
        assert_eq!(error.message.chars().count(), MAX_BODY_CHARS);
    }
}
