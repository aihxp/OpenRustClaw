//! Azure AD Authentication example for Azure OpenAI.
//!
//! This example demonstrates how to authenticate using Azure Active Directory.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_TOKEN=your-aad-token \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DEPLOYMENT=your-deployment \
//! cargo run --example azure_ad_auth
//!
//! # Or using client credentials:
//! AZURE_CLIENT_ID=your-client-id \
//! AZURE_CLIENT_SECRET=your-client-secret \
//! AZURE_TENANT_ID=your-tenant-id \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DEPLOYMENT=your-deployment \
//! cargo run --example azure_ad_auth --features azure-ad
//! ```

use azure_openai::{AzureConfig, AzureOpenAIClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
        .expect("AZURE_OPENAI_DEPLOYMENT environment variable not set");

    // Check for different authentication methods
    let client = if let Ok(token) = std::env::var("AZURE_OPENAI_TOKEN") {
        println!("Using Azure AD token authentication");
        let config = AzureConfig::azure_ad_token(token)
            .resource_name(&resource_name)
            .deployment_name(&deployment_name);
        AzureOpenAIClient::with_config(config)?
    } else if let (Ok(client_id), Ok(client_secret), Ok(tenant_id)) = (
        std::env::var("AZURE_CLIENT_ID"),
        std::env::var("AZURE_CLIENT_SECRET"),
        std::env::var("AZURE_TENANT_ID"),
    ) {
        println!("Using Azure AD client credentials authentication");
        let config = AzureConfig::azure_ad_client_credentials(client_id, client_secret, tenant_id)
            .resource_name(&resource_name)
            .deployment_name(&deployment_name);
        AzureOpenAIClient::with_config(config)?
    } else {
        eprintln!("Error: No Azure AD credentials found");
        eprintln!("Please set either:");
        eprintln!("  - AZURE_OPENAI_TOKEN");
        eprintln!("  - AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, and AZURE_TENANT_ID");
        std::process::exit(1);
    };

    println!("Sending chat completion request with Azure AD auth...\n");

    // Build the request
    let request = ChatRequest::builder()
        .system("You are a helpful assistant.")
        .user("Hello! I'm authenticated via Azure AD.")
        .build();

    // Send the request
    let response = client.chat().complete(request).await?;

    println!("Response: {}", response.content().unwrap_or("No content"));

    Ok(())
}
