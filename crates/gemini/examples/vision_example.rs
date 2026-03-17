//! Vision example for Google Gemini SDK
//!
//! Run with: cargo run --example vision_example -- path/to/image.png

use google_gemini::vision::VisionExt;
use google_gemini::{GeminiClient, GeminiModel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable must be set");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --example vision_example -- <image_path>");
        std::process::exit(1);
    }

    let image_path = &args[1];
    let image_data = std::fs::read(image_path)?;

    let client = GeminiClient::new(api_key).with_model(GeminiModel::Gemini15Pro);

    // Analyze image
    println!("=== Image Analysis ===");
    match client
        .vision()
        .describe("image/png", image_data.clone())
        .await
    {
        Ok(description) => println!("Description: {}", description),
        Err(e) => eprintln!("Error: {}", e),
    }

    // OCR
    println!("\n=== OCR ===");
    match client.ocr("image/png", image_data).await {
        Ok(text) => println!("Extracted text: {}", text),
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}
