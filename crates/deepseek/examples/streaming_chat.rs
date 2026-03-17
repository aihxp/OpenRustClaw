//! Streaming chat completion example
//!
//! Run with: cargo run --example streaming_chat

use deepseek::{ChatRequest, DeepSeekClient, Role};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY environment variable must be set");

    // Create client
    let client = DeepSeekClient::new(api_key)?;

    // Create a chat request with streaming enabled
    let request = ChatRequest::builder("deepseek-chat")
        .system("You are a helpful assistant.")
        .message(Role::User, "Write a short poem about programming in Rust.")
        .temperature(0.8)
        .build();

    // Send the streaming request
    println!("Streaming response:\n");
    let mut stream = client.chat().stream(request).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                // Check for content
                if let Some(content) = chunk.content() {
                    print!("{}", content);
                }

                // Check for finish reason
                if let Some(finish_reason) = chunk.finish_reason() {
                    println!("\n\n[Finished: {}]", finish_reason);
                }
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }

    println!();
    Ok(())
}
