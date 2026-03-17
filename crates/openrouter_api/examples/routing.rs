//! Routing strategies example.
//!
//! Run with:
//! ```bash
//! OPENROUTER_API_KEY=your-key cargo run --example routing
//! ```

use openrouter_api::{ChatRequest, OpenRouterClient, RouteStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");

    let client = OpenRouterClient::new(api_key)?;

    // Try different routing strategies
    let strategies = vec![
        (RouteStrategy::Quality, "Quality (default)"),
        (RouteStrategy::Price, "Price (cheapest)"),
        (RouteStrategy::Throughput, "Throughput (fastest)"),
    ];

    for (strategy, name) in strategies {
        let request = ChatRequest::builder("openai/gpt-4o-mini")
            .user("Say hello briefly")
            .route_strategy(strategy)
            .build();

        println!("\n--- {} routing ---", name);
        println!("Effective model: {}", request.model);

        let response = client.chat().complete(request).await?;
        println!("Response: {}", response.content());
    }

    Ok(())
}
