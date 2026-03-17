//! Content Safety example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DEPLOYMENT=your-deployment \
//! cargo run --example content_safety --features content-safety
//! ```

#[cfg(not(feature = "content-safety"))]
fn main() {
    println!("This example requires the 'content-safety' feature.");
    println!("Run with: cargo run --example content_safety --features content-safety");
}

#[cfg(feature = "content-safety")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use azure_openai::{AzureOpenAIClient, content_safety::*};
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
        .expect("AZURE_OPENAI_DEPLOYMENT environment variable not set");

    // Create the client
    let client = AzureOpenAIClient::new(
        &resource_name,
        &deployment_name,
        api_key.clone(),
    )?;

    println!("Azure OpenAI Content Safety Example\n");

    // Test different types of content
    let test_cases = vec![
        ("Safe content", "The weather is nice today. I enjoy going for walks in the park."),
        ("Potentially problematic", "I hate everyone who disagrees with me."),
        ("Programming content", "How do I write a function in Rust to parse JSON?"),
    ];

    for (name, text) in test_cases {
        println!("\n{}: '{}'", name, truncate(text, 50));
        
        let request = TextAnalysisRequest::new(text)
            .categories(vec![
                HarmCategory::Hate,
                HarmCategory::SelfHarm,
                HarmCategory::Sexual,
                HarmCategory::Violence,
            ]);

        match client.content_safety().analyze_text(request).await {
            Ok(response) => {
                if let Some(categories) = &response.categories_analysis {
                    println!("   Analysis results:");
                    for category in categories {
                        println!("     {}: severity {}", category.category, category.severity);
                    }

                    // Check with different thresholds
                    let thresholds = SafetyThresholds::moderate();
                    let is_safe = ContentSafety::is_content_safe(&response, &thresholds);
                    println!("   Is safe (moderate thresholds): {}", is_safe);

                    if !is_safe {
                        println!("   ⚠️  Content flagged by content safety filters!");
                    }
                }

                if let Some(blocklists) = &response.blocklists_match {
                    if !blocklists.is_empty() {
                        println!("   Blocklist matches:");
                        for blocklist in blocklists {
                            println!("     - {}: {}", 
                                blocklist.blocklist_name, 
                                blocklist.blocklist_item_text
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("   Error analyzing text: {}", e);
            }
        }
    }

    // Demonstrate threshold comparison
    println!("\n\nThreshold Comparison:");
    
    let test_text = "This is a sample text for testing thresholds.";
    let request = TextAnalysisRequest::new(test_text);
    
    if let Ok(response) = client.content_safety().analyze_text(request).await {
        let thresholds_configs = vec![
            ("Most Restrictive", SafetyThresholds::most_restrictive()),
            ("Moderate", SafetyThresholds::moderate()),
            ("Permissive", SafetyThresholds::permissive()),
        ];

        for (name, thresholds) in thresholds_configs {
            let is_safe = ContentSafety::is_content_safe(&response, &thresholds);
            println!("   {}: safe={}", name, is_safe);
        }
    }

    // Example of content filter details from OpenAI response
    println!("\n\nContent Filter Details (from OpenAI response):");
    let filter_details = ContentFilterDetails {
        hate: Some(ContentFilterResult {
            filtered: false,
            severity: Some("safe".to_string()),
        }),
        self_harm: Some(ContentFilterResult {
            filtered: false,
            severity: Some("safe".to_string()),
        }),
        sexual: Some(ContentFilterResult {
            filtered: false,
            severity: Some("safe".to_string()),
        }),
        violence: Some(ContentFilterResult {
            filtered: false,
            severity: Some("safe".to_string()),
        }),
        profanity: None,
    };

    println!("   Was filtered: {}", filter_details.was_filtered());
    println!("   Highest severity: {:?}", filter_details.highest_severity());
    
    let report = utils::format_filter_report(&filter_details);
    println!("   Report: {}", report);

    println!("\nDone!");
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
