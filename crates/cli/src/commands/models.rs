//! Model management commands.

use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

use super::runtime;

/// Model information for display.
#[derive(Debug)]
struct ModelInfo {
    name: String,
    provider: String,
    description: String,
    context_window: usize,
    supports_tools: bool,
    supports_vision: bool,
}

/// Default models for each provider.
fn get_default_models() -> HashMap<&'static str, Vec<ModelInfo>> {
    let mut providers = HashMap::new();

    // Anthropic models
    providers.insert(
        "anthropic",
        vec![
            ModelInfo {
                name: "claude-sonnet-4-20250514".to_string(),
                provider: "anthropic".to_string(),
                description: "Claude Sonnet 4 - Balanced performance and cost".to_string(),
                context_window: 200_000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                name: "claude-opus-4-20250514".to_string(),
                provider: "anthropic".to_string(),
                description: "Claude Opus 4 - Most capable model".to_string(),
                context_window: 200_000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                name: "claude-haiku-4-20250514".to_string(),
                provider: "anthropic".to_string(),
                description: "Claude Haiku 4 - Fast and cost-effective".to_string(),
                context_window: 200_000,
                supports_tools: true,
                supports_vision: true,
            },
        ],
    );

    // OpenAI models
    providers.insert(
        "openai",
        vec![
            ModelInfo {
                name: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                description: "GPT-4o - Omni model for text, vision, and audio".to_string(),
                context_window: 128_000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                name: "gpt-4o-mini".to_string(),
                provider: "openai".to_string(),
                description: "GPT-4o Mini - Faster and more affordable".to_string(),
                context_window: 128_000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                name: "o3-mini".to_string(),
                provider: "openai".to_string(),
                description: "O3 Mini - Reasoning model".to_string(),
                context_window: 200_000,
                supports_tools: true,
                supports_vision: false,
            },
        ],
    );

    // OpenRouter models (popular ones)
    providers.insert(
        "openrouter",
        vec![
            ModelInfo {
                name: "anthropic/claude-sonnet-4".to_string(),
                provider: "openrouter".to_string(),
                description: "Claude Sonnet via OpenRouter".to_string(),
                context_window: 200_000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                name: "openai/gpt-4o".to_string(),
                provider: "openrouter".to_string(),
                description: "GPT-4o via OpenRouter".to_string(),
                context_window: 128_000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                name: "meta-llama/llama-3.3-70b-instruct".to_string(),
                provider: "openrouter".to_string(),
                description: "Llama 3.3 70B Instruct".to_string(),
                context_window: 128_000,
                supports_tools: true,
                supports_vision: false,
            },
            ModelInfo {
                name: "google/gemini-2.0-flash-exp".to_string(),
                provider: "openrouter".to_string(),
                description: "Gemini 2.0 Flash".to_string(),
                context_window: 1_000_000,
                supports_tools: true,
                supports_vision: true,
            },
        ],
    );

    // Ollama models (local)
    providers.insert(
        "ollama",
        vec![
            ModelInfo {
                name: "llama3.1".to_string(),
                provider: "ollama".to_string(),
                description: "Llama 3.1 - Meta's open model (local)".to_string(),
                context_window: 128_000,
                supports_tools: true,
                supports_vision: false,
            },
            ModelInfo {
                name: "llama3.2".to_string(),
                provider: "ollama".to_string(),
                description: "Llama 3.2 - Lightweight multimodal (local)".to_string(),
                context_window: 128_000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelInfo {
                name: "mistral".to_string(),
                provider: "ollama".to_string(),
                description: "Mistral - Efficient open model (local)".to_string(),
                context_window: 32_000,
                supports_tools: true,
                supports_vision: false,
            },
            ModelInfo {
                name: "qwen2.5".to_string(),
                provider: "ollama".to_string(),
                description: "Qwen 2.5 - Alibaba's open model (local)".to_string(),
                context_window: 128_000,
                supports_tools: true,
                supports_vision: false,
            },
        ],
    );

    providers
}

