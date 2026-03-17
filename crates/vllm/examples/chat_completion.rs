//! Example: Chat completion with vLLM

use vllm::{ChatRequest, Role, VllmClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create client
    let base_url = std::env::var("VLLM_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let client = VllmClient::new(&base_url)?;

    println!("Connected to vLLM at: {}", client.base_url());

    // Check health
    match client.health().await {
        Ok(()) => println!("✓ vLLM server is healthy"),
        Err(e) => {
            eprintln!("✗ Health check failed: {}", e);
            return Ok(());
        }
    }

    // Create a chat request
    let model = std::env::var("VLLM_MODEL")
        .unwrap_or_else(|_| "meta-llama/Meta-Llama-3-8B-Instruct".to_string());

    let request = ChatRequest::builder(&model)
        .system("You are a helpful AI assistant running on vLLM.")
        .message(Role::User, "What is PagedAttention and how does it help with LLM inference?")
        .max_tokens(256)
        .temperature(0.7)
        .build();

    println!("\nSending request to model: {}", model);
    println!("Prompt: What is PagedAttention and how does it help with LLM inference?\n");

    // Send the request
    let response = client.chat().complete(request).await?;

    // Print the response
    println!("Response:");
    println!("{}", response.content());
    println!("\n---");
    println!("Usage: {} prompt tokens, {} completion tokens (total: {})",
        response.usage.prompt_tokens,
        response.usage.completion_tokens,
        response.usage.total_tokens
    );

    // Check if the model used tool calls
    if response.has_tool_calls() {
        println!("\nTool calls:");
        for tool_call in response.tool_calls().unwrap() {
            println!("  - {}: {}", tool_call.function.name, tool_call.function.arguments);
        }
    }

    Ok(())
}
