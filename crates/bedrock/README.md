# AWS Bedrock SDK for Rust

A comprehensive Rust SDK for AWS Bedrock, part of the OpenRustClaw project.

## Features

- **Converse API**: Unified conversation interface for all Bedrock models
- **InvokeModel API**: Direct model invocation with native request/response formats
- **InvokeModelWithResponseStream**: Streaming responses for real-time output
- **All Bedrock Models**: Support for Anthropic Claude, Amazon Titan, Meta Llama, Mistral, Cohere, AI21, and Stability AI
- **AWS SigV4**: Full authentication with credential chain support
- **Cross-Region Inference**: Route requests to optimal regions
- **Guardrails**: Apply content filtering to model responses
- **Streaming**: Real-time streaming support for applicable models

## Quick Start

```rust
use aws_bedrock::{BedrockClient, ConverseRequest, Message, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with default credential chain
    let client = BedrockClient::new("us-east-1").await?;

    // Send a message
    let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
        .user_message("Hello, Claude!")
        .max_tokens(1000)
        .build();

    let response = client.converse().invoke(request).await?;
    println!("{}", response.text());

    Ok(())
}
```

## Authentication

The SDK uses the standard AWS credential chain:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. Shared credentials file (`~/.aws/credentials`)
3. Container credentials (ECS/EKS)
4. EC2 instance metadata service (IMDS)

```rust
// From environment
let client = BedrockClient::from_env("us-east-1").await?;

// With specific profile
let client = BedrockClient::with_profile("us-west-2", "production").await?;

// With explicit credentials
use aws_bedrock::auth::AwsCredentials;
let creds = AwsCredentials::new("AKIAIOS...", "wJalrXUtn...");
let client = BedrockClient::with_credentials("us-east-1", creds).await?;
```

## Streaming

```rust
use futures::StreamExt;

let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
    .user_message("Tell me a story")
    .build();

let mut stream = client.converse().stream(request).await?;
while let Some(chunk) = stream.next().await {
    match chunk? {
        StreamEvent::ContentBlockDelta { delta, .. } => {
            if let Some(text) = delta.text() {
                print!("{}", text);
            }
        }
        _ => {}
    }
}
```

## Tool Use

```rust
use aws_bedrock::types::{Tool, ToolInputSchema, ToolChoice};

let calculator = Tool::new("calculator", "Perform mathematical calculations")
    .with_schema(
        ToolInputSchema::object()
            .with_string("expression", "The mathematical expression")
            .required("expression")
    );

let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
    .user_message("What is 2 + 2?")
    .tool(calculator)
    .tool_choice(ToolChoice::auto())
    .build();
```

## Supported Models

### Anthropic Claude
- Claude 3 Opus, Sonnet, Haiku
- Claude 3.5 Sonnet, Haiku

### Amazon Titan
- Titan Text Express, Lite, Premier
- Titan Embeddings
- Titan Image Generator

### Meta Llama
- Llama 2 13B, 70B
- Llama 3 8B, 70B Instruct
- Llama 3.1 8B, 70B, 405B Instruct
- Llama 3.2 Vision models

### Mistral AI
- Mistral 7B Instruct
- Mixtral 8x7B Instruct
- Mistral Large, Small

### Cohere
- Command, Command Light
- Command R, Command R+

### AI21 Labs
- Jurassic-2 Ultra, Mid
- Jamba Instruct

### Stability AI
- Stable Diffusion XL
- Stable Diffusion 3
- Stable Image Core/Ultra

## Cross-Region Inference

```rust
use aws_bedrock::constants::{InferenceProfile, InferenceType};

// Use cross-region inference
let model_id = InferenceProfile::Us
    .create_profile_id("anthropic.claude-3-sonnet-20240229-v1:0");

let request = ConverseRequest::builder(&model_id)
    .user_message("Hello!")
    .build();
```

## Guardrails

```rust
use aws_bedrock::types::{GuardrailConfiguration, Trace};

let guardrail = GuardrailConfiguration::new("guardrail-id", "1")
    .with_trace(Trace::Enabled);

let request = ConverseRequest::builder("anthropic.claude-3-sonnet-20240229-v1:0")
    .user_message("Generate content")
    .guardrail(guardrail)
    .build();
```

## License

MIT - See LICENSE file for details.