/// List all available providers and their models.
pub async fn list() -> Result<()> {
    if let Ok(workspace_root) = std::env::current_dir() {
        let _ = runtime::apply_runtime_secret_sources(&workspace_root);
    }
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              Available LLM Providers                     ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let providers = get_default_models();

    // Check which providers are configured
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").is_ok();
    let openai_key = std::env::var("OPENAI_API_KEY").is_ok();
    let openrouter_key = std::env::var("OPENROUTER_API_KEY").is_ok();
    let ollama_available = check_ollama().await;

    for (provider_name, models) in providers {
        let status = match provider_name {
            "anthropic" if anthropic_key => "\x1b[32m✓ configured\x1b[0m",
            "anthropic" => "\x1b[90m○ not configured\x1b[0m",
            "openai" if openai_key => "\x1b[32m✓ configured\x1b[0m",
            "openai" => "\x1b[90m○ not configured\x1b[0m",
            "openrouter" if openrouter_key => "\x1b[32m✓ configured\x1b[0m",
            "openrouter" => "\x1b[90m○ not configured\x1b[0m",
            "ollama" if ollama_available => "\x1b[32m✓ available\x1b[0m",
            "ollama" => "\x1b[90m○ not detected\x1b[0m",
            _ => "",
        };

        println!("\x1b[1m{}\x1b[0m {}", provider_name.to_uppercase(), status);
        println!(
            "  Default: {}",
            models.first().map(|m| m.name.as_str()).unwrap_or("N/A")
        );

        for model in models {
            let features = format_features(model.supports_tools, model.supports_vision);
            println!("    • {} {}", model.name, features);
        }
        println!();
    }

    println!("Environment variables:");
    println!("  ANTHROPIC_API_KEY  - Required for Anthropic models");
    println!("  OPENAI_API_KEY     - Required for OpenAI models");
    println!("  OPENROUTER_API_KEY - Required for OpenRouter models");
    println!("  OLLAMA_BASE_URL    - Optional, defaults to http://localhost:11434");
    println!();

    Ok(())
}

/// Show detailed info about a specific model.
pub async fn info(name: &str) -> Result<()> {
    println!("Model: \x1b[1m{}\x1b[0m", name);
    println!();

    let providers = get_default_models();

    // Search for the model
    let mut found = None;
    for models in providers.values() {
        if let Some(model) = models.iter().find(|m| m.name == name) {
            found = Some(model);
            break;
        }
    }

    if let Some(model) = found {
        println!("Provider:   {}", model.provider);
        println!("Description: {}", model.description);
        println!(
            "Context Window: {} tokens",
            format_number(model.context_window)
        );
        println!("Features:");
        println!(
            "  - Tool Calling: {}",
            if model.supports_tools {
                "✓ Yes"
            } else {
                "✗ No"
            }
        );
        println!(
            "  - Vision:       {}",
            if model.supports_vision {
                "✓ Yes"
            } else {
                "✗ No"
            }
        );
    } else {
        println!("Model '{}' not found in the default registry.", name);
        println!();
        println!("You can still use this model if your provider supports it.");
        println!("Run `openrustclaw models list` to see available models.");
    }

    println!();
    Ok(())
}

/// Scan configured providers and recommend role assignments.
pub async fn scan() -> Result<()> {
    if let Ok(workspace_root) = std::env::current_dir() {
        let _ = runtime::apply_runtime_secret_sources(&workspace_root);
    }
    let ollama_available = check_ollama().await;
    let providers = vec![
        ProviderScan {
            provider: "groq",
            key_env: Some("GROQ_API_KEY"),
            role_hint: "core_model",
            note: "Low-latency core runtime recommendation",
        },
        ProviderScan {
            provider: "openrouter",
            key_env: Some("OPENROUTER_API_KEY"),
            role_hint: "control_plane_model",
            note: "Broad fallback/control-plane recommendation",
        },
        ProviderScan {
            provider: "siliconflow",
            key_env: Some("SILICONFLOW_API_KEY"),
            role_hint: "secondary_core_model",
            note: "Higher-capability secondary routing recommendation",
        },
        ProviderScan {
            provider: "ollama",
            key_env: None,
            role_hint: "offline_fallback",
            note: "Local/offline fallback recommendation",
        },
    ];

    println!("Recommended model-role scan");
    println!();
    for provider in providers {
        let healthy = match provider.provider {
            "ollama" => ollama_available,
            _ => provider
                .key_env
                .and_then(|key| std::env::var(key).ok())
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false),
        };
        let status = if healthy {
            "\x1b[32mhealthy\x1b[0m"
        } else {
            "\x1b[90mnot configured\x1b[0m"
        };
        println!(
            "- {:12} {:22} {:20} {}",
            provider.provider, status, provider.role_hint, provider.note
        );
    }

    println!();
    println!("Suggested defaults:");
    println!("  core_model:           Groq when BYOK is configured and healthy");
    println!("  control_plane_model:  OpenRouter for broad fallback/model discovery");
    println!("  secondary_core_model: SiliconFlow for higher-capability routing");
    println!("  offline_fallback:     Ollama");
    println!();
    println!("Use `openrustclaw control init` to scaffold model profiles reflecting these roles.");
    Ok(())
}

