//! Example of using cross-region inference with AWS Bedrock.
//!
//! This example demonstrates how to:
//! - Use cross-region inference profiles
//! - Route requests to optimal regions
//!
//! Run with:
//! ```bash
//! export AWS_ACCESS_KEY_ID=your_access_key
//! export AWS_SECRET_ACCESS_KEY=your_secret_key
//! cargo run --example cross_region
//! ```

use aws_bedrock::constants::InferenceProfile;
use aws_bedrock::{BedrockClient, ConverseRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create a Bedrock client
    // Note: Cross-region inference is configured at the model ID level
    let client = BedrockClient::new("us-east-1").await?;

    println!("Cross-Region Inference Example\n");

    // Create a model ID with cross-region inference profile
    // This routes the request to the optimal region within the US
    let base_model_id = "anthropic.claude-3-haiku-20240307-v1:0";
    let cross_region_model_id = InferenceProfile::Us.create_profile_id(base_model_id);

    println!("Base model ID: {}", base_model_id);
    println!("Cross-region model ID: {}", cross_region_model_id);

    // Create a conversation request using the cross-region profile
    let request = ConverseRequest::builder(&cross_region_model_id)
        .user_message("What are the benefits of cross-region inference?")
        .max_tokens(500)
        .build();

    println!("\nSending request using cross-region inference...\n");

    let response = client.converse().invoke(request).await?;

    println!("Response:");
    println!("==================");
    println!("{}", response.text());
    println!("==================\n");

    println!("Token usage:");
    println!("  Input: {}", response.usage().input_tokens);
    println!("  Output: {}", response.usage().output_tokens);
    println!("  Total: {}", response.usage().total_tokens);

    // You can also use the EU or APAC profiles
    println!("\nAvailable inference profiles:");
    println!("  - US: {}", InferenceProfile::Us.as_str());
    println!("  - EU: {}", InferenceProfile::Eu.as_str());
    println!("  - APAC: {}", InferenceProfile::Apac.as_str());

    Ok(())
}
