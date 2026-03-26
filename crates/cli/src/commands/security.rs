//! Security operations commands.

use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::io::Write;
use std::path::Path;

use openrustclaw_security::SkillVerifier;
use openrustclaw_security::audit::{AuditEvent, AuditSeverity};
use sqlx::Row;

use super::runtime;

#[derive(Debug, Clone, Serialize)]
pub struct SecurityPostureCheck {
    pub area: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityPostureSummary {
    pub generated_at: String,
    pub require_auth: bool,
    pub origin_validation: bool,
    pub allowed_origins: Vec<String>,
    pub control_api_token_configured: bool,
    pub trusted_proxy_token_configured: bool,
    pub skill_signature_required: bool,
    pub skill_verifying_key_configured: bool,
    pub vault_present: bool,
    pub secure_by_default: bool,
    pub issues_found: usize,
    pub warnings_found: usize,
    pub checks: Vec<SecurityPostureCheck>,
    pub recommended_actions: Vec<String>,
}

pub fn posture_summary(config_path: &str, workspace_root: &Path) -> Result<SecurityPostureSummary> {
    let config = runtime::load_effective_config(config_path, workspace_root)?;
    let vault_present = runtime::vault_path_for(workspace_root).exists();
    let control_api_token_configured = config.security.control_api_token_env.is_some();
    let trusted_proxy_token_configured = config.security.trusted_proxy_token_env.is_some();
    let skill_verifying_key_configured = config
        .security
        .skill_verifying_key
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    let mut issues_found = 0;
    let mut warnings_found = 0;
    let mut checks = Vec::new();

    if config.security.require_auth {
        checks.push(SecurityPostureCheck {
            area: "gateway_auth".to_string(),
            status: "pass".to_string(),
            detail: "Gateway authentication is required.".to_string(),
        });
    } else {
        warnings_found += 1;
        checks.push(SecurityPostureCheck {
            area: "gateway_auth".to_string(),
            status: "warn".to_string(),
            detail: "Gateway authentication is disabled.".to_string(),
        });
    }

    if config.security.origin_validation && !config.gateway.allowed_origins.is_empty() {
        checks.push(SecurityPostureCheck {
            area: "origin_validation".to_string(),
            status: "pass".to_string(),
            detail: format!(
                "Origin validation is enabled for {} allowed origin(s).",
                config.gateway.allowed_origins.len()
            ),
        });
    } else {
        issues_found += 1;
        checks.push(SecurityPostureCheck {
            area: "origin_validation".to_string(),
            status: "fail".to_string(),
            detail: "Origin validation is disabled or has no explicit allowed origins.".to_string(),
        });
    }

    if control_api_token_configured || trusted_proxy_token_configured {
        checks.push(SecurityPostureCheck {
            area: "control_plane_access".to_string(),
            status: "pass".to_string(),
            detail: format!(
                "Control-plane protection configured via {}{}.",
                if control_api_token_configured {
                    "control API bearer token"
                } else {
                    "trusted proxy token"
                },
                if control_api_token_configured && trusted_proxy_token_configured {
                    " and trusted proxy token"
                } else {
                    ""
                }
            ),
        });
    } else {
        warnings_found += 1;
        checks.push(SecurityPostureCheck {
            area: "control_plane_access".to_string(),
            status: "warn".to_string(),
            detail: "Control-plane bearer-token and trusted-proxy gates are both unset."
                .to_string(),
        });
    }

    if config.security.skill_signature_required && !skill_verifying_key_configured {
        issues_found += 1;
        checks.push(SecurityPostureCheck {
            area: "skill_verification".to_string(),
            status: "fail".to_string(),
            detail: "Skill signature verification is required but no verifying key is configured."
                .to_string(),
        });
    } else if skill_verifying_key_configured {
        checks.push(SecurityPostureCheck {
            area: "skill_verification".to_string(),
            status: "pass".to_string(),
            detail: "Skill verifying key is configured.".to_string(),
        });
    } else {
        warnings_found += 1;
        checks.push(SecurityPostureCheck {
            area: "skill_verification".to_string(),
            status: "warn".to_string(),
            detail:
                "Skill verifying key is not configured; external skill verification remains weaker than the final release target."
                    .to_string(),
        });
    }

    if vault_present {
        checks.push(SecurityPostureCheck {
            area: "runtime_vault".to_string(),
            status: "pass".to_string(),
            detail: "Runtime vault artifact is present.".to_string(),
        });
    } else {
        warnings_found += 1;
        checks.push(SecurityPostureCheck {
            area: "runtime_vault".to_string(),
            status: "warn".to_string(),
            detail: "Runtime vault artifact is absent; secrets may still be env-only.".to_string(),
        });
    }

    warnings_found += 1;
    checks.push(SecurityPostureCheck {
        area: "sandbox_boundary".to_string(),
        status: "warn".to_string(),
        detail:
            "Bounded execution lanes are present, but the final release gate should still treat sandbox verification as an explicit operator review item."
                .to_string(),
    });

    let mut recommended_actions = Vec::new();
    if !config.security.require_auth {
        recommended_actions
            .push("Set `[security].require_auth = true` before release promotion.".to_string());
    }
    if !config.security.origin_validation || config.gateway.allowed_origins.is_empty() {
        recommended_actions.push(
            "Enable origin validation with explicit `gateway.allowed_origins` before release."
                .to_string(),
        );
    }
    if !control_api_token_configured && !trusted_proxy_token_configured {
        recommended_actions.push(
            "Configure `security.control_api_token_env` or `security.trusted_proxy_token_env` for the control plane.".to_string(),
        );
    }
    if !skill_verifying_key_configured {
        recommended_actions.push(
            "Generate and configure a skill verifying key with `openrustclaw security generate-keys`."
                .to_string(),
        );
    }
    if !vault_present {
        recommended_actions.push(
            "Initialize and populate the runtime vault before final release candidate testing."
                .to_string(),
        );
    }
    recommended_actions.push(
        "Run `openrustclaw security audit` and review `/control/security/posture` before release sign-off.".to_string(),
    );

    Ok(SecurityPostureSummary {
        generated_at: Utc::now().to_rfc3339(),
        require_auth: config.security.require_auth,
        origin_validation: config.security.origin_validation,
        allowed_origins: config.gateway.allowed_origins.clone(),
        control_api_token_configured,
        trusted_proxy_token_configured,
        skill_signature_required: config.security.skill_signature_required,
        skill_verifying_key_configured,
        vault_present,
        secure_by_default: issues_found == 0 && warnings_found == 0,
        issues_found,
        warnings_found,
        checks,
        recommended_actions,
    })
}

/// Run security audit checks.
pub async fn audit() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              Security Audit Report                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    let mut issues_found = 0;
    let mut warnings = 0;

