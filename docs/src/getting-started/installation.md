# Installation

This guide walks you through installing OpenRustClaw from source, configuring your environment, and verifying your installation.

---

## 📋 Prerequisites

Before installing OpenRustClaw, ensure you have the following dependencies installed:

### Required

| Dependency | Version | Purpose |
|------------|---------|---------|
| **Rust** | 1.85+ | Core framework (stable channel) |
| **protoc** | 3.20+ | Protocol Buffers compiler |
| **SQLite** | 3.35+ | Database (usually pre-installed) |

### Optional but Recommended

| Dependency | Purpose |
|------------|---------|
| **Python** | Optional compatibility sidecar and sidecar-focused development |
| **mdbook** | Build documentation |
| **cargo-watch** | Auto-rebuild during development |
| **just** | Task runner (alternative to make) |

---

## 🦀 Installing Rust

### Linux/macOS

```bash
# Using rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then reload your shell
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should show 1.85.0 or higher
cargo --version
```

### Windows

```powershell
# Download and run rustup-init.exe from https://rustup.rs/
# Or use winget
winget install Rustlang.Rustup

# Verify installation
rustc --version
cargo --version
```

### Updating Rust

```bash
# Update to latest stable
rustup update stable

# Install specific components if needed
rustup component add rustfmt clippy
```

---

## 🐍 Optional Python Compatibility Sidecar

Python is no longer required for the default production or local runtime path. Install it only if you need the bounded compatibility sidecar or are working on the sidecar itself.

### Linux (Ubuntu/Debian)

```bash
# Install Python 3.11+
sudo apt update
sudo apt install python3.11 python3.11-venv python3.11-dev python3-pip

# Verify installation
python3.11 --version
```

### macOS

```bash
# Using Homebrew
brew install python@3.11

# Verify installation
python3 --version
```

### Windows

```powershell
# Download from python.org or use winget
winget install Python.Python.3.11

# Verify installation
python --version
```

### Setting up the Optional Sidecar Virtual Environment

```bash
# Create a virtual environment for the optional sidecar
cd sidecar
python3 -m venv .venv

# Activate it
# Linux/macOS:
source .venv/bin/activate
# Windows:
# .venv\Scripts\activate

# Install dependencies
pip install -e ".[dev]"
```

---

## 📦 Installing protoc

### Linux (Ubuntu/Debian)

```bash
sudo apt install protobuf-compiler

# Verify
protoc --version  # Should show 3.20.0 or higher
```

### macOS

```bash
# Using Homebrew
brew install protobuf

# Verify
protoc --version
```

### Windows

```powershell
# Using Chocolatey
choco install protoc

# Or download from https://github.com/protocolbuffers/protobuf/releases
# Add to PATH manually
```

---

## 📥 Installing OpenRustClaw from Source

### 1. Clone the Repository

```bash
# Clone with SSH
git clone git@github.com:openrustclaw/openrustclaw.git

# Or clone with HTTPS
git clone https://github.com/openrustclaw/openrustclaw.git

# Enter the directory
cd OpenRustClaw
```

### 2. Build the Rust Workspace

```bash
# Build all crates in debug mode (faster compile, slower runtime)
cargo build --workspace

# Or build in release mode (slower compile, faster runtime)
cargo build --workspace --release

# Package a versioned release artifact for the current host target
scripts/build-release-artifacts.sh
```

The first build will take several minutes as it downloads and compiles dependencies. Subsequent builds will be much faster.

Tag builds and manual runs of `.github/workflows/release-binaries.yml` now also package `openrustclaw` tarballs plus `.sha256` files for the declared Linux/macOS x86_64 and ARM64 release targets.

### 3. Verify the Build

```bash
# Check that the CLI binary works
cargo run --bin openrustclaw -- --help

# You should see the CLI help output
```

---

## ⚙️ Configuration Setup

### 1. Copy the Example Environment File

```bash
# Copy the example environment file
cp .env.example .env

# Edit it with your favorite editor
nano .env  # or vim, code, etc.
```

### 2. Configure API Keys

Edit `.env` and add your API keys:

```bash
# === LLM Provider API Keys ===
# At least one provider is required
ANTHROPIC_API_KEY=sk-ant-api03-xxxxxxxxxxxxxxxxxxxxxxxx
OPENAI_API_KEY=sk-proj-xxxxxxxxxxxxxxxxxxxxxxxx
OPENROUTER_API_KEY=sk-or-v1-xxxxxxxxxxxxxxxxxxxxxxxx

# For local models (optional)
OLLAMA_BASE_URL=http://localhost:11434
```

#### Getting API Keys

