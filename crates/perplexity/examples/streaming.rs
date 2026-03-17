//! Streaming chat completion example.
//!
//! Run with:
//! ```bash
//! PERPLEXITY_API_KEY=your_key cargo run --example streaming --features streaming
//! ```

use futures::StreamExt;
use perplexity::{ChatRequest, PerplexityClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("PERPLEXITY_API_KEY")
        .expect("PERPLEXITY_API_KEY environment variable must be set");

    let client = PerplexityClient::new(api_key)?;

    println!("=== Streaming Chat Example ===\n");

    let request = ChatRequest::builder("llama-3.1-sonar-small-128k-online")
        .system("You are a helpful assistant. Provide concise answers.")
        .user("What are the key benefits of using Rust for systems programming?")
        .temperature(0.7)
        .build();

    println!("Streaming response:\n");

    let mut stream = client.chat().complete_stream(request).await?;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Check if this is the final chunk
                if chunk.is_final() {
                    println!(); // New line after content

                    // Print finish reason
                    if let Some(reason) = chunk.finish_reason() {
                        println!("\n[Finish reason: {:?}]", reason);
                    }

                    // Citations appear in the final chunk
                    if chunk.has_citations() {
                        println!("\n=== Citations ===");
                        for citation in chunk.citations() {
                            if let Some(title) = &citation.title {
                                println!("[{}] {} - {}", citation.index, title, citation.url);
                            } else {
                                println!("[{}] {}", citation.index, citation.url);
                            }
                        }
                    }

                    // Related questions also appear in final chunk
                    if chunk.has_related_questions() {
                        println!("\n=== Related Questions ===");
                        for (i, question) in chunk.related_questions().iter().enumerate() {
                            println!("{}. {}", i + 1, question.question);
                        }
                    }

                    break;
                }

                // Print content delta
                if let Some(content) = chunk.delta_content() {
                    print!("{}", content);
                    // Flush stdout for real-time display
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
            }
            Err(e) => {
                eprintln!("\nStream error: {}", e);
                break;
            }
        }
    }

    println!("\n\nStream complete.");

    Ok(())
}
