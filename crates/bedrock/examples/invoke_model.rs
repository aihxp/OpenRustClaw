//! Example of using the InvokeModel API directly.
//!
//! This example demonstrates how to:
//! - Use the InvokeModel API for direct model invocation
//! - Work with model-specific request/response formats
//! - Call different models with their native APIs
//!
//! Run with:
//! ```bash
//! export AWS_ACCESS_KEY_ID=your_access_key
//! export AWS_SECRET_ACCESS_KEY=your_secret_key
//! cargo run --example invoke_model
//! ```

use aws_bedrock::BedrockClient;
use aws_bedrock::auth::Service;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = BedrockClient::new("us-east-1").await?;

    println!("Using InvokeModel API directly\n");

    // Example 1: Invoke Anthropic Claude with native format
    println!("=== Example 1: Claude 3 (native format) ===");

    let claude_body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 500,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "What is the capital of France?"
                    }
                ]
            }
        ]
    });

    let model_id = "anthropic.claude-3-haiku-20240307-v1:0";
    let path = format!("/model/{}/invoke", model_id);

    let response = client
        .request("POST", &path, Some(claude_body), Service::BedrockRuntime)
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if let Some(content) = body["content"].as_array() {
            for block in content {
                if let Some(text) = block["text"].as_str() {
                    println!("Claude: {}", text);
                }
            }
        }
        println!("Stop reason: {}\n", body["stop_reason"].as_str().unwrap_or("unknown"));
    } else {
        println!("Error: {:?}\n", response.text().await?);
    }

    // Example 2: Invoke Meta Llama
    println!("=== Example 2: Meta Llama 3 ===");

    let llama_body = json!({
        "prompt": "<|begin_of_text|><|start_header_id|>user<|end_header_id|>\n\nWhat is machine learning?<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n",
        "max_gen_len": 300,
        "temperature": 0.5,
        "top_p": 0.9
    });

    let model_id = "meta.llama3-8b-instruct-v1:0";
    let path = format!("/model/{}/invoke", model_id);

    let response = client
        .request("POST", &path, Some(llama_body), Service::BedrockRuntime)
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if let Some(generation) = body["generation"].as_str() {
            println!("Llama: {}", generation);
        }
        println!("Stop reason: {}\n", body["stop_reason"].as_str().unwrap_or("unknown"));
    } else {
        println!("Error: {:?}\n", response.text().await?);
    }

    // Example 3: Invoke Amazon Titan
    println!("=== Example 3: Amazon Titan Text Express ===");

    let titan_body = json!({
        "inputText": "Explain quantum computing in simple terms",
        "textGenerationConfig": {
            "maxTokenCount": 300,
            "temperature": 0.7,
            "topP": 0.9
        }
    });

    let model_id = "amazon.titan-text-express-v1";
    let path = format!("/model/{}/invoke", model_id);

    let response = client
        .request("POST", &path, Some(titan_body), Service::BedrockRuntime)
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if let Some(results) = body["results"].as_array() {
            for result in results {
                if let Some(text) = result["outputText"].as_str() {
                    println!("Titan: {}", text.trim());
                }
            }
        }
    } else {
        println!("Error: {:?}\n", response.text().await?);
    }

    println!("\nDone!");
    Ok(())
}