**Anthropic:**
1. Visit [console.anthropic.com](https://console.anthropic.com)
2. Create an account or sign in
3. Go to "Get API keys"
4. Generate a new key

**OpenAI:**
1. Visit [platform.openai.com](https://platform.openai.com)
2. Create an account or sign in
3. Go to "API keys"
4. Create a new secret key

**OpenRouter:**
1. Visit [openrouter.ai](https://openrouter.ai)
2. Create an account
3. Go to "Keys"
4. Create a new key

### 3. Configure Gateway (Optional)

```bash
# === Gateway Configuration ===
GATEWAY_HOST=127.0.0.1
GATEWAY_PORT=18789
GATEWAY_ALLOWED_ORIGINS=http://localhost:3000,http://127.0.0.1:3000
```

### 4. Configure Database (Optional)

```bash
# === Database ===
DATABASE_URL=sqlite://data/openrustclaw.db
DATABASE_WAL_MODE=true  # Enable for better concurrency
```

### 5. Configure Security (Production)

```bash
# === Security ===
# Generate a random secret (at least 32 characters)
AUTH_SECRET=$(openssl rand -base64 32)

# Or use a passphrase
AUTH_SECRET=your-very-long-and-random-secret-key-here

# For skill signing (optional)
# Generate with: openrustclaw security generate-keys
SKILL_SIGNING_KEY=
```

### 6. Configure Observability (Optional)

```bash
# === LangSmith Observability ===
LANGSMITH_API_KEY=lsv2_xxxxxxxxxxxxxxxxxxxxxxxx
LANGSMITH_PROJECT=openrustclaw
LANGSMITH_ENDPOINT=https://api.smith.langchain.com
```

## Optional Compatibility Sidecar Setup

If you need the legacy compatibility sidecar for migration or workflow experiments, set it up after the Rust workspace is already working:

```bash
cd sidecar
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
python src/server.py --help
```

Get your LangSmith API key at [smith.langchain.com](https://smith.langchain.com).

---

## ✅ Verifying Installation

### 1. Run Diagnostics

```bash
# Run the built-in doctor command
cargo run --bin openrustclaw -- doctor

# Or if installed globally
openrustclaw doctor
```

This checks:
- ✅ Rust toolchain
- ✅ Python installation
- ✅ Protocol buffers
- ✅ Database connectivity
- ✅ API key validity
- ✅ Sidecar connectivity

### 2. Test Provider Connectivity

```bash
# List available models
openrustclaw models list --provider anthropic

# Test a simple completion
openrustclaw chat --provider anthropic --once "Hello, are you working?"
```

### 3. Test Memory System

```bash
# Check memory stats
openrustclaw memory stats

# Store a test memory
openrustclaw chat --once "Remember that my favorite color is blue"

# Search for it
openrustclaw memory search "favorite color"
```

---

## 🔧 Troubleshooting

### Build Issues

#### "linker `cc` not found" (Linux)

```bash
# Install build essentials
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

#### "protoc not found"

```bash
# Install protobuf compiler
# Ubuntu/Debian:
sudo apt install protobuf-compiler

# macOS:
brew install protobuf

# Then rebuild
cargo clean
cargo build --workspace
```

#### Slow compilation

```bash
# Use sccache for faster rebuilds
cargo install sccache
export RUSTC_WRAPPER=sccache

# Or use mold linker (Linux)
cargo install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### Runtime Issues

#### "Failed to connect to sidecar"

This only applies if you intentionally enabled the optional compatibility sidecar.

```bash
# Start the sidecar manually first
cd sidecar
source .venv/bin/activate
python src/server.py

# In another terminal, start the gateway
cargo run --bin openrustclaw -- start
```

#### "Database locked" errors

```bash
# Ensure WAL mode is enabled in .env
DATABASE_WAL_MODE=true

# Or manually enable it
sqlite3 data/openrustclaw.db "PRAGMA journal_mode=WAL;"
```

#### "Rate limited by provider"

```bash
# Configure multiple providers for fallback
# In .env, set keys for multiple providers
# OpenRustClaw will automatically fallback
```

#### "Authentication failed"

```bash
# Check your API keys are valid
openrustclaw doctor --verbose

# Verify key format (no extra spaces)
echo $ANTHROPIC_API_KEY | wc -c  # Should match expected length
```

### Python Sidecar Issues

#### "ModuleNotFoundError: No module named 'langgraph'"

```bash
cd sidecar
source .venv/bin/activate
pip install -e ".[dev]"
```

#### "gRPC connection refused"

This only applies if you are using the optional compatibility sidecar lane.

```bash
# Check if sidecar port is available
lsof -i :50051  # macOS/Linux
netstat -ano | findstr :50051  # Windows

# Change port in .env if needed
SIDECAR_GRPC_PORT=50052
```

---

## 🚀 Next Steps

Now that you have OpenRustClaw installed:

1. **[Quickstart Guide](./quickstart.md)** — Your first 5 minutes with OpenRustClaw
2. **[First Agent](./first-agent.md)** — Create a custom agent
3. **[Architecture Overview](../architecture/overview.md)** — Understand the system design

---

## 📚 Additional Resources

- [Environment Variables Reference](../guides/providers.md)
- [Security Configuration](../guides/security.md)
- [Troubleshooting FAQ](../contributing/development.md#troubleshooting)
