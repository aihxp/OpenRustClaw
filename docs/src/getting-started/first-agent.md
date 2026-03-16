# Creating Your First Agent

This guide walks you through creating a custom AI agent with OpenRustClaw. You'll learn how to configure providers, add custom tools, set up memory, and deploy your agent.

---

## 🎯 What You'll Build

A **Code Review Agent** that:
- Analyzes code for bugs and improvements
- Remembers your coding preferences
- Schedules daily code review reminders
- Integrates with your Git workflow

---

## 📁 Step 1: Create Agent Configuration

Create a new directory for your agent:

```bash
mkdir -p ~/my-agents/code-reviewer-agent
cd ~/my-agents/code-reviewer-agent
```

### Create `agent.toml`

```toml
[agent]
name = "code-reviewer"
description = "AI-powered code review assistant"
version = "1.0.0"

# LLM Configuration
[agent.llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
temperature = 0.2  # Lower for more consistent reviews
max_tokens = 4096

# Fallback chain
[[agent.llm.fallbacks]]
provider = "openai"
model = "gpt-4o"

[[agent.llm.fallbacks]]
provider = "openrouter"
model = "anthropic/claude-3.5-sonnet"

# System prompt
[agent.prompt]
system = """
You are an expert code reviewer with deep knowledge of software engineering best practices.

Your responsibilities:
1. Identify bugs, security issues, and performance problems
2. Suggest improvements following clean code principles
3. Check for test coverage and documentation
4. Respect the user's coding style preferences (stored in core memory)

When reviewing code:
- Be constructive and specific
- Explain WHY something is an issue
- Provide concrete suggestions with code examples
- Prioritize issues by severity (critical, warning, suggestion)
"""

# Memory configuration
[agent.memory]
core_memory_budget = 500  # tokens
enable_recall = true
enable_archive = true

# Tool permissions
[agent.tools]
allowed = ["memory_search", "memory_store", "file_read", "shell_exec"]
blocked = ["file_write"]  # Read-only for safety
```

---

## 🛠️ Step 2: Add Custom Tools

Create a custom tool for code analysis.

### Create `tools/code_analyzer.rs`

```rust
use async_trait::async_trait;
use openrustclaw_core::traits::{Tool, ToolContext};
use openrustclaw_core::types::{SkillCapability, ToolOutput};
use serde_json::Value;

pub struct CodeAnalyzerTool;

#[async_trait]
impl Tool for CodeAnalyzerTool {
    fn name(&self) -> &str {
        "analyze_code"
    }

    fn description(&self) -> &str {
        "Analyze source code for issues, style violations, and improvements. \
         Returns structured analysis with severity levels."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the source file to analyze"
                },
                "language": {
                    "type": "string",
                    "enum": ["rust", "python", "javascript", "typescript", "go"],
                    "description": "Programming language of the code"
                }
            },
            "required": ["file_path", "language"]
        })
    }

    fn capabilities_required(&self) -> Vec<SkillCapability> {
        vec![SkillCapability::FileRead]
    }

    async fn execute(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput, Error> {
        let file_path = input["file_path"].as_str()
            .ok_or_else(|| Error::invalid_input("file_path required"))?;
        let language = input["language"].as_str()
            .ok_or_else(|| Error::invalid_input("language required"))?;

        // Read the file
        let code = tokio::fs::read_to_string(file_path).await
            .map_err(|e| Error::execution_failed(format!("Failed to read file: {}", e)))?;

        // Run language-specific analysis
        let analysis = match language {
            "rust" => analyze_rust(&code).await?,
            "python" => analyze_python(&code).await?,
            "javascript" | "typescript" => analyze_js(&code).await?,
            _ => return Err(Error::unsupported_language(language)),
        };

        Ok(ToolOutput {
            tool_call_id: ctx.tool_call_id.clone(),
            content: serde_json::to_string_pretty(&analysis)?,
            is_error: false,
        })
    }
}

async fn analyze_rust(code: &str) -> Result<Value, Error> {
    let mut issues = vec![];

    // Check for unwrap() usage
    if code.contains(".unwrap()") {
        let count = code.matches(".unwrap()").count();
        issues.push(serde_json::json!({
            "severity": "warning",
            "message": format!("Found {} unwrap() calls. Consider using proper error handling.", count),
            "suggestion": "Use `?` operator or `match` for error handling instead of unwrap()"
        }));
    }

    // Check for TODO comments
    if code.contains("TODO") || code.contains("FIXME") {
        issues.push(serde_json::json!({
            "severity": "suggestion",
            "message": "Code contains TODO/FIXME comments",
            "suggestion": "Address TODOs before merging or create tickets to track them"
        }));
    }

    // Check for unsafe blocks
    if code.contains("unsafe {") {
        issues.push(serde_json::json!({
            "severity": "critical",
            "message": "Unsafe code detected",
            "suggestion": "Ensure unsafe blocks are necessary and properly documented with SAFETY comments"
        }));
    }

    Ok(serde_json::json!({
        "language": "rust",
        "total_lines": code.lines().count(),
        "issues": issues,
        "issue_count": issues.len()
    }))
}
```

