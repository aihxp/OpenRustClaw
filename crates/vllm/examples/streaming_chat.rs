//! Example: Streaming chat completion with vLLM

use futures::StreamExt;
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
        Ok(()) => println!("✓ vLLM server is healthy\n"),
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
        .message(Role::User, "Explain continuous batching in vLLM and why it improves throughput.")
        .max_tokens(300)
        .temperature(0.7)
        .build();

    println!("Sending streaming request to model: {}", model);
    println!("Prompt: Explain continuous batching in vLLM and why it improves throughput.\n");
    println!("Response (streaming):\n");

    // Send the streaming request
    let mut stream = client.chat().complete_stream(request).await?;

    let mut total_tokens = 0;

    // Process the stream
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                print!("{}", chunk.content());
                std::io::Write::flush(&mut std::io::stdout())?;

                if chunk.is_final() {
                    total_tokens = chunk.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0);
                }
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }

    println!("\n\n---");
    println!("Streaming complete! Generated approximately {} tokens", total_tokens);

    Ok(())
}
