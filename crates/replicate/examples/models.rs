//! Example: List and use models
//!
//! Run with:
//!     REPLICATE_API_TOKEN=your_token cargo run --example models

use replicate::{ReplicateClient, PredictionRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API token from environment
    let api_token = std::env::var("REPLICATE_API_TOKEN")
        .expect("REPLICATE_API_TOKEN environment variable must be set");

    // Create client
    let client = ReplicateClient::new(api_token)?;

    // Example 1: List popular models
    println!("Listing popular models...");
    let models = client.models().list(None, Some(10)).await?;
    for model in &models.results {
        println!("  {}/{} - {:?}", model.owner, model.name, model.description.as_ref().map(|s| &s[..s.len().min(50)]));
    }

    // Example 2: Get details about a specific model
    println!("\nGetting model details for flux-schnell...");
    let model = client.models().get("black-forest-labs", "flux-schnell").await?;
    println!("  Name: {}", model.name);
    println!("  Owner: {}", model.owner);
    println!("  Description: {:?}", model.description);
    
    if let Some(latest_version) = &model.latest_version {
        println!("  Latest version: {}", latest_version.id);
        println!("  Created at: {}", latest_version.created_at);
    }

    // Example 3: Create a prediction using the official models API
    println!("\nCreating a prediction with Flux Schnell (via models API)...");
    let request = PredictionRequest::new()
        .input("prompt", "A red cat sitting on a blue mat");

    let prediction = client.models()
        .create_prediction("black-forest-labs", "flux-schnell", request)
        .await?;
    
    println!("  Prediction ID: {}", prediction.id);
    println!("  Status: {:?}", prediction.status);

    // Wait for completion
    println!("\nWaiting for completion...");
    let completed = client.models()
        .run("black-forest-labs", "flux-schnell", PredictionRequest::new().input("prompt", "A dog"))
        .await?;
    
    println!("Completed! Output: {:?}", completed.output);

    // Example 4: Search for models
    println!("\nSearching for image generation models...");
    let search_results = client.models().search("image generation", Some(5)).await?;
    for model in &search_results.results {
        println!("  {}/{}", model.owner, model.name);
    }

    Ok(())
}
