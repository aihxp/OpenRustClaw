//! Example of using streaming with AWS Bedrock.
//!
//! This example demonstrates how to:
//! - Create a Bedrock client
//! - Stream responses from Claude
//! - Handle stream events in real-time
//!
//! Run with:
//! ```bash
//! export AWS_ACCESS_KEY_ID=your_access_key
//! export AWS_SECRET_ACCESS_KEY=your_secret_key
//! cargo run --example streaming
//! ```

use aws_bedrock::{BedrockClient, ConverseRequest, StreamEvent};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a Bedrock client
    let client = BedrockClient::new("us-east-1").await?;

    println!("Bedrock client created");
    println!("Starting streaming conversation...\n");

    // Create a streaming request
    let request = ConverseRequest::builder("anthropic.claude-3-haiku-20240307-v1:0")
        .user_message("Write a short poem about programming in Rust.")
        .max_tokens(500)
        .temperature(0.9)
        .build();

    println!("Claude is thinking...\n");
    println!("==================");

    // Stream the response
    let mut stream = client.converse().stream(request).await?;
    let mut full_text = String::new();

    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                if let Some(text) = delta.text() {
                    print!("{}", text);
                    full_text.push_str(text);
                    // Flush stdout for real-time display
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            }
            Ok(StreamEvent::MessageStop { .. }) => {
                println!("\n==================\n");
                println!("Stream complete!");
            }
            Ok(_) => {
                // Other events (message start, content block start/stop, metadata)
                // Can be handled if needed
            }
            Err(e) => {
                eprintln!("\nStream error: {}", e);
                return Err(e.into());
            }
        }
    }

    println!("\nFull response length: {} characters", full_text.len());

    Ok(())
}
