//! Example: Chat completion with Together AI.
//!
//! Run with:
//! ```
//! TOGETHER_API_KEY=your-api-key cargo run --example chat_completion
//! ```

use together_ai::{ChatRequest, TogetherClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key =
        std::env::var("TOGETHER_API_KEY").expect("TOGETHER_API_KEY environment variable not set");

    // Create client
    let client = TogetherClient::new(api_key)?;
    println!("✅ Connected to Together AI");

    // Simple completion
    println!("\n📝 Simple chat completion:");
    let request = ChatRequest::simple(
        "meta-llama/Llama-3-8b-chat-hf",
        "What is the capital of France?",
    );

    let response = client.chat().complete(request).await?;
    println!("🤖 Response: {}", response.content());
    println!(
        "📊 Tokens used: {} prompt, {} completion, {} total",
        response.usage.prompt_tokens, response.usage.completion_tokens, response.usage.total_tokens
    );

    // Multi-turn conversation with system prompt
    println!("\n💬 Multi-turn conversation:");
    let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
        .system("You are a helpful coding assistant. Provide concise answers.")
        .user("How do I reverse a string in Rust?")
        .max_tokens(300)
        .temperature(0.3)
        .build();

    let response = client.chat().complete(request).await?;
    println!("🤖 Response:\n{}", response.content());

    // JSON mode example
    println!("\n📋 JSON mode example:");
    let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
        .system("You are a helpful assistant that outputs valid JSON.")
        .user("List 3 programming languages with their creators.")
        .json_mode()
        .max_tokens(200)
        .build();

    let response = client.chat().complete(request).await?;
    println!("🤖 JSON Response:\n{}", response.content());

    println!("\n✨ Done!");
    Ok(())
}
