//! Simple example of using the AWS Bedrock Converse API.
//!
//! This example demonstrates how to:
//! - Create a Bedrock client
//! - Send a message to Claude
//! - Handle the response
//!
//! Run with:
//! ```bash
//! export AWS_ACCESS_KEY_ID=your_access_key
//! export AWS_SECRET_ACCESS_KEY=your_secret_key
//! export AWS_REGION=us-east-1
//! cargo run --example simple_converse
//! ```

use aws_bedrock::{BedrockClient, ConverseRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Create a Bedrock client
    // This will use the default credential chain:
    // 1. Environment variables
    // 2. Shared credentials file (~/.aws/credentials)
    // 3. Container credentials
    // 4. EC2 instance metadata
    let client = BedrockClient::new("us-east-1").await?;

    println!("Bedrock client created successfully");
    println!("Region: {}", client.region());
    println!("Runtime endpoint: {}", client.runtime_endpoint());

    // Create a simple conversation request
    let request = ConverseRequest::builder("anthropic.claude-3-haiku-20240307-v1:0")
        .user_message("What are three interesting facts about Rust programming?")
        .max_tokens(500)
        .temperature(0.7)
        .build();

    println!("\nSending request to Claude...\n");

    // Send the request and get the response
    let response = client.converse().invoke(request).await?;

    // Print the response
    println!("Claude's response:");
    println!("==================");
    println!("{}", response.text());
    println!("==================\n");

    // Print usage statistics
    println!("Token usage:");
    println!("  Input tokens: {}", response.usage().input_tokens);
    println!("  Output tokens: {}", response.usage().output_tokens);
    println!("  Total tokens: {}", response.usage().total_tokens);
    println!("Latency: {}ms", response.latency_ms());
    println!("Stop reason: {:?}", response.stop_reason());

    Ok(())
}
