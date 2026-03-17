//! Basic usage example for Google Gemini SDK
//! 
//! Run with: cargo run --example basic_usage

use google_gemini::{GeminiClient, GeminiModel, Content, GenerateContentRequest, GenerationConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable must be set");
    
    // Create client
    let client = GeminiClient::new(api_key)
        .with_model(GeminiModel::Gemini15Flash);
    
    // Simple text generation
    println!("=== Simple Text Generation ===");
    let request = GenerateContentRequest {
        contents: vec![Content::user("Explain quantum computing in simple terms")],
        system_instruction: None,
        generation_config: Some(GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(500),
            ..Default::default()
        }),
        tools: None,
        tool_config: None,
        safety_settings: None,
    };
    
    match client.generate_content(request).await {
        Ok(response) => {
            for candidate in response.candidates {
                for part in candidate.content.parts {
                    if let google_gemini::types::Part::Text { text } = part {
                        println!("Response: {}", text);
                    }
                }
            }
            
            if let Some(usage) = response.usage_metadata {
                println!("\nToken usage: {} prompt, {} completion, {} total",
                    usage.prompt_token_count,
                    usage.candidates_token_count,
                    usage.total_token_count
                );
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    
    // Chat session
    println!("\n=== Chat Session ===");
    let mut chat = client
        .chat()
        .with_system_instruction("You are a helpful coding assistant.");
    
    match chat.send_message("What is Rust programming language?").await {
        Ok(response) => println!("Response: {}", response),
        Err(e) => eprintln!("Error: {}", e),
    }
    
    match chat.send_message("What are its key features?").await {
        Ok(response) => println!("Follow-up: {}", response),
        Err(e) => eprintln!("Error: {}", e),
    }
    
    println!("\nChat history has {} messages", chat.history().len());
    
    // List available models
    println!("\n=== Available Models ===");
    match client.list_models().await {
        Ok(models) => {
            for model in models.iter().take(5) {
                println!("- {}: {}", model.name, model.display_name);
            }
        }
        Err(e) => eprintln!("Error listing models: {}", e),
    }
    
    Ok(())
}
