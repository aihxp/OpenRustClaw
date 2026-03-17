//! Streaming chat completion example for Fireworks AI.

use fireworks_ai::{ChatRequest, FireworksClient, Role};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("FIREWORKS_API_KEY")
        .expect("FIREWORKS_API_KEY environment variable must be set");

    // Create client
    let client = FireworksClient::new(api_key)?;
    println!("Created Fireworks client\n");

    // Streaming chat completion
    let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
        .system("You are a helpful AI assistant.")
        .user("Write a short poem about Rust programming.")
        .max_tokens(200)
        .temperature(0.8)
        .build();

    println!("Streaming response:");
    let mut stream = client.chat().complete_stream(request).await?;

    let mut full_content = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                if let Some(content) = chunk.content() {
                    print!("{}", content);
                    full_content.push_str(content);
                    std::io::Write::flush(&mut std::io::stdout())?;
                }

                if chunk.is_final() {
                    if let Some(reason) = chunk.finish_reason() {
                        println!("\n\n[Finished: {}]", reason);
                    }
                }
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }

    println!("\n\nFull content length: {} characters", full_content.len());

    Ok(())
}
