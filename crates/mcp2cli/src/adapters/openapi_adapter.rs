//! Adapter for OpenAPI specifications
//!
//! This adapter parses OpenAPI specs and exposes endpoints as tools,
//! providing a unified interface for tool discovery and execution.

use crate::adapters::ToolSourceAdapter;
use crate::discovery::{ParamHelp, ToolHelp, ToolSummary};
use crate::error::{Mcp2CliError, Result};
use async_trait::async_trait;
use openapiv3::{OpenAPI, Operation, Parameter, ParameterSchemaOrContent, ReferenceOr};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Adapter for OpenAPI specifications
pub struct OpenApiAdapter {
    _spec: OpenAPI,
    base_url: String,
    endpoints: Vec<EndpointInfo>,
}

/// Information about an API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EndpointInfo {
    operation_id: String,
    path: String,
    method: HttpMethod,
    summary: Option<String>,
    description: Option<String>,
    parameters: Vec<ParameterInfo>,
    request_body: Option<Value>,
    responses: HashMap<String, Value>,
}

/// HTTP method
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Options => write!(f, "OPTIONS"),
        }
    }
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParameterInfo {
    name: String,
    location: String, // query, path, header, cookie
    description: Option<String>,
    required: bool,
    schema: Option<Value>,
}

impl OpenApiAdapter {
    /// Load an OpenAPI spec from a URL
    pub async fn from_url(url: &str) -> Result<Self> {
        debug!("Loading OpenAPI spec from {}", url);

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| Mcp2CliError::other(format!("Failed to fetch OpenAPI spec: {}", e)))?;

        if !response.status().is_success() {
            return Err(Mcp2CliError::other(format!(
                "Failed to fetch OpenAPI spec: HTTP {}",
                response.status()
            )));
        }

