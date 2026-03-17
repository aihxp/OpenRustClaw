//! DALL-E Image Generation example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DALLE_DEPLOYMENT=your-dalle-deployment \
//! cargo run --example dalle --features images
//! ```

#[cfg(not(feature = "images"))]
fn main() {
    println!("This example requires the 'images' feature.");
    println!("Run with: cargo run --example dalle --features images");
}

#[cfg(feature = "images")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use azure_openai::{AzureOpenAIClient, ImageRequest, ImageSize, ImageQuality, ImageStyle};
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_DALLE_DEPLOYMENT")
        .expect("AZURE_OPENAI_DALLE_DEPLOYMENT environment variable not set");

    // Create the client
    let client = AzureOpenAIClient::new(
        &resource_name,
        &deployment_name,
        &api_key,
    )?;

    println!("Azure OpenAI DALL-E Image Generation Example\n");

    // Generate a standard quality image
    println!("1. Generating standard quality image...");
    let request = ImageRequest::new("A serene mountain landscape with a lake at sunset")
        .size(ImageSize::Size1024x1024)
        .quality(ImageQuality::Standard)
        .style(ImageStyle::Natural)
        .n(1);

    let response = client.images().generate(request).await?;

    println!("   Created at: {}", response.created);
    for (i, image) in response.data.iter().enumerate() {
        if let Some(url) = &image.url {
            println!("   Image {} URL: {}", i + 1, url);
        }
        if let Some(revised_prompt) = &image.revised_prompt {
            println!("   Revised prompt: {}", revised_prompt);
        }
    }

    // Generate HD quality image with vivid style
    println!("\n2. Generating HD quality image with vivid style...");
    let request_hd = ImageRequest::new("A futuristic city with flying cars at night")
        .size(ImageSize::Size1792x1024)
        .quality(ImageQuality::Hd)
        .style(ImageStyle::Vivid)
        .n(1);

    let response_hd = client.images().generate(request_hd).await?;

    for (i, image) in response_hd.data.iter().enumerate() {
        if let Some(url) = &image.url {
            println!("   HD Image {} URL: {}", i + 1, url);
        }
    }

    // Generate portrait orientation image
    println!("\n3. Generating portrait orientation image...");
    let request_portrait = ImageRequest::new("A majestic lion in the savanna")
        .size(ImageSize::Size1024x1792)
        .quality(ImageQuality::Standard)
        .n(1);

    let response_portrait = client.images().generate(request_portrait).await?;

    for (i, image) in response_portrait.data.iter().enumerate() {
        if let Some(url) = &image.url {
            println!("   Portrait Image {} URL: {}", i + 1, url);
        }
    }

    println!("\nDone!");
    Ok(())
}
