//! Example: Create and manage predictions
//!
//! Run with:
//!     REPLICATE_API_TOKEN=your_token cargo run --example predictions

use replicate::{PredictionRequest, ReplicateClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API token from environment
    let api_token = std::env::var("REPLICATE_API_TOKEN")
        .expect("REPLICATE_API_TOKEN environment variable must be set");

    // Create client
    let client = ReplicateClient::new(api_token)?;

    // Example 1: Create a prediction and poll for completion
    println!("Creating a prediction with Flux Schnell...");
    let request = PredictionRequest::new()
        .model("black-forest-labs/flux-schnell")
        .input("prompt", "Astronaut riding a horse on the moon")
        .input("num_inference_steps", 4);

    let prediction = client.predictions().run(request).await?;
    println!("Prediction completed!");
    println!("  ID: {}", prediction.id);
    println!("  Status: {}", prediction.status);
    println!("  Output: {:?}", prediction.output);

    // Example 2: List recent predictions
    println!("\nRecent predictions:");
    let predictions = client.predictions().list(None).await?;
    for pred in predictions.results.iter().take(5) {
        println!(
            "  {} - {:?} - {}",
            pred.id,
            pred.status,
            pred.model.as_deref().unwrap_or("unknown")
        );
    }

    // Example 3: Get a specific prediction
    println!("\nGetting prediction details...");
    let details = client.predictions().get(&prediction.id).await?;
    println!("  Created at: {}", details.created_at);
    if let Some(metrics) = details.metrics {
        if let Some(predict_time) = metrics.predict_time {
            println!("  Predict time: {:.2}s", predict_time);
        }
    }

    Ok(())
}
