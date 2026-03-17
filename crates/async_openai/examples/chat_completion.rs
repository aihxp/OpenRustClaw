//! Simple chat completion example.
//!
//! Run with:
//! ```bash
//! OPENAI_API_KEY=your-key cargo run --example chat_completion
//! ```

use async_openai::{OpenAIClient, ChatRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable not set");

    let client = OpenAIClient::new(api_key)?;

    let request = ChatRequest::builder("gpt-4o-mini")
        .system("You are a helpful assistant.")
        .user("What is Rust?")
        .temperature(0.7)
        .build();

    println!("Sending request...\n");

    let response = client.chat().complete(request).await?;

    println!("Response ID: {}", response.id);
    println!("Model: {}", response.model);
    println!("\nContent:\n{}", response.content().unwrap_or("No content"));
    println!("\nUsage: {} tokens", response.usage.total_tokens);

    Ok(())
}
