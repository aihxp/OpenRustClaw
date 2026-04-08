use openrustclaw_core::error::{Error, Result};

pub fn validate_model_for_provider(provider: &str, model: &str) -> Result<()> {
    let provider = provider.trim().to_ascii_lowercase();
    let model = model.trim();
    if model.is_empty() || model.eq_ignore_ascii_case("auto") || model.eq_ignore_ascii_case("vendor-managed") {
        return Ok(());
    }

    match provider.as_str() {
        "anthropic" | "claude_code" => validate_anthropic_model(model),
        "codex" => validate_codex_model(model),
        _ => Ok(()),
    }
}

fn validate_anthropic_model(model: &str) -> Result<()> {
    if is_valid_anthropic_model(model) {
        return Ok(());
    }
    Err(Error::Internal(format!(
        "Model '{}' is not a recognized Anthropic Claude model. Expected a Claude model such as `claude-sonnet-4-20250514`, `claude-opus-4-1-20250805`, or `claude-3-5-haiku-20241022`.",
        model
    )))
}

fn validate_codex_model(model: &str) -> Result<()> {
    if is_valid_codex_model(model) {
        return Ok(());
    }
    Err(Error::Internal(format!(
        "Model '{}' is not a recognized Codex model. Expected a Codex-specific model such as `gpt-5.3-codex` or `gpt-5.1-codex-max`, not a generic model like `gpt-4o`.",
        model
    )))
}

fn is_valid_anthropic_model(model: &str) -> bool {
    matches!(
        model,
        "claude-opus-4-1-20250805"
            | "claude-opus-4-1"
            | "claude-opus-4-20250514"
            | "claude-opus-4-0"
            | "claude-sonnet-4-20250514"
            | "claude-sonnet-4-0"
            | "claude-sonnet-4-6"
            | "claude-3-7-sonnet-20250219"
            | "claude-3-7-sonnet-latest"
            | "claude-3-5-sonnet-20241022"
            | "claude-3-5-sonnet-latest"
            | "claude-3-5-haiku-20241022"
            | "claude-3-5-haiku-latest"
            | "claude-3-opus-20240229"
            | "claude-3-opus"
            | "claude-3-sonnet-20240229"
            | "claude-3-sonnet"
            | "claude-3-haiku-20240307"
            | "claude-3-haiku"
    )
}

fn is_valid_codex_model(model: &str) -> bool {
    if model == "codex-mini-latest" {
        return true;
    }

    if !model.starts_with("gpt-5") || !model.contains("-codex") {
        return false;
    }

    let suffix = &model["gpt-5".len()..];
    matches!(
        suffix,
        "-codex"
            | ".1-codex"
            | ".1-codex-mini"
            | ".1-codex-max"
            | ".2-codex"
            | ".3-codex"
            | ".4-codex"
    )
}

#[cfg(test)]
mod tests {
    use super::validate_model_for_provider;

    #[test]
    fn accepts_current_anthropic_models() {
        assert!(validate_model_for_provider("anthropic", "claude-sonnet-4-20250514").is_ok());
        assert!(validate_model_for_provider("anthropic", "claude-opus-4-1-20250805").is_ok());
        assert!(validate_model_for_provider("claude_code", "claude-sonnet-4-6").is_ok());
    }

    #[test]
    fn rejects_non_anthropic_models_for_claude_lanes() {
        let error = validate_model_for_provider("anthropic", "gpt-4o").unwrap_err();
        assert!(error.to_string().contains("Anthropic Claude model"));
    }

    #[test]
    fn accepts_codex_model_family() {
        assert!(validate_model_for_provider("codex", "gpt-5.3-codex").is_ok());
        assert!(validate_model_for_provider("codex", "gpt-5.1-codex-max").is_ok());
        assert!(validate_model_for_provider("codex", "codex-mini-latest").is_ok());
    }

    #[test]
    fn rejects_generic_openai_models_for_codex() {
        let error = validate_model_for_provider("codex", "gpt-4o").unwrap_err();
        assert!(error.to_string().contains("Codex model"));
    }
}