    // 1. Check API keys in environment
    println!("[1/5] Checking API key configuration...");
    let api_keys = vec![
        ("ANTHROPIC_API_KEY", "Anthropic"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("OPENROUTER_API_KEY", "OpenRouter"),
    ];

    for (env_var, name) in api_keys {
        match std::env::var(env_var) {
            Ok(key) => {
                if key.len() < 20 {
                    println!(
                        "  \x1b[33m⚠ {}: Key seems too short ({} chars)\x1b[0m",
                        name,
                        key.len()
                    );
                    warnings += 1;
                } else {
                    println!("  \x1b[32m✓ {}: Configured\x1b[0m", name);
                }
            }
            Err(_) => {
                println!("  \x1b[90m○ {}: Not configured\x1b[0m", name);
            }
        }
    }
    println!();

    // 2. Check skill signatures
    println!("[2/5] Checking skill signatures...");

    // Load configuration
    let config = openrustclaw_core::config::AppConfig::load().unwrap_or_default();

    // Initialize pool
    let pool = openrustclaw_db::init_pool(&config.database.url, 2)
        .await
        .ok();

    if let Some(pool) = pool {
        let rows = sqlx::query(
            "SELECT name, source, verified FROM skills WHERE source IN ('marketplace', 'managed')",
        )
        .fetch_all(&pool)
        .await?;

        if rows.is_empty() {
            println!("  \x1b[32m✓ No external skills installed\x1b[0m");
        } else {
            for row in rows {
                let name: String = row.get("name");
                let source: String = row.get("source");
                let verified: i64 = row.get("verified");

                if verified == 1 {
                    println!("  \x1b[32m✓ {} ({}): Verified\x1b[0m", name, source);
                } else {
                    println!("  \x1b[31m✗ {} ({}): Not verified\x1b[0m", name, source);
                    issues_found += 1;
                }
            }
        }
    }
    println!();

    // 3. Check sandbox policies
    println!("[3/5] Checking sandbox configuration...");
    println!("  \x1b[33m⚠ WASM sandbox: Planned, executor not yet implemented\x1b[0m");
    warnings += 1;
    println!("  \x1b[90m○ Capability model: Defined in skill metadata\x1b[0m");
    match config.security.skill_verifying_key.as_deref() {
        Some(key) if !key.trim().is_empty() => {
            println!("  \x1b[32m✓ Skill verifying key configured\x1b[0m");
        }
        _ => {
            println!("  \x1b[33m⚠ Skill verifying key not configured\x1b[0m");
            warnings += 1;
        }
    }
    println!();

    // 4. Check origin validation
    println!("[4/5] Checking origin validation...");
    if config.security.origin_validation {
        println!("  \x1b[32m✓ Origin validation enabled\x1b[0m");
        println!("  Allowed origins:");
        for origin in &config.gateway.allowed_origins {
            println!("    - {}", origin);
        }
    } else {
        println!("  \x1b[33m⚠ Origin validation disabled\x1b[0m");
        warnings += 1;
    }
    println!();

    // 5. Check authentication
    println!("[5/5] Checking authentication configuration...");
    if config.security.require_auth {
        println!("  \x1b[32m✓ Authentication required\x1b[0m");
    } else {
        println!("  \x1b[33m⚠ Authentication disabled\x1b[0m");
        warnings += 1;
    }
    println!();

    // Summary
    println!("══════════════════════════════════════════════════════════");
    if issues_found == 0 && warnings == 0 {
        println!("\x1b[32m✓ All security checks passed!\x1b[0m");
    } else {
        if issues_found > 0 {
            println!("\x1b[31m✗ {} issue(s) found\x1b[0m", issues_found);
        }
        if warnings > 0 {
            println!("\x1b[33m⚠ {} warning(s)\x1b[0m", warnings);
        }
    }
    println!();

    // Log audit event
    let event = AuditEvent::new(
        "security_audit",
        if issues_found > 0 {
            AuditSeverity::Error
        } else {
            AuditSeverity::Info
        },
    )
    .with_details(&format!(
        "Found {} issues, {} warnings",
        issues_found, warnings
    ));
    event.log();

    Ok(())
}

/// Generate Ed25519 keypair for skill signing.
pub async fn generate_keys() -> Result<()> {
    println!("Generating Ed25519 keypair for skill signing...");
    println!();

    // Generate keypair
    let (signing_key, verifying_key) = SkillVerifier::generate_keypair();

    let signing_key_hex = hex::encode(&signing_key.to_bytes());
    let verifying_key_hex = hex::encode(verifying_key.as_bytes());

    println!("✓ Keypair generated successfully!");
    println!();

    // Display keys
    println!("══════════════════════════════════════════════════════════");
    println!("  \x1b[1mSIGNING KEY (PRIVATE - keep secret!)\x1b[0m");
    println!("══════════════════════════════════════════════════════════");
    println!("{}", signing_key_hex);
    println!();
    println!("  Store this in a secure location (e.g., password manager)");
    println!("  This key is used to sign skills before distribution");
    println!();

    println!("══════════════════════════════════════════════════════════");
    println!("  \x1b[1mVERIFYING KEY (PUBLIC - can be shared)\x1b[0m");
    println!("══════════════════════════════════════════════════════════");
    println!("{}", verifying_key_hex);
    println!();
    println!("  This key should be distributed with OpenRustClaw");
    println!("  It's used to verify skill signatures");
    println!();

    // Option to save to file
    print!("Save keys to files? (signing_key.pem, verifying_key.pem) [y/N]: ");
    std::io::stdout().flush()?;

    let mut response = String::new();
    std::io::stdin().read_line(&mut response)?;

    if response.trim().eq_ignore_ascii_case("y") {
        // Save signing key
        let signing_key_pem = format!(
            "-----BEGIN OPENRUSTCLAW SIGNING KEY-----\n{}\n-----END OPENRUSTCLAW SIGNING KEY-----\n",
            signing_key_hex
        );
        tokio::fs::write("signing_key.pem", signing_key_pem).await?;

        // Save verifying key
        let verifying_key_pem = format!(
            "-----BEGIN OPENRUSTCLAW VERIFYING KEY-----\n{}\n-----END OPENRUSTCLAW VERIFYING KEY-----\n",
            verifying_key_hex
        );
        tokio::fs::write("verifying_key.pem", verifying_key_pem).await?;

        println!("✓ Keys saved to signing_key.pem and verifying_key.pem");
        println!();
        println!("  \x1b[1mIMPORTANT:\x1b[0m");
        println!("  - Move signing_key.pem to a secure location");
        println!(
            "  - Set [security].skill_verifying_key = \"{}\"",
            verifying_key_hex
        );
        println!("  - Never commit signing_key.pem to version control!");
    }

    // Log audit event
    let event = AuditEvent::new("key_generation", AuditSeverity::Info)
        .with_details("Generated new Ed25519 keypair for skill signing");
    event.log();

    Ok(())
}

/// Simple hex encoding utility
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::hex;
    use super::posture_summary;
    use anyhow::Result;
    use tempfile::tempdir;

