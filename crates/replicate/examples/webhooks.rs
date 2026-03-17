//! Example: Webhook verification
//!
//! Run with:
//!     REPLICATE_WEBHOOK_SECRET=your_secret cargo run --example webhooks

use replicate::webhooks::WebhookVerifier;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get webhook secret from environment
    let secret = std::env::var("REPLICATE_WEBHOOK_SECRET")
        .expect("REPLICATE_WEBHOOK_SECRET environment variable must be set");

    // Create verifier
    let verifier = WebhookVerifier::new(secret)?;

    // Example webhook data (normally this comes from the HTTP request)
    // Note: This is a fake example, real webhooks would have valid signatures
    let signature = "t=1234567890,v1=invalid_signature_for_demo";
    let body = br#"{"id": "pred_123", "status": "succeeded"}"#;

    // Verify webhook
    match verifier.verify(signature, body) {
        Ok(()) => println!("Webhook verified successfully!"),
        Err(e) => println!("Webhook verification failed (expected for demo): {}", e),
    }

    // Example: Parsing a webhook body
    let webhook_body = br#"{
        "id": "pred_abc123",
        "model": "black-forest-labs/flux-schnell",
        "status": "succeeded",
        "input": {"prompt": "A cat"},
        "output": ["https://example.com/image.png"],
        "created_at": "2024-01-01T00:00:00Z"
    }"#;

    let prediction = replicate::webhooks::parse_webhook_body(webhook_body)?;
    println!("\nParsed webhook:");
    println!("  ID: {}", prediction.id);
    println!("  Status: {:?}", prediction.status);
    println!("  Model: {:?}", prediction.model);

    Ok(())
}
