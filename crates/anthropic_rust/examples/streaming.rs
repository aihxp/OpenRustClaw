//! Streaming completion example using anthropic-rust.
//!
//! Run with:
//! ```bash
//! ANTHROPIC_API_KEY=your-key cargo run --example streaming --features streaming
//! ```

use futures::StreamExt;

use anthropic_rust::{AnthropicClient, MessageRequest, StreamEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key from environment
    let api_key =
        std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");

    // Create client
    let client = AnthropicClient::new(api_key)?;

    // Create a streaming request
    let request = MessageRequest::builder("claude-3-5-sonnet-20241022")
        .system("You are a creative writing assistant.")
        .user("Write a haiku about programming in Rust.")
        .max_tokens(1024)
        .stream(true)
        .build();

    println!("Streaming response:\n");

    // Stream the response
    let messages = client.messages();
    let mut stream = messages.stream(request).await?;

    while let Some(result) = stream.next().await {
        match result? {
            StreamEvent::MessageStart { message } => {
                println!("[Started message: {}]\n", message.id);
            }
            StreamEvent::ContentBlockDelta { delta, .. } => {
                use anthropic_rust::types::ContentBlockDelta;
                match delta {
                    ContentBlockDelta::TextDelta(t) => print!("{}", t.text),
                    ContentBlockDelta::PartialJson { partial_json } => {
                        print!("{}", partial_json)
                    }
                }
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                println!("\n[Content block {} started: {:?}]", index, content_block);
            }
            StreamEvent::ContentBlockStop { index } => {
                println!("\n[Content block {} finished]", index);
            }
            StreamEvent::MessageDelta { delta, usage } => {
                if let Some(reason) = delta.stop_reason {
                    println!("\n\n[Stopped: {}]", reason);
                }
                if let Some(u) = usage {
                    println!(
                        "\n[Usage: {} input, {} output tokens]",
                        u.input_tokens, u.output_tokens
                    );
                }
            }
            StreamEvent::MessageStop => {
                println!("\n[Message complete]");
                break;
            }
            StreamEvent::Ping => {
                // Heartbeat, ignore
            }
        }
    }

    println!();
    Ok(())
}
