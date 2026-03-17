# Replicate SDK for Rust

A native Rust SDK for the [Replicate](https://replicate.com) API - Run ML models in the cloud.

[![Crates.io](https://img.shields.io/crates/v/replicate)](https://crates.io/crates/replicate)
[![Documentation](https://docs.rs/replicate/badge.svg)](https://docs.rs/replicate)

## Features

- **Predictions**: Create and manage predictions with full lifecycle support
- **Official Models**: Run official models like Llama, Flux, Stable Diffusion without version IDs
- **Community Models**: Run any model on Replicate with specific version IDs
- **Deployments**: Create and manage dedicated model deployments
- **Webhooks**: Receive real-time updates with webhook support and signature verification
- **Streaming**: Stream prediction output in real-time with Server-Sent Events
- **Trainings**: Fine-tune models with custom training jobs
- **Async/Long-running Jobs**: Full support for asynchronous predictions with polling

## Installation

```toml
[dependencies]
replicate = "0.1.0"
```

## Quick Start

```rust
use replicate::{ReplicateClient, PredictionRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ReplicateClient::new("your-api-token")?;

    // Run an official model
    let request = PredictionRequest::new()
        .model("black-forest-labs/flux-schnell")
        .input("prompt", "Astronaut riding a horse");

    let prediction = client.predictions().create(request).await?;
    println!("Prediction ID: {}", prediction.id);

    // Poll for completion
    let output = client.predictions().wait_for_completion(&prediction.id).await?;
    println!("Output: {:?}", output);

    Ok(())
}
```

## Usage Examples

### Run and Wait (Convenience Method)

```rust
use replicate::{ReplicateClient, PredictionRequest};

let client = ReplicateClient::new("your-api-token")?;

let request = PredictionRequest::new()
    .model("black-forest-labs/flux-schnell")
    .input("prompt", "A cat playing piano");

let prediction = client.predictions().run(request).await?;
println!("Output: {:?}", prediction.output);
```

### Async Predictions with Webhooks

```rust
use replicate::{ReplicateClient, PredictionRequest, WebhookEvents};

let client = ReplicateClient::new("your-api-token")?;

let request = PredictionRequest::new()
    .model("meta/meta-llama-3-70b-instruct")
    .input("prompt", "Tell me a story")
    .webhook("https://myapp.com/webhooks/replicate")
    .webhook_events(vec![WebhookEvents::Completed, WebhookEvents::Output]);

let prediction = client.predictions().create(request).await?;
```

### Streaming Output

```rust
use replicate::ReplicateClient;
use futures::StreamExt;

let client = ReplicateClient::new("your-api-token")?;

let prediction = client.predictions()
    .create(PredictionRequest::new()
        .model("meta/meta-llama-3-70b-instruct")
        .input("prompt", "Count to 100"))
    .await?;

let mut stream = client.streaming().stream_output(&prediction.id).await?;
while let Some(event) = stream.next().await {
    match event {
        Ok(event) => println!("Event: {:?}", event),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Webhook Verification

```rust
use replicate::webhooks::WebhookVerifier;

let verifier = WebhookVerifier::new("your-webhook-secret")?;

// In your webhook handler
let signature = "t=1234567890,v1=abc123...";
let body = b"{\"id\": \"pred_123\", ...}";

match verifier.verify(signature, body) {
    Ok(()) => println!("Webhook verified!"),
    Err(e) => println!("Invalid webhook: {}", e),
}
```

### Official Models

Official models don't require version IDs:

```rust
use replicate::ReplicateClient;

let client = ReplicateClient::new("your-api-token")?;

// Using the models API for official models
let prediction = client.models()
    .create_prediction(
        "black-forest-labs",
        "flux-schnell",
        PredictionRequest::new().input("prompt", "A cat")
    )
    .await?;
```

### Deployments

```rust
use replicate::{ReplicateClient, DeploymentConfig};

let client = ReplicateClient::new("your-api-token")?;

// Create a deployment
let config = DeploymentConfig::new(
    "stability-ai/sdxl",
    "version_id_here",
    "gpu-a100-large",
    1,  // min instances
    5,  // max instances
);

let deployment = client.deployments()
    .create("myuser", "mydeployment", config)
    .await?;

// Run a deployment
let prediction = client.deployments()
    .create_prediction("myuser", "mydeployment", PredictionRequest::new())
    .await?;
```

### Fine-tuning

```rust
use replicate::{ReplicateClient, TrainingRequest};

let client = ReplicateClient::new("your-api-token")?;

let request = TrainingRequest::new("myuser/my-fine-tuned-model")
    .input("train_data", "https://example.com/data.zip")
    .input("num_train_epochs", 3);

let training = client.trainings()
    .create("base-owner", "base-model", "version-id", request)
    .await?;
```

## Supported Models

### Text Generation
- `meta/meta-llama-3-70b-instruct` - Meta's Llama 3 70B
- `meta/meta-llama-3-8b-instruct` - Meta's Llama 3 8B  
- `mistralai/mixtral-8x7b-instruct-v0.1` - Mistral's Mixtral 8x7B

### Image Generation
- `black-forest-labs/flux-schnell` - Fastest Flux image generation
- `black-forest-labs/flux-dev` - Flux development model
- `black-forest-labs/flux-1.1-pro` - Flux Pro
- `stability-ai/stable-diffusion-3` - Stable Diffusion 3
- `stability-ai/sdxl` - Stable Diffusion XL

### Audio
- `openai/whisper` - Speech-to-text transcription
- `meta/musicgen` - Music generation

## Features

- `default` - Enables `rustls-tls`, `predictions`, `models`, `webhooks`, `streaming`, `trainings`, `deployments`
- `rustls-tls` - Use rustls for TLS (default)
- `native-tls` - Use native TLS
- `predictions` - Enable predictions API
- `models` - Enable models API
- `deployments` - Enable deployments API
- `trainings` - Enable trainings API
- `webhooks` - Enable webhook verification
- `streaming` - Enable streaming support

## License

This project is licensed under the MIT License - see the LICENSE file for details.
