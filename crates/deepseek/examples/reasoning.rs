//! Reasoning mode example using DeepSeek-R1
//!
//! Run with: cargo run --example reasoning
//!
//! This example demonstrates how to use DeepSeek-R1's reasoning capabilities.
//! The model provides both reasoning_content (chain-of-thought) and content (final answer).

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

    // Create a reasoning request using DeepSeek-R1
    let request = ChatRequest::builder("deepseek-reasoner")
        .message(
            Role::User,
            "A farmer has 17 sheep and all but 9 die. How many are left?",
        )
        .build();

    // Send the request
    println!("Sending reasoning request...\n");
    let response = client.chat().complete(request).await?;

    // Print the reasoning content (chain-of-thought)
    println!("=== Reasoning Process ===");
    if let Some(reasoning) = response.reasoning_content() {
        println!("{}", reasoning);
    } else {
        println!("(No reasoning content returned)");
    }

    // Print the final answer
    println!("\n=== Final Answer ===");
    if let Some(content) = response.content() {
        println!("{}", content);
    }

    // Print token usage
    println!("\n=== Token Usage ===");
    println!("Prompt tokens: {}", response.usage.prompt_tokens);
    println!("Completion tokens: {}", response.usage.completion_tokens);
    println!("Total tokens: {}", response.usage.total_tokens);

    // Print finish reason
    if let Some(finish_reason) = response.finish_reason() {
        println!("Finish reason: {}", finish_reason);
    }

    Ok(())
}