### Register Your Tool

In your agent configuration, add:

```toml
[agent.custom_tools]
paths = ["./tools/code_analyzer.rs"]
```

---

## 🧠 Step 3: Configure Memory

Set up memory to remember your coding preferences.

### Create `memory/core.json`

```json
{
  "coding_style": {
    "language_preferences": ["Rust", "Python", "TypeScript"],
    "formatting": {
      "rust": "rustfmt default",
      "python": "black --line-length 88",
      "typescript": "prettier default"
    }
  },
  "review_preferences": {
    "focus_areas": ["security", "performance", "maintainability"],
    "ignore_patterns": ["*.generated.rs", "*.pb.go"],
    "severity_threshold": "warning"
  },
  "project_context": {
    "current_project": "OpenRustClaw",
    "team_size": 5,
    "deployment_environment": "Kubernetes"
  }
}
```

### Load Core Memory

```bash
# Use CLI to load core memory
openrustclaw memory load-core --file ./memory/core.json --namespace code-reviewer
```

---

## ⏰ Step 4: Set Up Scheduled Tasks

Create a scheduled job for daily code reviews.

### Create `scheduler/review_reminder.yaml`

```yaml
jobs:
  - id: daily-code-review
    name: "Daily Code Review Reminder"
    description: "Remind user to review pending PRs"
    
    # Trigger: Every weekday at 9 AM
    trigger:
      type: cron
      expression: "0 9 * * 1-5"
      timezone: "America/New_York"
    
    # Workflow to execute
    workflow:
      name: "review_reminder"
      steps:
        - name: check_pending_prs
          tool: shell_exec
          input:
            command: "gh pr list --repo myorg/myrepo --state open --json number,title,author"
        
        - name: analyze_prs
          llm:
            prompt: "Analyze these PRs and prioritize which to review first: {{check_pending_prs.output}}"
        
        - name: send_reminder
          tool: notify_user
          input:
            message: "{{analyze_prs.response}}"
    
    # Retry policy
    retry:
      max_attempts: 3
      backoff: exponential
      initial_delay_ms: 1000
    
    # Idempotency
    idempotency_key: "daily-review-{{trigger.fire_time|date:'%Y-%m-%d'}}"
```

### Register the Job

```bash
openrustclaw schedule create --file ./scheduler/review_reminder.yaml
```

---

## 🚀 Step 5: Start Your Agent

### Option A: CLI Mode

```bash
# Start with your custom configuration
openrustclaw start --config ./agent.toml

# In another terminal
openrustclaw chat --agent code-reviewer
```

### Option B: Programmatic

Create `main.rs`:

```rust
use openrustclaw_agent::{Agent, AgentConfig};
use openrustclaw_core::types::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = AgentConfig::from_file("./agent.toml").await?;
    
    // Create agent
    let agent = Agent::new(config).await?;
    
    // Start a session
    let session = agent.create_session("user_123").await?;
    
    // Send a message
    let response = agent
        .chat(&session.id, Message::user("Please review src/main.rs"))
        .await?;
    
    println!("Agent: {}", response.content);
    
    Ok(())
}
```

Run it:

```bash
cargo run --example code-reviewer
```

---

## 🧪 Step 6: Test Your Agent

### Test 1: Code Review

```
You: Please review this Rust code:
```rust
fn main() {
    let data = fetch_data().unwrap();
    println!("{:?}", data);
}

// TODO: Add error handling
```

Expected response:
```
I'll analyze this code for you.

