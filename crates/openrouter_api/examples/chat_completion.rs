//! Simple chat completion example.
//!
//! Run with:
//! ```bash
//! OPENROUTER_API_KEY=your-key cargo run --example chat_completion
//! ```

use openrouter_api::{OpenRouterClient, ChatRequest, RouteStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    let client = OpenRouterClient::new(api_key)?;

    let request = ChatRequest::builder("anthropic/claude-3.5-sonnet")
        .system("You are a helpful assistant.")
        .user("What is the capital of France?")
        .route_strategy(RouteStrategy::Quality)
        .build();

    println!("Sending request...\n");

    let response = client.chat().complete(request).await?;

    println!("Response ID: {}", response.id);
    println!("Model: {}", response.model);
    println!("\nContent:\n{}", response.content());

    Ok(())
}
