//! Simple chat completion example
//!
//! Run with: cargo run --example chat_completion

use deepseek::{ChatRequest, DeepSeekClient, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY environment variable must be set");

    // Create client
    let client = DeepSeekClient::new(api_key)?;

    // Create a chat request
    let request = ChatRequest::builder("deepseek-chat")
        .system("You are a helpful assistant.")
        .message(Role::User, "What are the main features of Rust?")
        .temperature(0.7)
        .max_tokens(500)
        .build();

    // Send the request
    println!("Sending request...\n");
    let response = client.chat().complete(request).await?;

    // Print the response
    if let Some(content) = response.content() {
        println!("Response:\n{}", content);
    }

    // Print token usage
    println!("\n--- Token Usage ---");
    println!("Prompt tokens: {}", response.usage.prompt_tokens);
    println!("Completion tokens: {}", response.usage.completion_tokens);
    println!("Total tokens: {}", response.usage.total_tokens);

    Ok(())
}
