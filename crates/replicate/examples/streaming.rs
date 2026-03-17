//! Example: Stream prediction output
//!
//! Run with:
//!     REPLICATE_API_TOKEN=your_token cargo run --example streaming

use futures::StreamExt;
use replicate::{PredictionRequest, ReplicateClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API token from environment
    let api_token = std::env::var("REPLICATE_API_TOKEN")
        .expect("REPLICATE_API_TOKEN environment variable must be set");

    // Create client
    let client = ReplicateClient::new(api_token)?;

    // Create a prediction (using a streaming-capable model)
    println!("Creating a prediction...");
    let request = PredictionRequest::new()
        .model("meta/meta-llama-3-70b-instruct")
        .input("prompt", "Count from 1 to 10, one number per line");

    let prediction = client.predictions().create(request).await?;
    println!("Prediction created: {}", prediction.id);
    println!("Status: {:?}", prediction.status);

    // Check if streaming is available
    if let Some(stream_url) = &prediction.urls.stream {
        println!("\nStreaming from: {}", stream_url);

        // Stream the output
        let mut stream = client.streaming().stream_output(&prediction.id).await?;

        println!("Streaming output:");
        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    println!("  Event: {:?}", event);
                }
                Err(e) => {
                    eprintln!("  Stream error: {}", e);
                    break;
                }
            }
        }
    } else {
        println!("Streaming not available for this prediction.");
        println!("Polling for completion instead...");

        let completed = client
            .predictions()
            .wait_for_completion(&prediction.id)
            .await?;
        println!("Prediction completed!");
        println!("Output: {:?}", completed.output);
    }

    Ok(())
}
