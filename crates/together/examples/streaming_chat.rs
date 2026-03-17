//! Example: Streaming chat completion with Together AI.
//!
//! Run with:
//! ```
//! TOGETHER_API_KEY=your-api-key cargo run --example streaming_chat --features streaming
//! ```

use futures::StreamExt;
use together_ai::{ChatRequest, TogetherClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("TOGETHER_API_KEY")
        .expect("TOGETHER_API_KEY environment variable not set");

    // Create client
    let client = TogetherClient::new(api_key)?;
    println!("✅ Connected to Together AI\n");

    // Streaming chat
    println!("📝 Streaming chat completion:");
    let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
        .system("You are a creative storyteller.")
        .user("Write a 3-sentence story about a robot learning to paint.")
        .max_tokens(200)
        .temperature(0.8)
        .build();

    println!("🤖 Response: ");
    
    let stream = client.chat().complete_stream(request).await?;
    futures::pin_mut!(stream);

    let mut full_response = String::new();
    
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                let content = chunk.content();
                print!("{}", content);
                std::io::Write::flush(&mut std::io::stdout())?;
                full_response.push_str(content);
                
                if chunk.is_final() {
                    if let Some(reason) = chunk.finish_reason() {
                        println!("\n\n[Finished: {}]", reason);
                    }
                }
            }
            Err(e) => {
                eprintln!("\n⚠️ Stream error: {}", e);
                break;
            }
        }
    }

    println!("\n📊 Full response length: {} characters", full_response.len());

    // Streaming with early termination
    println!("\n📝 Streaming with early termination (first 50 chars):");
    let request = ChatRequest::builder("meta-llama/Llama-3-8b-chat-hf")
        .user("Write a poem about nature.")
        .max_tokens(500)
        .build();

    let stream = client.chat().complete_stream(request).await?;
    futures::pin_mut!(stream);

    let mut count = 0;
    while let Some(chunk) = stream.next().await {
        if let Ok(chunk) = chunk {
            let content = chunk.content();
            print!("{}", content);
            std::io::Write::flush(&mut std::io::stdout())?;
            count += content.len();
            
            if count >= 50 {
                println!("\n\n[Stopped after 50 characters]");
                break;
            }
        }
    }

    println!("\n✨ Done!");
    Ok(())
}
