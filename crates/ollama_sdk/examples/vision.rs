//! Example: Multi-modal vision with Ollama
//!
//! This example shows how to use vision-capable models like llava
//! to analyze images.
//!
//! Run with: cargo run --example vision -- <path_to_image>
//!
//! Note: Requires a vision-capable model like llava to be installed:
//!   ollama pull llava

use ollama_sdk::{ChatRequest, ImageInput, OllamaClient, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Get image path from command line or use default
    let image_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./examples/sample_image.jpg".to_string());

    // Check if image exists
    if !std::path::Path::new(&image_path).exists() {
        eprintln!("Error: Image '{}' not found.", image_path);
        eprintln!("Usage: cargo run --example vision -- <path_to_image>");
        std::process::exit(1);
    }

    // Create client with default local Ollama instance
    let client = OllamaClient::new("http://localhost:11434");

    println!("Loading image: {}\n", image_path);

    // Load and encode the image
    let image = ImageInput::from_path(&image_path);
    
    // Build a chat request with image
    let request = ChatRequest::builder("llava")
        .message_with_images(
            Role::User,
            "Describe what you see in this image in detail.",
            vec![image],
        )
        .build();

    println!("Sending vision request to Ollama (model: llava)...\n");

    // Send the request and get response
    match client.chat().generate(request).await {
        Ok(response) => {
            println!("Model: {}", response.model);
            println!("Response:\n{}\n", response.content());
            
            if let Some(eval_count) = response.eval_count {
                println!("Tokens generated: {}", eval_count);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nMake sure you have pulled the llava model:");
            eprintln!("  ollama pull llava");
            std::process::exit(1);
        }
    }

    // Example with multiple images
    println!("\n=== Multi-Image Example ===");
    println!("Note: This requires multiple images to be provided.\n");

    // For demonstration, we'd use multiple images like this:
    // let images = vec![
    //     ImageInput::from_path("image1.jpg"),
    //     ImageInput::from_path("image2.jpg"),
    // ];
    // let request = ChatRequest::builder("llava")
    //     .message_with_images(
    //         Role::User,
    //         "Compare these two images.",
    //         images,
    //     )
    //     .build();

    Ok(())
}
