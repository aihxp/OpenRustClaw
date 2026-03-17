//! Image generation example for Fireworks AI.
//!
//! Note: This example demonstrates the API usage. To actually generate images,
//! you need sufficient credits in your Fireworks account.

use fireworks_ai::image_generation::{ImageGenerationRequest, ImageSize};
use fireworks_ai::FireworksClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("FIREWORKS_API_KEY")
        .expect("FIREWORKS_API_KEY environment variable must be set");

    // Create client
    let client = FireworksClient::new(api_key)?;
    println!("Created Fireworks client\n");

    // Generate image with Flux.1 Dev
    println!("Generating image with Flux.1 Dev...");
    let request = ImageGenerationRequest::builder(
        "accounts/fireworks/models/flux-1-dev",
        "A serene mountain landscape at sunset, with snow-capped peaks reflecting golden light"
    )
        .size(ImageSize::S1024x1024)
        .n(1)
        .seed(42)
        .cfg_scale(7.5)
        .num_inference_steps(50)
        .build();

    match client.images().generate(request).await {
        Ok(response) => {
            println!("Generated {} image(s)", response.data.len());
            
            for (i, image) in response.data.iter().enumerate() {
                if let Some(url) = &image.url {
                    println!("Image {} URL: {}", i + 1, url);
                }
                if let Some(b64) = &image.b64_json {
                    println!("Image {} (base64, {} chars)", i + 1, b64.len());
                }
                if let Some(prompt) = &image.revised_prompt {
                    println!("Revised prompt: {}", prompt);
                }
            }
        }
        Err(e) => {
            println!("Error generating image: {}", e);
            println!("(This is expected if you don't have image generation credits)");
        }
    }

    // Generate with SDXL and negative prompt
    println!("\nGenerating image with SDXL...");
    let request = ImageGenerationRequest::builder(
        "accounts/fireworks/models/sdxl",
        "A futuristic cityscape with flying cars and neon lights, cyberpunk style"
    )
        .negative_prompt("blurry, low quality, distorted, ugly")
        .size(ImageSize::S1024x1024)
        .guidance_scale(8.0)
        .num_inference_steps(30)
        .build();

    match client.images().generate(request).await {
        Ok(response) => {
            println!("Generated {} image(s)", response.data.len());
        }
        Err(e) => {
            println!("Error generating image: {}", e);
        }
    }

    // Generate multiple images with different sizes
    println!("\nGenerating images in different aspect ratios...");
    let sizes = vec![
        ("Square", ImageSize::S1024x1024),
        ("Landscape", ImageSize::S1024x576),
        ("Portrait", ImageSize::S576x1024),
    ];

    for (name, size) in sizes {
        let request = ImageGenerationRequest::builder(
            "accounts/fireworks/models/flux-1-dev",
            "A cute robot reading a book in a cozy library"
        )
            .size(size)
            .n(1)
            .build();

        match client.images().generate(request).await {
            Ok(_) => println!("  {}: Success", name),
            Err(_) => println!("  {}: Failed (check credits)", name),
        }
    }

    println!("\nExample completed!");

    Ok(())
}
