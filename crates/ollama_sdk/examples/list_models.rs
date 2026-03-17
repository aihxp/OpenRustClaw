//! Example: Model management with Ollama
//!
//! Run with: cargo run --example list_models

use ollama_sdk::OllamaClient;

// Import StreamExt only when running the pull/push examples
#[allow(unused_imports)]
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Create client with default local Ollama instance
    let client = OllamaClient::new("http://localhost:11434");

    // Get Ollama version
    match client.models().version().await {
        Ok(version) => println!("Ollama version: {}\n", version),
        Err(e) => eprintln!("Failed to get version: {}", e),
    }

    // List all models
    println!("=== Local Models ===\n");
    match client.models().list().await {
        Ok(models) => {
            if models.is_empty() {
                println!("No models found. Pull a model with: ollama pull <model>");
            } else {
                for model in models {
                    let size_mb = model.size as f64 / 1_048_576.0;
                    println!("📦 {}", model.name);
                    println!("   Size: {:.1} MB", size_mb);

                    if let Some(details) = &model.details {
                        if let Some(family) = &details.family {
                            println!("   Family: {}", family);
                        }
                        if let Some(params) = &details.parameter_size {
                            println!("   Parameters: {}", params);
                        }
                        if let Some(quant) = &details.quantization_level {
                            println!("   Quantization: {}", quant);
                        }
                    }
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing models: {}", e);
        }
    }

    // List running models
    println!("=== Running Models ===\n");
    match client.models().running().await {
        Ok(models) => {
            if models.is_empty() {
                println!("No models currently running.");
            } else {
                for model in models {
                    let size_mb = model.size as f64 / 1_048_576.0;
                    let vram_mb = model.size_vram as f64 / 1_048_576.0;
                    println!("⚡ {}", model.name);
                    println!("   Size: {:.1} MB", size_mb);
                    println!("   VRAM: {:.1} MB", vram_mb);
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing running models: {}", e);
        }
    }

    // Example: Show model info for a specific model
    println!("=== Model Info Example ===\n");
    if let Ok(models) = client.models().list().await {
        if let Some(first_model) = models.first() {
            println!("Showing info for: {}\n", first_model.name);

            match client.models().show(&first_model.name).await {
                Ok(info) => {
                    if let Some(template) = info.template {
                        println!("Template: {} characters", template.len());
                    }
                    if let Some(system) = info.system {
                        println!("System prompt: {} characters", system.len());
                    }
                    if let Some(details) = info.details {
                        println!("Details: {:?}", details);
                    }
                }
                Err(e) => {
                    eprintln!("Error showing model info: {}", e);
                }
            }
        }
    }

    // Example: Pull a model (commented out as it takes time)
    // println!("\n=== Pull Model Example ===\n");
    // println!("Pulling llama3.2...");
    // let mut stream = client.models().pull("llama3.2").await?;
    // while let Some(status) = stream.next().await {
    //     match status {
    //         Ok(status) => {
    //             if let Some(progress) = status.progress() {
    //                 print!("\rProgress: {:.1}%", progress * 100.0);
    //             }
    //             if status.is_complete() {
    //                 println!("\n✅ Pull complete!");
    //             }
    //         }
    //         Err(e) => eprintln!("Error: {}", e),
    //     }
    // }

    Ok(())
}