    #[test]
    fn test_hex_encode_empty() {
        assert_eq!(hex::encode(&[]), "");
    }

    #[test]
    fn test_hex_encode_single_byte() {
        assert_eq!(hex::encode(&[0x00]), "00");
        assert_eq!(hex::encode(&[0xff]), "ff");
        assert_eq!(hex::encode(&[0x0a]), "0a");
    }

    #[test]
    fn test_hex_encode_multiple_bytes() {
        assert_eq!(hex::encode(&[0x48, 0x65, 0x6c, 0x6c, 0x6f]), "48656c6c6f");
    }

    #[test]
    fn test_hex_encode_all_zeros() {
        assert_eq!(hex::encode(&[0, 0, 0, 0]), "00000000");
    }

    #[test]
    fn test_hex_encode_all_ff() {
        assert_eq!(hex::encode(&[0xff, 0xff, 0xff]), "ffffff");
    }

    #[test]
    fn test_hex_encode_lowercase() {
        let encoded = hex::encode(&[0xAB, 0xCD, 0xEF]);
        assert_eq!(encoded, "abcdef");
        // Verify all lowercase
        assert!(
            encoded
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }

    #[test]
    fn posture_summary_reports_release_critical_fields() -> Result<()> {
        let temp = tempdir()?;
        let workspace_root = temp.path();
        std::fs::create_dir_all(workspace_root.join("config"))?;
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/default.toml"),
            workspace_root.join("config/default.toml"),
        )?;
        std::fs::create_dir_all(workspace_root.join(".claw/control"))?;
        std::fs::write(
            workspace_root.join(".claw/control/runtime-vault.json"),
            "{}",
        )?;

        let summary = posture_summary(
            workspace_root.join("config/default.toml").to_str().unwrap(),
            workspace_root,
        )?;
        assert!(summary.require_auth);
        assert!(summary.origin_validation);
        assert!(!summary.allowed_origins.is_empty());
        assert!(summary.vault_present);
        assert!(
            summary
                .recommended_actions
                .iter()
                .any(|entry| entry.contains("security generate-keys"))
        );
        Ok(())
    }
}