        let content = response.text().await?;
        Self::from_string(&content, Some(url))
    }

    /// Load an OpenAPI spec from a file
    pub async fn from_file(path: &str) -> Result<Self> {
        debug!("Loading OpenAPI spec from file {}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Mcp2CliError::other(format!("Failed to read OpenAPI spec file: {}", e)))?;

        Self::from_string(&content, None)
    }

    /// Parse an OpenAPI spec from a string
    fn from_string(content: &str, source_url: Option<&str>) -> Result<Self> {
        let spec: OpenAPI = serde_yaml::from_str(content)
            .or_else(|_| serde_json::from_str(content))
            .map_err(|e| Mcp2CliError::openapi(format!("Failed to parse OpenAPI spec: {}", e)))?;

        let base_url = Self::extract_base_url(&spec, source_url);
        let endpoints = Self::extract_endpoints(&spec);

        info!(
            "Loaded OpenAPI spec with {} endpoints from {}",
            endpoints.len(),
            source_url.unwrap_or("file")
        );

        Ok(Self {
            _spec: spec,
            base_url,
            endpoints,
        })
    }

    /// Extract base URL from spec
    fn extract_base_url(spec: &OpenAPI, source_url: Option<&str>) -> String {
        // Try servers array first
        if !spec.servers.is_empty() {
            if let Some(server) = spec.servers.first() {
                return server.url.clone();
            }
        }

        // Fall back to source URL
        if let Some(url) = source_url {
            // Extract base URL from spec URL
            if let Ok(parsed) = url::Url::parse(url) {
                let base = format!("{}://{}{}", 
                    parsed.scheme(),
                    parsed.host_str().unwrap_or("localhost"),
                    parsed.port().map_or_else(String::new, |p| format!(":{}", p))
                );
                return base;
            }
        }

        "http://localhost".to_string()
    }

    /// Extract endpoints from the spec
    fn extract_endpoints(spec: &OpenAPI) -> Vec<EndpointInfo> {
        let mut endpoints = Vec::new();

        for (path, path_item_ref) in spec.paths.iter() {
            let path_item = match path_item_ref {
                ReferenceOr::Item(item) => item,
                ReferenceOr::Reference { reference } => {
                    warn!("Skipping referenced path item: {}", reference);
                    continue;
                }
            };

            // Extract operations for each HTTP method
            let operations = vec![
                (HttpMethod::Get, path_item.get.as_ref()),
                (HttpMethod::Post, path_item.post.as_ref()),
                (HttpMethod::Put, path_item.put.as_ref()),
                (HttpMethod::Delete, path_item.delete.as_ref()),
                (HttpMethod::Patch, path_item.patch.as_ref()),
                (HttpMethod::Head, path_item.head.as_ref()),
                (HttpMethod::Options, path_item.options.as_ref()),
            ];

            for (method, operation_opt) in operations {
                if let Some(operation) = operation_opt {
                    if let Some(endpoint) = Self::extract_endpoint(path, method, operation) {
                        endpoints.push(endpoint);
                    }
                }
            }
        }

        endpoints
    }

    /// Extract a single endpoint
    fn extract_endpoint(path: &str, method: HttpMethod, operation: &Operation) -> Option<EndpointInfo> {
        let operation_id = operation.operation_id.clone()
            .or_else(|| {
                // Generate operation ID from method and path
                let clean_path = path.replace('/', "_").replace(['{', '}'], "");
                Some(format!("{:?}_{}", method, clean_path).to_lowercase())
            })?;

        let parameters: Vec<ParameterInfo> = operation
            .parameters
            .iter()
            .filter_map(|p| Self::extract_parameter(p))
            .collect();

        Some(EndpointInfo {
            operation_id,
            path: path.to_string(),
            method,
            summary: operation.summary.clone(),
            description: operation.description.clone(),
            parameters,
            request_body: None, // Simplified for now
            responses: HashMap::new(), // Simplified for now
        })
    }

    /// Extract parameter info
    fn extract_parameter(param_ref: &ReferenceOr<Parameter>) -> Option<ParameterInfo> {
        use openapiv3::Parameter::{Header, Path, Query, Cookie};
        
        let param = match param_ref {
            ReferenceOr::Item(p) => p,
            ReferenceOr::Reference { .. } => return None,
        };

        // Determine the location based on parameter variant
        let location = match param {
            Query { .. } => "query",
            Path { .. } => "path",
            Header { .. } => "header",
            Cookie { .. } => "cookie",
        };

        let schema = match &param.clone().parameter_data().format {
            ParameterSchemaOrContent::Schema(schema_ref) => {
                match schema_ref {
                    ReferenceOr::Item(schema) => {
                        Some(serde_json::to_value(schema).unwrap_or_default())
                    }
                    ReferenceOr::Reference { .. } => None,
                }
            }
            _ => None,
        };

        Some(ParameterInfo {
            name: param.clone().parameter_data().name.clone(),
            location: location.to_string(),
            description: param.clone().parameter_data().description.clone(),
            required: param.clone().parameter_data().required,
            schema,
        })
    }

    /// Convert endpoint info to ToolSummary
    fn endpoint_to_summary(endpoint: &EndpointInfo) -> ToolSummary {
        let description = endpoint
            .summary
            .as_ref()
            .or(endpoint.description.as_ref())
            .cloned()
            .unwrap_or_else(|| format!("{} {}", endpoint.method, endpoint.path));

        ToolSummary::new(&endpoint.operation_id, description)
    }

    /// Convert endpoint info to ToolHelp
    fn endpoint_to_help(endpoint: &EndpointInfo) -> ToolHelp {
        let description = endpoint
            .description
            .as_ref()
            .or(endpoint.summary.as_ref())
            .cloned()
            .unwrap_or_else(|| format!("{} {}", endpoint.method, endpoint.path));

        // Build usage string
        let usage = format!(
            "{} {} {}",
            endpoint.operation_id,
            endpoint.method,
            endpoint.path
        );

        // Convert parameters
        let parameters: Vec<ParamHelp> = endpoint
            .parameters
            .iter()
            .map(|p| {
                let type_name = p
                    .schema
                    .as_ref()
                    .and_then(|s| s.get("type").and_then(|t| t.as_str()))
                    .unwrap_or("string")
                    .to_string();

                ParamHelp::new(&p.name, p.description.clone().unwrap_or_default(), type_name, p.required)
            })
            .collect();

        ToolHelp::new(&endpoint.operation_id, description, usage, parameters)
    }

    /// Get endpoint by operation ID
    fn get_endpoint(&self, operation_id: &str) -> Option<&EndpointInfo> {
        self.endpoints.iter().find(|e| e.operation_id == operation_id)
    }

    /// Build full URL for an endpoint
    fn build_url(&self, endpoint: &EndpointInfo, args: &Value) -> String {
        let mut url = format!("{}{}", self.base_url, endpoint.path);

        // Replace path parameters
        for param in &endpoint.parameters {
            if param.location == "path" {
                let placeholder = format!("{{{}}}", param.name);
                if let Some(value) = args.get(&param.name) {
                    let value_str = match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    url = url.replace(&placeholder, &value_str);
                }
            }
        }

        // Add query parameters
        let query_params: Vec<String> = endpoint
            .parameters
            .iter()
            .filter(|p| p.location == "query")
            .filter_map(|p| {
                args.get(&p.name).map(|v| {
                    let value_str = match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    format!("{}={}", p.name, urlencoding::encode(&value_str))
                })
            })
            .collect();

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        url
    }

    /// Build request body from arguments
    fn build_body(&self, endpoint: &EndpointInfo, args: &Value) -> Option<Value> {
        // Filter out path and query parameters
        let body_params: serde_json::Map<String, Value> = args
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter(|(key, _)| {
                        !endpoint.parameters.iter().any(|p| {
                            &p.name == *key && (p.location == "path" || p.location == "query")
                        })
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        if body_params.is_empty() {
            None
        } else {
            Some(Value::Object(body_params))
        }
    }
}

#[async_trait]
impl ToolSourceAdapter for OpenApiAdapter {
    async fn list_tools(&self) -> Result<Vec<ToolSummary>> {
        Ok(self.endpoints.iter().map(Self::endpoint_to_summary).collect())
    }

    async fn get_tool_help(&self, operation_id: &str) -> Result<ToolHelp> {
        self.get_endpoint(operation_id)
            .map(Self::endpoint_to_help)
            .ok_or_else(|| Mcp2CliError::EndpointNotFound(operation_id.to_string()))
    }

    async fn execute_tool(&self, operation_id: &str, args: Value) -> Result<String> {
        let endpoint = self
            .get_endpoint(operation_id)
            .ok_or_else(|| Mcp2CliError::EndpointNotFound(operation_id.to_string()))?;

        let url = self.build_url(endpoint, &args);
        let body = self.build_body(endpoint, &args);

        info!(
            operation_id = %operation_id,
            method = %endpoint.method,
            url = %url,
            "Executing OpenAPI endpoint"
        );

        let client = reqwest::Client::new();
        let request = match endpoint.method {
            HttpMethod::Get => client.get(&url),
            HttpMethod::Post => {
                let mut req = client.post(&url);
                if let Some(b) = body {
                    req = req.json(&b);
                }
                return self.send_request(req).await;
            }
            HttpMethod::Put => {
                let mut req = client.put(&url);
                if let Some(b) = body {
                    req = req.json(&b);
                }
                return self.send_request(req).await;
            }
            HttpMethod::Delete => client.delete(&url),
            HttpMethod::Patch => {
                let mut req = client.patch(&url);
                if let Some(b) = body {
                    req = req.json(&b);
                }
                return self.send_request(req).await;
            }
            HttpMethod::Head => client.head(&url),
            HttpMethod::Options => client.request(reqwest::Method::OPTIONS, &url),
        };

        self.send_request(request).await
    }
}

impl OpenApiAdapter {
    /// Send HTTP request and return response
    async fn send_request(&self, request: reqwest::RequestBuilder) -> Result<String> {
        let response = request
            .send()
            .await
            .map_err(|e| Mcp2CliError::Http(e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| Mcp2CliError::Http(e))?;

        if !status.is_success() {
            return Err(Mcp2CliError::other(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OPENAPI: &str = r#"{
  "openapi": "3.0.0",
  "info": {
    "title": "Test API",
    "version": "1.0.0"
  },
  "paths": {
    "/users": {
      "get": {
        "operationId": "listUsers",
        "summary": "List all users",
        "parameters": [
          {
            "name": "limit",
            "in": "query",
            "schema": {
              "type": "integer"
            }
          }
        ],
        "responses": {
          "200": {
            "description": "Success"
          }
        }
      },
      "post": {
        "operationId": "createUser",
        "summary": "Create a new user",
        "responses": {
          "201": {
            "description": "Created"
          }
        }
      }
    },
    "/users/{id}": {
      "get": {
        "operationId": "getUser",
        "summary": "Get user by ID",
        "parameters": [
          {
            "name": "id",
            "in": "path",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "description": "Success"
          }
        }
      }
    }
  }
}"#;

    #[test]
    fn test_parse_openapi() {
        let adapter = OpenApiAdapter::from_string(SAMPLE_OPENAPI, None).unwrap();
        assert_eq!(adapter.endpoints.len(), 3);
        assert_eq!(adapter.base_url, "http://localhost");
    }

    #[test]
    fn test_list_endpoints() {
        let adapter = OpenApiAdapter::from_string(SAMPLE_OPENAPI, None).unwrap();
        let summaries: Vec<_> = adapter.endpoints.iter().map(OpenApiAdapter::endpoint_to_summary).collect();
        
        assert_eq!(summaries.len(), 3);
        assert!(summaries.iter().any(|s| s.name == "listUsers"));
        assert!(summaries.iter().any(|s| s.name == "createUser"));
        assert!(summaries.iter().any(|s| s.name == "getUser"));
    }

    #[test]
    fn test_build_url_with_path_param() {
        let adapter = OpenApiAdapter::from_string(SAMPLE_OPENAPI, None).unwrap();
        let endpoint = adapter.get_endpoint("getUser").unwrap();
        
        let args = serde_json::json!({"id": "123"});
        let url = adapter.build_url(endpoint, &args);
        
        assert!(url.contains("/users/123"));
    }

    #[test]
    fn test_build_url_with_query_param() {
        let adapter = OpenApiAdapter::from_string(SAMPLE_OPENAPI, None).unwrap();
        let endpoint = adapter.get_endpoint("listUsers").unwrap();
        
        let args = serde_json::json!({"limit": 10});
        let url = adapter.build_url(endpoint, &args);
        
        assert!(url.contains("?limit=10"));
    }
}
