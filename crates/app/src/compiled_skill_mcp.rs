use crate::compiled_skill_overview::CompiledSkillOverviewService;
use openrustclaw_core::error::{Error, Result};
use openrustclaw_skills::{
    CompiledBackgroundService, CompiledSkillArtifact, CompiledSkillStatus,
    compiled_skill_background_services,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CompiledSkillMcpToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct CompiledSkillMcpService {
    root: PathBuf,
}

impl CompiledSkillMcpService {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn artifacts(&self) -> Vec<CompiledSkillArtifact> {
        CompiledSkillOverviewService::new(self.root.clone()).artifacts()
    }

    pub fn executable_components(artifact: &CompiledSkillArtifact) -> Vec<String> {
        CompiledSkillOverviewService::executable_components(artifact)
    }

    pub fn background_services(artifact: &CompiledSkillArtifact) -> Vec<CompiledBackgroundService> {
        compiled_skill_background_services(artifact)
    }

    pub fn tool_specs(&self, artifacts: &[CompiledSkillArtifact]) -> Vec<CompiledSkillMcpToolSpec> {
        if artifacts.is_empty() {
            return Vec::new();
        }

        let mut tools = vec![
            CompiledSkillMcpToolSpec {
                name: "list_compiled_skills".to_string(),
                description: "List compiled skills that are available to the MCP server."
                    .to_string(),
                input_schema: json!({"type": "object", "properties": {}}),
            },
            CompiledSkillMcpToolSpec {
                name: "inspect_compiled_skill".to_string(),
                description: "Inspect one compiled skill artifact bundle from the workspace cache."
                    .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }),
            },
        ];

        for artifact in artifacts {
            tools.push(CompiledSkillMcpToolSpec {
                name: Self::summary_tool_name(artifact),
                description: format!(
                    "Return the token-efficient compiled summary for skill '{}'.",
                    artifact.manifest.name
                ),
                input_schema: json!({"type": "object", "properties": {}}),
            });
            tools.push(CompiledSkillMcpToolSpec {
                name: Self::detail_tool_name(artifact),
                description: format!(
                    "Return the compiled detail bundle for skill '{}'.",
                    artifact.manifest.name
                ),
                input_schema: json!({"type": "object", "properties": {}}),
            });

            let executable_components = Self::executable_components(artifact);
            let background_services = Self::background_services(artifact);
            if !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked)
                && !executable_components.is_empty()
            {
                let mut execute_schema = json!({
                    "type": "object",
                    "properties": {
                        "input": {
                            "description": "Optional JSON value forwarded into the bounded Rust/WASM skill executor."
                        }
                    }
                });
                if executable_components.len() > 1 {
                    execute_schema["properties"]["component"] = json!({
                        "type": "string",
                        "description": "Executable `.wasm` or `.wat` component path from the compiled skill artifact."
                    });
                    execute_schema["required"] = json!(["component"]);
                } else {
                    execute_schema["properties"]["component"] = json!({
                        "type": "string",
                        "description": "Optional executable component path. Omit to use the only available `.wasm`/`.wat` artifact."
                    });
                }
                tools.push(CompiledSkillMcpToolSpec {
                    name: Self::execute_tool_name(artifact),
                    description: format!(
                        "Execute bounded Rust/WASM component{} for skill '{}'.",
                        if executable_components.len() == 1 {
                            format!(" '{}'", executable_components[0])
                        } else {
                            "s".to_string()
                        },
                        artifact.manifest.name
                    ),
                    input_schema: execute_schema,
                });
            }

            if !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked)
                && !background_services.is_empty()
            {
                tools.push(CompiledSkillMcpToolSpec {
                    name: Self::schedule_tool_name(artifact),
                    description: format!(
                        "Schedule a durable background workflow for compiled skill '{}'.",
                        artifact.manifest.name
                    ),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "service": {"type": "string"},
                            "component": {"type": "string"},
                            "input": {
                                "description": "Optional JSON value forwarded into the background workflow executor."
                            },
                            "every_seconds": {"type": "integer", "minimum": 1},
                            "at": {"type": "string", "description": "Optional RFC3339 timestamp for a one-shot execution."},
                            "priority": {"type": "integer"}
                        }
                    }),
                });
            }

            if !matches!(artifact.manifest.status, CompiledSkillStatus::Blocked) {
                for reference in &artifact.manifest.references {
                    tools.push(CompiledSkillMcpToolSpec {
                        name: Self::reference_tool_name(artifact, reference),
                        description: format!(
                            "Read compiled skill reference '{}' from skill '{}'.",
                            reference, artifact.manifest.name
                        ),
                        input_schema: json!({
                            "type": "object",
                            "properties": {
                                "max_chars": {"type": "integer", "minimum": 1}
                            }
                        }),
                    });
                }
            }
        }

        tools
    }

    pub fn summary_payload(&self, artifact: &CompiledSkillArtifact) -> Value {
        json!({
            "manifest": &artifact.manifest,
            "help": {
                "summary": &artifact.help_index.summary,
                "argument_hint": &artifact.help_index.argument_hint,
                "allowed_tools": &artifact.help_index.allowed_tools,
                "capabilities": &artifact.help_index.capabilities,
                "scripts": &artifact.help_index.scripts,
                "references": &artifact.help_index.references,
                "safety_notes": &artifact.help_index.safety_notes,
            },
            "cli": &artifact.cli_schema,
            "mcp": {
                "summary_tool": Self::summary_tool_name(artifact),
                "details_tool": Self::detail_tool_name(artifact),
                "execute_tool": if Self::executable_components(artifact).is_empty() {
                    Value::Null
                } else {
                    json!(Self::execute_tool_name(artifact))
                },
                "schedule_tool": if Self::background_services(artifact).is_empty() {
                    Value::Null
                } else {
                    json!(Self::schedule_tool_name(artifact))
                },
                "executable_components": Self::executable_components(artifact),
                "background_services": Self::background_services(artifact),
                "reference_tools": artifact
                    .manifest
                    .references
                    .iter()
                    .map(|reference| Self::reference_tool_name(artifact, reference))
                    .collect::<Vec<_>>(),
            },
            "scan_report": &artifact.scan_report,
        })
    }

    pub fn detail_payload(&self, artifact: &CompiledSkillArtifact) -> Value {
        let blocked = matches!(artifact.manifest.status, CompiledSkillStatus::Blocked);
        json!({
            "manifest": &artifact.manifest,
            "help_index": {
                "summary": &artifact.help_index.summary,
                "body_excerpt": if blocked { Value::Null } else { json!(&artifact.help_index.body_excerpt) },
                "argument_hint": &artifact.help_index.argument_hint,
                "allowed_tools": &artifact.help_index.allowed_tools,
                "capabilities": &artifact.help_index.capabilities,
                "scripts": &artifact.help_index.scripts,
                "references": &artifact.help_index.references,
                "examples": if blocked { json!([]) } else { json!(&artifact.help_index.examples) },
                "safety_notes": &artifact.help_index.safety_notes,
            },
            "mcp_schema": &artifact.mcp_schema,
            "cli_schema": &artifact.cli_schema,
            "background_services": Self::background_services(artifact),
            "scan_report": &artifact.scan_report,
            "blocked_content_redacted": blocked,
        })
    }

    pub fn read_reference(
        &self,
        artifact: &CompiledSkillArtifact,
        reference: &str,
        max_chars: Option<usize>,
    ) -> Result<Value> {
        let mut payload = CompiledSkillOverviewService::new(self.root.clone())
            .read_reference(artifact, reference, max_chars)?;
        if let Some(object) = payload.as_object_mut() {
            object.insert(
                "skill".to_string(),
                Value::String(artifact.manifest.name.clone()),
            );
        } else {
            return Err(Error::Internal(
                "compiled skill reference payload must be an object".to_string(),
            ));
        }
        Ok(payload)
    }

    pub fn tool_prefix(name: &str) -> String {
        format!("skill.{}", Self::sanitize_name(name))
    }

    pub fn summary_tool_name(artifact: &CompiledSkillArtifact) -> String {
        format!("{}.summary", Self::tool_prefix(&artifact.manifest.name))
    }

    pub fn detail_tool_name(artifact: &CompiledSkillArtifact) -> String {
        format!("{}.details", Self::tool_prefix(&artifact.manifest.name))
    }

    pub fn reference_tool_name(artifact: &CompiledSkillArtifact, reference: &str) -> String {
        format!(
            "{}.reference.{}",
            Self::tool_prefix(&artifact.manifest.name),
            Self::sanitize_name(reference)
        )
    }

    pub fn execute_tool_name(artifact: &CompiledSkillArtifact) -> String {
        format!("{}.execute", Self::tool_prefix(&artifact.manifest.name))
    }

    pub fn schedule_tool_name(artifact: &CompiledSkillArtifact) -> String {
        format!("{}.schedule", Self::tool_prefix(&artifact.manifest.name))
    }

    fn sanitize_name(value: &str) -> String {
        value
            .chars()
            .map(|ch| match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '/' => ch,
                _ => '-',
            })
            .collect::<String>()
            .replace('/', "__")
    }
}

#[cfg(test)]
mod tests {
    use super::CompiledSkillMcpService;

    #[test]
    fn tool_prefix_sanitizes_nested_paths() {
        assert_eq!(
            CompiledSkillMcpService::tool_prefix("demo/reference.md"),
            "skill.demo__reference.md"
        );
    }
}
