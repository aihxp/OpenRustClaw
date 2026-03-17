//! Marketplace client for skill discovery and package download.

use serde::{Deserialize, Serialize};

use openrustclaw_core::error::{Error, Result};

use crate::registry::SkillMetadata;

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
    base_url: String,
    client: reqwest::Client,
}

impl MarketplaceClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Search marketplace for skills.
    pub async fn search(&self, query: &str) -> Result<Vec<MarketplaceListing>> {
        let url = format!("{}/api/v1/skills/search", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[("q", query)])
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to search marketplace: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Marketplace search failed: {}",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| Error::Internal(format!("Failed to read marketplace response: {}", e)))?;

        parse_marketplace_search(&body)
    }

    /// Download a skill from the marketplace.
    pub async fn download(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}/api/v1/skills/{}/{}/download",
            self.base_url, name, version
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to download skill package: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Marketplace download failed: {}",
                response.status()
            )));
        }

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|e| Error::Internal(format!("Failed to read skill package: {}", e)))
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MarketplaceSearchResponse {
    Listings(Vec<MarketplaceListing>),
    Metadata(Vec<SkillMetadata>),
}

fn parse_marketplace_search(body: &str) -> Result<Vec<MarketplaceListing>> {
    match serde_json::from_str::<MarketplaceSearchResponse>(body)
        .map_err(|e| Error::Internal(format!("Failed to parse marketplace results: {}", e)))?
    {
        MarketplaceSearchResponse::Listings(listings) => Ok(listings),
        MarketplaceSearchResponse::Metadata(skills) => Ok(skills
            .into_iter()
            .map(|skill| MarketplaceListing {
                name: skill.name,
                description: skill.description,
                version: skill.version.to_string(),
                author: skill.author,
                downloads: skill.downloads,
                verified: skill.signature.is_some(),
                signature: skill.signature,
            })
            .collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use semver::Version;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
        assert_eq!(client.base_url, "https://marketplace.example.com");
    }

    #[tokio::test]
    async fn test_marketplace_search_supports_listing_payload() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/skills/search"))
            .and(query_param("q", "calendar"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![MarketplaceListing {
                name: "calendar".to_string(),
                description: "Calendar skill".to_string(),
                version: "1.2.3".to_string(),
                author: "Alice".to_string(),
                downloads: 25,
                verified: true,
                signature: Some("sig".to_string()),
            }]))
            .mount(&server)
            .await;

        let client = MarketplaceClient::new(server.uri());
        let result = client.search("calendar").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "calendar");
        assert!(result[0].verified);
    }

    #[tokio::test]
    async fn test_marketplace_search_supports_skill_metadata_payload() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/skills/search"))
            .and(query_param("q", "memory"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![SkillMetadata {
                name: "memory-helper".to_string(),
                version: Version::parse("2.0.0").unwrap(),
                description: "Memory helper".to_string(),
                author: "Bob".to_string(),
                repository: "https://example.com/memory-helper".to_string(),
                license: "MIT".to_string(),
                keywords: vec!["memory".to_string()],
                categories: vec!["productivity".to_string()],
                downloads: 99,
                rating: 4.8,
                rating_count: 20,
                signature: None,
                published_at: Utc::now(),
                updated_at: Utc::now(),
                dependencies: vec![],
                capabilities: vec![],
                min_openrustclaw_version: None,
            }]))
            .mount(&server)
            .await;

        let client = MarketplaceClient::new(server.uri());
        let result = client.search("memory").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].version, "2.0.0");
        assert!(!result[0].verified);
    }

    #[tokio::test]
    async fn test_marketplace_download_returns_bytes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/skills/some-skill/1.0.0/download"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"skill-tarball".to_vec()))
            .mount(&server)
            .await;

        let client = MarketplaceClient::new(server.uri());
        let result = client.download("some-skill", "1.0.0").await.unwrap();
        assert_eq!(result, b"skill-tarball".to_vec());
    }

    #[tokio::test]
    async fn test_marketplace_download_surfaces_http_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/skills/missing/1.0.0/download"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = MarketplaceClient::new(server.uri());
        let result = client.download("missing", "1.0.0").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
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
