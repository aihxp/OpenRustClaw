//! Webhooks API - Verify and handle webhook events.

use hmac::Mac;
use sha2::Sha256;
use tracing::debug;

use crate::error::{ReplicateError, Result};

type HmacSha256 = hmac::SimpleHmac<Sha256>;

/// Webhook verifier for validating incoming webhook requests.
#[derive(Debug, Clone)]
pub struct WebhookVerifier {
    secret: String,
}

impl WebhookVerifier {
    /// Create a new webhook verifier with the given secret.
    ///
    /// The secret can be obtained from the Replicate dashboard or API.
    ///
    /// # Example
    ///
    /// ```
    /// use replicate::webhooks::WebhookVerifier;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let verifier = WebhookVerifier::new("whsec_...")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(secret: impl Into<String>) -> Result<Self> {
        let secret = secret.into();
        if secret.is_empty() {
            return Err(ReplicateError::Config {
                message: "Webhook secret cannot be empty".to_string(),
            });
        }
        Ok(Self { secret })
    }

    /// Verify a webhook request.
    ///
    /// # Arguments
    ///
    /// * `signature_header` - The value of the `Replicate-Signature` header
    /// * `body` - The raw request body bytes
    ///
    /// # Example
    ///
    /// ```
    /// use replicate::webhooks::WebhookVerifier;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let verifier = WebhookVerifier::new("whsec_...")?;
    ///
    /// let signature = "t=1234567890,v1=abc123...";
    /// let body = b"{\"id\": \"pred_123\", ...}";
    ///
    /// match verifier.verify(signature, body) {
    ///     Ok(()) => println!("Webhook verified!"),
    ///     Err(e) => println!("Invalid webhook: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn verify(&self, signature_header: &str, body: &[u8]) -> Result<()> {
        // Parse the signature header
        // Format: t=<timestamp>,v1=<signature>
        let parts: Vec<&str> = signature_header.split(',').collect();

        let mut timestamp = None;
        let mut signature = None;

        for part in parts {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() != 2 {
                continue;
            }

            match kv[0] {
                "t" => timestamp = Some(kv[1]),
                "v1" => signature = Some(kv[1]),
                _ => {}
            }
        }

        let timestamp = timestamp.ok_or_else(|| ReplicateError::Webhook {
            message: "Missing timestamp in signature".to_string(),
        })?;

        let signature = signature.ok_or_else(|| ReplicateError::Webhook {
            message: "Missing signature in signature header".to_string(),
        })?;

        // Construct the signed payload
        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(body));

        // Compute expected signature
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes()).map_err(|e| {
            ReplicateError::Webhook {
                message: format!("Failed to create HMAC: {}", e),
            }
        })?;
        mac.update(signed_payload.as_bytes());
        let result = mac.finalize();
        let expected_signature = hex::encode(result.into_bytes());

        // Compare signatures
        if !constant_time_eq(&expected_signature, signature) {
            return Err(ReplicateError::Webhook {
                message: "Invalid signature".to_string(),
            });
        }

        debug!("Webhook signature verified successfully");
        Ok(())
    }

    /// Verify a webhook request with a custom tolerance.
    ///
    /// This also checks if the timestamp is within the tolerance window.
    pub fn verify_with_tolerance(
        &self,
        signature_header: &str,
        body: &[u8],
        tolerance_seconds: u64,
    ) -> Result<()> {
        // Parse the signature header to get timestamp
        let parts: Vec<&str> = signature_header.split(',').collect();

        let mut timestamp = None;
        for part in parts {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() == 2 && kv[0] == "t" {
                timestamp = Some(kv[1]);
            }
        }

        let timestamp_str = timestamp.ok_or_else(|| ReplicateError::Webhook {
            message: "Missing timestamp in signature".to_string(),
        })?;

        let timestamp_ms: i64 = timestamp_str.parse().map_err(|_| ReplicateError::Webhook {
            message: "Invalid timestamp".to_string(),
        })?;

        let now = chrono::Utc::now().timestamp_millis();
        let tolerance_ms = (tolerance_seconds as i64) * 1000;

        if (now - timestamp_ms).abs() > tolerance_ms {
            return Err(ReplicateError::Webhook {
                message: "Timestamp outside tolerance window".to_string(),
            });
        }

        self.verify(signature_header, body)
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

/// A webhook event received from Replicate.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WebhookEvent {
    /// The event type (start, output, logs, completed).
    pub event: String,
    /// The prediction object.
    pub prediction: crate::types::Prediction,
}

/// Webhook event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookEventType {
    /// Prediction started.
    Start,
    /// New output generated.
    Output,
    /// New logs available.
    Logs,
    /// Prediction completed (succeeded, failed, or canceled).
    Completed,
}

impl WebhookEventType {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "start" => Some(Self::Start),
            "output" => Some(Self::Output),
            "logs" => Some(Self::Logs),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }
}

impl std::fmt::Display for WebhookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Output => write!(f, "output"),
            Self::Logs => write!(f, "logs"),
            Self::Completed => write!(f, "completed"),
        }
    }
}

/// Utility function to parse a webhook payload.
///
/// # Example
///
/// ```
/// use replicate::webhooks::parse_webhook_body;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let body = br#"{"id": "pred_123", "status": "succeeded", ...}"#;
/// let prediction = parse_webhook_body(body)?;
/// # Ok(())
/// # }
/// ```
pub fn parse_webhook_body(body: &[u8]) -> Result<crate::types::Prediction> {
    serde_json::from_slice(body).map_err(|e| ReplicateError::Webhook {
        message: format!("Failed to parse webhook body: {}", e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_type() {
        assert_eq!(
            WebhookEventType::parse("start"),
            Some(WebhookEventType::Start)
        );
        assert_eq!(
            WebhookEventType::parse("output"),
            Some(WebhookEventType::Output)
        );
        assert_eq!(
            WebhookEventType::parse("completed"),
            Some(WebhookEventType::Completed)
        );
        assert_eq!(WebhookEventType::parse("invalid"), None);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "def"));
        assert!(!constant_time_eq("ab", "abc"));
    }

    #[test]
    fn test_webhook_verifier_empty_secret() {
        let result = WebhookVerifier::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_verifier_invalid_signature_format() {
        let verifier = WebhookVerifier::new("test_secret").unwrap();

        // Missing timestamp
        let result = verifier.verify("v1=abc123", b"{}");
        assert!(result.is_err());

        // Missing signature
        let result = verifier.verify("t=1234567890", b"{}");
        assert!(result.is_err());
    }
}