struct ProviderScan {
    provider: &'static str,
    key_env: Option<&'static str>,
    role_hint: &'static str,
    note: &'static str,
}

/// Check if Ollama is available locally.
async fn check_ollama() -> bool {
    let base_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

    reqwest::Client::new()
        .get(format!("{}/api/tags", base_url))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .is_ok()
}

/// Format model features for display.
fn format_features(tools: bool, vision: bool) -> String {
    let mut features = Vec::new();
    if tools {
        features.push("tools");
    }
    if vision {
        features.push("vision");
    }

    if features.is_empty() {
        String::new()
    } else {
        format!("\x1b[90m[{}]\x1b[0m", features.join(", "))
    }
}

/// Format a number with commas.
fn format_number(n: usize) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_features_both() {
        let result = format_features(true, true);
        assert!(result.contains("tools"));
        assert!(result.contains("vision"));
    }

    #[test]
    fn test_format_features_tools_only() {
        let result = format_features(true, false);
        assert!(result.contains("tools"));
        assert!(!result.contains("vision"));
    }

    #[test]
    fn test_format_features_vision_only() {
        let result = format_features(false, true);
        assert!(!result.contains("tools"));
        assert!(result.contains("vision"));
    }

    #[test]
    fn test_format_features_none() {
        let result = format_features(false, false);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_number_small() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1), "1");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn test_format_number_thousands() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(12345), "12,345");
    }

    #[test]
    fn test_format_number_millions() {
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(200000), "200,000");
    }

    #[test]
    fn test_get_default_models_has_all_providers() {
        let models = get_default_models();
        assert!(models.contains_key("anthropic"));
        assert!(models.contains_key("openai"));
        assert!(models.contains_key("openrouter"));
        assert!(models.contains_key("ollama"));
    }

    #[test]
    fn test_get_default_models_anthropic_has_models() {
        let models = get_default_models();
        let anthropic = models.get("anthropic").unwrap();
        assert!(!anthropic.is_empty());
        // All Anthropic models should support tools and vision
        for model in anthropic {
            assert_eq!(model.provider, "anthropic");
            assert!(model.supports_tools);
            assert!(model.supports_vision);
            assert!(model.context_window > 0);
        }
    }

    #[test]
    fn test_get_default_models_openai_has_models() {
        let models = get_default_models();
        let openai = models.get("openai").unwrap();
        assert!(!openai.is_empty());
        for model in openai {
            assert_eq!(model.provider, "openai");
            assert!(model.context_window > 0);
        }
    }

    #[test]
    fn test_get_default_models_ollama_has_models() {
        let models = get_default_models();
        let ollama = models.get("ollama").unwrap();
        assert!(!ollama.is_empty());
        for model in ollama {
            assert_eq!(model.provider, "ollama");
        }
    }

    #[test]
    fn test_model_info_fields_populated() {
        let models = get_default_models();
        for (_provider, model_list) in &models {
            for model in model_list {
                assert!(!model.name.is_empty());
                assert!(!model.provider.is_empty());
                assert!(!model.description.is_empty());
                assert!(model.context_window > 0);
            }
        }
    }
}