[tool: analyze_code] {"file_path": "example.rs", "language": "rust"}

**Issues Found:**

🔴 **Critical**: None

⚠️ **Warnings**:
1. Using `.unwrap()` on `fetch_data()` result
   - Suggestion: Use `match` or `?` operator for proper error handling
   - Example:
     ```rust
     fn main() -> Result<(), Box<dyn std::error::Error>> {
         let data = fetch_data()?;
         println!("{:?}", data);
         Ok(())
     }
     ```

💡 **Suggestions**:
2. TODO comment present - consider creating a ticket
```

### Test 2: Memory Recall

```
You: What are my code formatting preferences?

Agent: Based on your core memory, your formatting preferences are:

- **Rust**: rustfmt default
- **Python**: black --line-length 88
- **TypeScript**: prettier default

You mentioned preferring Rust, Python, and TypeScript as your primary languages.
```

### Test 3: Scheduled Task

```bash
# List scheduled jobs
openrustclaw schedule list

# Manually trigger for testing
openrustclaw schedule run daily-code-review --now

# Check job history
openrustclaw schedule history daily-code-review
```

---

## 📦 Step 7: Deployment Options

### Option 1: Local Development

```bash
# Run with hot reload
cargo watch -x 'run --bin code-reviewer'
```

### Option 2: Docker

Create `Dockerfile`:

```dockerfile
FROM rust:1.85 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/code-reviewer /usr/local/bin/
COPY agent.toml /etc/code-reviewer/
ENV AGENT_CONFIG=/etc/code-reviewer/agent.toml
EXPOSE 18789
CMD ["code-reviewer"]
```

Build and run:

```bash
docker build -t code-reviewer-agent .
docker run -p 18789:18789 --env-file .env code-reviewer-agent
```

### Option 3: Kubernetes

Create `deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: code-reviewer-agent
spec:
  replicas: 1
  selector:
    matchLabels:
      app: code-reviewer
  template:
    metadata:
      labels:
        app: code-reviewer
    spec:
      containers:
      - name: agent
        image: code-reviewer-agent:latest
        ports:
        - containerPort: 18789
        envFrom:
        - secretRef:
            name: api-keys
        volumeMounts:
        - name: agent-config
          mountPath: /etc/code-reviewer
      volumes:
      - name: agent-config
        configMap:
          name: agent-config
---
apiVersion: v1
kind: Service
metadata:
  name: code-reviewer-service
spec:
  selector:
    app: code-reviewer
  ports:
  - port: 80
    targetPort: 18789
```

Deploy:

```bash
kubectl apply -f deployment.yaml
```

### Option 4: systemd Service (Linux)

Create `/etc/systemd/system/code-reviewer.service`:

```ini
[Unit]
Description=Code Reviewer Agent
After=network.target

[Service]
Type=simple
User=agent
WorkingDirectory=/opt/code-reviewer
ExecStart=/opt/code-reviewer/target/release/code-reviewer
Restart=always
RestartSec=10
Environment=AGENT_CONFIG=/opt/code-reviewer/agent.toml
EnvironmentFile=/opt/code-reviewer/.env

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable code-reviewer
sudo systemctl start code-reviewer
sudo systemctl status code-reviewer
```

---

## 📊 Step 8: Monitoring

### Check Agent Health

```bash
# Health check
curl http://localhost:18789/health

# Metrics
curl http://localhost:18789/metrics
```

### View Logs

```bash
# Follow logs
journalctl -u code-reviewer -f

# Or with Docker
docker logs -f code-reviewer-agent
```

### LangSmith Traces

Visit [smith.langchain.com](https://smith.langchain.com) to view:
- Conversation traces
- Tool call latencies
- Token usage statistics
- Error rates

---

## 🎉 Congratulations!

You've created a fully functional custom agent with:

- ✅ Custom LLM configuration with fallback chain
- ✅ Custom tool (code analyzer)
- ✅ 3-tier memory system
- ✅ Scheduled tasks
- ✅ Multiple deployment options

---

## 🚀 Next Steps

1. **[Memory System Deep Dive](../guides/memory.md)** — Optimize memory usage
2. **[Security Hardening](../guides/security.md)** — Secure your agent
3. **[MCP Integration](../guides/mcp-servers.md)** — Connect external tools
