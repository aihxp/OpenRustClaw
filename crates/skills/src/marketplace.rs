//! Marketplace client scaffolding.

use serde::{Deserialize, Serialize};

use openrustclaw_core::error::Result;

/// A skill listing from the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceListing {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub downloads: u64,
    pub verified: bool,
    pub signature: Option<String>,
}

/// Client for the skill marketplace.
pub struct MarketplaceClient {
    #[allow(dead_code)]
    base_url: String,
}

impl MarketplaceClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Search marketplace for skills.
    pub async fn search(&self, _query: &str) -> Result<Vec<MarketplaceListing>> {
        Err(openrustclaw_core::error::Error::Internal(
            "MarketplaceClient search is not implemented; use ClawHubRegistry for live registry access".to_string(),
        ))
    }

    /// Download a skill from the marketplace.
    pub async fn download(&self, _name: &str, _version: &str) -> Result<Vec<u8>> {
        Err(openrustclaw_core::error::Error::Internal(
            "MarketplaceClient download is not implemented; use ClawHubRegistry for live registry access".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MarketplaceListing tests ────────────────────────────────────────

    #[test]
    fn test_marketplace_listing_serialization_roundtrip() {
        let listing = MarketplaceListing {
            name: "cool-skill".to_string(),
            description: "A very cool skill".to_string(),
            version: "1.0.0".to_string(),
            author: "Alice".to_string(),
            downloads: 5000,
            verified: true,
            signature: Some("sig123".to_string()),
        };

        let json = serde_json::to_string(&listing).unwrap();
        let deserialized: MarketplaceListing = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "cool-skill");
        assert_eq!(deserialized.description, "A very cool skill");
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.author, "Alice");
        assert_eq!(deserialized.downloads, 5000);
        assert!(deserialized.verified);
        assert_eq!(deserialized.signature, Some("sig123".to_string()));
    }

    #[test]
    fn test_marketplace_listing_unverified_no_signature() {
        let listing = MarketplaceListing {
            name: "unsigned-skill".to_string(),
            description: "Not verified".to_string(),
            version: "0.1.0".to_string(),
            author: "Unknown".to_string(),
            downloads: 0,
            verified: false,
            signature: None,
        };

        let json = serde_json::to_string(&listing).unwrap();
        let deserialized: MarketplaceListing = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.verified);
        assert!(deserialized.signature.is_none());
        assert_eq!(deserialized.downloads, 0);
    }

    #[test]
    fn test_marketplace_listing_clone() {
        let listing = MarketplaceListing {
            name: "clonable".to_string(),
            description: "Can clone".to_string(),
            version: "1.0.0".to_string(),
            author: "Bob".to_string(),
            downloads: 100,
            verified: true,
            signature: Some("abc".to_string()),
        };

        let cloned = listing.clone();
        assert_eq!(cloned.name, listing.name);
        assert_eq!(cloned.downloads, listing.downloads);
        assert_eq!(cloned.verified, listing.verified);
    }

    #[test]
    fn test_marketplace_listing_debug_format() {
        let listing = MarketplaceListing {
            name: "debug-skill".to_string(),
            description: "Test debug".to_string(),
            version: "1.0.0".to_string(),
            author: "Dev".to_string(),
            downloads: 42,
            verified: false,
            signature: None,
        };

        let debug = format!("{:?}", listing);
        assert!(debug.contains("debug-skill"));
        assert!(debug.contains("42"));
    }

    // ── MarketplaceClient tests ─────────────────────────────────────────

    #[test]
    fn test_marketplace_client_creation() {
        let client = MarketplaceClient::new("https://marketplace.example.com".to_string());
        // If it was created without panicking, it works
        let _ = client;
    }

    #[tokio::test]
    async fn test_marketplace_search_returns_error() {
        let client = MarketplaceClient::new("https://marketplace.example.com".to_string());
        let result = client.search("anything").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ClawHubRegistry"),
            "Expected ClawHubRegistry guidance, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_marketplace_download_returns_error() {
        let client = MarketplaceClient::new("https://marketplace.example.com".to_string());
        let result = client.download("some-skill", "1.0.0").await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ClawHubRegistry"),
            "Expected ClawHubRegistry guidance, got: {}",
            err
        );
    }

    #[test]
    fn test_marketplace_listing_deserialization_from_json() {
        let json = r#"{
            "name": "from-json",
            "description": "Deserialized from raw JSON",
            "version": "2.0.0",
            "author": "JSON Author",
            "downloads": 999,
            "verified": true,
            "signature": "xyz789"
        }"#;

        let listing: MarketplaceListing = serde_json::from_str(json).unwrap();
        assert_eq!(listing.name, "from-json");
        assert_eq!(listing.version, "2.0.0");
        assert_eq!(listing.downloads, 999);
        assert!(listing.verified);
        assert_eq!(listing.signature, Some("xyz789".to_string()));
    }

    #[test]
    fn test_marketplace_listing_deserialization_with_null_signature() {
        let json = r#"{
            "name": "null-sig",
            "description": "Null signature field",
            "version": "1.0.0",
            "author": "Author",
            "downloads": 0,
            "verified": false,
            "signature": null
        }"#;

        let listing: MarketplaceListing = serde_json::from_str(json).unwrap();
        assert!(listing.signature.is_none());
    }
}
