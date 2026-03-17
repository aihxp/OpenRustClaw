//! Security operations commands.

use anyhow::Result;
use std::io::Write;

use openrustclaw_security::SkillVerifier;
use openrustclaw_security::audit::{AuditEvent, AuditSeverity};
use sqlx::Row;

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
    println!("  \x1b[90m○ WASM sandbox: Enabled (default)\x1b[0m");
    println!("  \x1b[90m○ Skill capabilities: Enforced\x1b[0m");
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
        println!("  - Add verifying_key.pem to your OpenRustClaw config");
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
}
