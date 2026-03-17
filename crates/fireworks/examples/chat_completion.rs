//! Chat completion example for Fireworks AI.

use fireworks_ai::{ChatRequest, FireworksClient, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("FIREWORKS_API_KEY")
        .expect("FIREWORKS_API_KEY environment variable must be set");

    // Create client
    let client = FireworksClient::new(api_key)?;
    println!("Created Fireworks client (base_url: {})", client.base_url());

    // Simple chat completion
    let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
        .system("You are a helpful AI assistant.")
        .user("What is the capital of France?")
        .max_tokens(100)
        .temperature(0.7)
        .build();

    println!("\nSending chat completion request...");
    let response = client.chat().complete(request).await?;
    
    println!("Response ID: {}", response.id);
    println!("Model: {}", response.model);
    println!("Content: {}", response.content());
    println!(
        "Usage: {} prompt, {} completion, {} total tokens",
        response.usage.prompt_tokens,
        response.usage.completion_tokens,
        response.usage.total_tokens
    );

    // Multi-turn conversation
    println!("\n--- Multi-turn Conversation ---");
    let request = ChatRequest::builder("accounts/fireworks/models/llama-v3p1-8b-instruct")
        .system("You are a helpful AI assistant.")
        .user("My name is Alice.")
        .assistant("Hello Alice! Nice to meet you. How can I help you today?")
        .user("What's my name?")
        .max_tokens(50)
        .build();

    let response = client.chat().complete(request).await?;
    println!("Content: {}", response.content());

    Ok(())
}
