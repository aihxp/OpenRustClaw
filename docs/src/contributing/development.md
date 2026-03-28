# Development Setup

This guide covers setting up your development environment for contributing to OpenRustClaw.

---

## 📋 Prerequisites

### Required

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.85+ | Core framework |
| protoc | 3.20+ | Protocol buffers |
| Node.js | 18+ | MCP servers (optional) |

### Recommended

| Tool | Purpose |
|------|---------|
| `cargo-watch` | Auto-rebuild on changes |
| `just` | Task runner |
| `cargo-nextest` | Better test runner |
| `mold` | Faster linker (Linux) |
| `sccache` | Compiler cache |
| Python 3.11+ | Optional compatibility-sidecar development |

---

## 🛠️ Development Environment Setup

### 1. Clone the Repository

```bash
git clone https://github.com/openrustclaw/openrustclaw.git
cd OpenRustClaw
```

### 2. Install Rust Dependencies

```bash
# Update to latest stable
rustup update stable

# Install useful tools
cargo install cargo-watch cargo-nextest just

# Optional: faster builds
# Linux: install mold linker
cargo install mold linker

# Optional: compiler cache
cargo install sccache
export RUSTC_WRAPPER=sccache
```

### 3. Optional: Set Up the Compatibility Sidecar

```bash
cd sidecar
python3 -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -e ".[dev]"

# Verify installation
python src/server.py --help
cd ..
```

### 4. Install Protocol Buffers

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt install protobuf-compiler

# Verify
protoc --version  # Should be 3.20+
```

### 5. Configure Environment

```bash
# Copy example environment
cp .env.example .env

# Edit with your API keys
nano .env
```

Minimum required:

```bash
ANTHROPIC_API_KEY=sk-ant-...
# or
OPENAI_API_KEY=sk-...
# or
OPENROUTER_API_KEY=sk-or-...
```

---

## 🏗️ Building

### Full Build

```bash
# Build entire workspace
cargo build --workspace

# Release build (optimized)
cargo build --workspace --release

# Package a versioned release tarball for a target triple
scripts/build-release-artifacts.sh --target x86_64-unknown-linux-gnu

# Run the shipped runtime resource-budget regression check
scripts/check-runtime-budgets.sh
```

### Building Specific Crates

```bash
# Build only the core crate
cargo build -p openrustclaw-core

# Build only the CLI
cargo build -p openrustclaw-cli
```

### Watch Mode

```bash
# Auto-rebuild on changes
cargo watch -x 'build --workspace'

# Auto-run tests on changes
cargo watch -x 'test --workspace'
```

---

## 🧪 Running Tests

### Full Test Suite

```bash
# Run all tests
cargo test --workspace

# Run with nextest (faster, better output)
cargo nextest run --workspace

# Run tests for specific crate
cargo test -p openrustclaw-core
```

### Test Categories

```bash
# Unit tests only
cargo test --workspace --lib

# Integration tests only
cargo test --workspace --test '*'

# Doctests
cargo test --workspace --doc
```

### Test Filters

```bash
# Run tests matching pattern
cargo test memory
cargo test provider::anthropic

# Run ignored tests (slow tests)
cargo test --workspace -- --ignored

# Run with output
cargo test --workspace -- --nocapture
```

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html

# View report
open tarpaulin-report.html
```

---

## 📝 Code Style

### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check
```

### Linting

```bash
# Run clippy
cargo clippy --workspace --all-targets --all-features

# Fix auto-fixable issues
cargo clippy --workspace --fix
```

### Pre-commit Checks

```bash
# Run before committing
just pre-commit

# Or manually:
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo test --workspace
```

---

## 🔧 Development Workflow

### Typical Development Session

```bash
# Terminal 1: Start the Rust runtime with watch
cargo watch -x 'run --bin openrustclaw -- start'

# Terminal 2: Run tests on changes
cargo watch -x 'test --workspace'

# Terminal 3: Make changes and test
cargo run --bin openrustclaw -- assistant --provider anthropic
```

If you are touching the bounded compatibility sidecar, run it in a separate terminal only for that work:

```bash
cd sidecar
source .venv/bin/activate
python src/server.py
```

### Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin openrustclaw -- chat

# Enable tracing for specific modules
RUST_LOG=openrustclaw_memory=debug,openrustclaw_agent=trace cargo run ...

# Full backtrace on panic
RUST_BACKTRACE=1 cargo run ...
RUST_BACKTRACE=full cargo run ...
```

### Profiling

```bash
# CPU profiling with samply
cargo install samply
cargo build --release
samply record ./target/release/openrustclaw start

# Memory profiling with dhat
cargo build --features dhat-heap
./target/debug/openrustclaw start
```

The shipped CI now also runs `scripts/check-runtime-budgets.sh` to guard release-binary size, CLI startup latency, and idle gateway RSS on Linux, and `.github/workflows/release-binaries.yml` publishes packaged tarballs for the declared Linux/macOS x86_64 and ARM64 release targets.

---

## 📦 Adding Dependencies

### Adding to a Crate

```bash
# Add to specific crate
cargo add -p openrustclaw-core serde

# Add with features
cargo add -p openrustclaw-core tokio --features full

# Add dev dependency
cargo add -p openrustclaw-core --dev mockall
```

### Adding to Workspace

Edit `Cargo.toml`:

```toml
[workspace.dependencies]
# Add new dependency here
new-crate = "1.0"
```

Then in crate `Cargo.toml`:

```toml
[dependencies]
new-crate = { workspace = true }
```

---

## 🔨 Creating a New Crate

### 1. Create Directory Structure

```bash
mkdir -p crates/new-crate/src
touch crates/new-crate/src/lib.rs
touch crates/new-crate/Cargo.toml
```

### 2. Add Cargo.toml

```toml
[package]
name = "openrustclaw-new-crate"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true

[dependencies]
openrustclaw-core = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
```

### 3. Add to Workspace

Edit root `Cargo.toml`:

```toml
[workspace]
members = [
    # ... existing crates
    "crates/new-crate",
]
```

### 4. Create Library

```rust
// crates/new-crate/src/lib.rs
//! New crate for OpenRustClaw.
//!
//! Description of what this crate does.

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;
```

### 5. Follow Dependency Order

Ensure your crate only depends on lower-level crates:

```
core → db → [your crate should be here or below]
```

---

## 🐍 Python Sidecar Development

### Running Tests

```bash
cd sidecar
source .venv/bin/activate

# Run tests
pytest

# Run with coverage
pytest --cov=src --cov-report=html

# Type checking
mypy src/

# Linting
ruff check src/
ruff format src/
```

### Adding Dependencies

```bash
# Add to pyproject.toml
cd sidecar
pip install new-package
pip freeze | grep new-package >> requirements.txt
```

### Regenerating Protobuf

```bash
cd sidecar
python -m grpc_tools.protoc \
    -I../proto \
    --python_out=src/proto \
    --grpc_python_out=src/proto \
    ../proto/orchestration.proto \
    ../proto/tracing.proto
```

---

## 🔄 Pull Request Process

### 1. Create a Branch

```bash
git checkout -b feature/my-feature
# or
git checkout -b fix/my-bugfix
```

### 2. Make Changes

Follow the coding conventions:

- Use `tracing` for logging
- Use `thiserror` for errors
- Add tests for new functionality
- Update documentation

### 3. Run Checks

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace --all-targets

# Run tests
cargo test --workspace

# Build documentation
cargo doc --workspace --no-deps
```

### 4. Commit

```bash
git add .
git commit -m "feat: add new feature

Detailed description of what changed and why.

Closes #123"
```

Commit message format:
- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation
- `refactor:` — Code refactoring
- `test:` — Tests
- `chore:` — Maintenance

### 5. Push and Create PR

```bash
git push origin feature/my-feature
```

Then create a pull request on GitHub with:
- Clear description
- Link to related issues
- Screenshots/logs if relevant

---

## 🆘 Troubleshooting

### Build Failures

```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Update dependencies
cargo update

# Check for locked files
rm Cargo.lock
cargo build
```

### Test Failures

```bash
# Run single test with output
cargo test my_test -- --nocapture

# Run with backtrace
RUST_BACKTRACE=1 cargo test my_test

# Check if the optional sidecar is running
lsof -i :50051
```

### Clippy Warnings

```bash
# Fix auto-fixable
cargo clippy --workspace --fix

# Allow specific lint
cargo clippy --workspace -- -A clippy::some_lint
```

### Protobuf Issues

```bash
# Regenerate if .proto files changed
cd sidecar
python -m grpc_tools.protoc -I../proto --python_out=src/proto --grpc_python_out=src/proto ../proto/*.proto

# In Rust:
cargo build  # build.rs will regenerate
```

---

## 📚 Resources

- [Documentation Contract](../../documentation-contract.md)
- [Docs Audit](../planning/docs-audit.md)
- [Architecture Overview](../architecture/overview.md)
- [Rust Core](../architecture/rust-core.md)
- [Testing Guide](./testing.md)
- [API Reference](../api-reference/)

## Documentation Hygiene

When shipped behavior changes:

1. update the canonical doc first
2. update any mdBook mirror or stub that points to it
3. update the relevant planning doc under `docs/` if the shipped-surface claim changed
4. update `docs/docs-audit.md` if the feature-family coverage changed
5. delete stale duplicates instead of leaving them behind

## Greenfield Transition Defaults

During `v1.13`, new code should default to the greenfield transition lane:

1. keep domain contracts in the existing shared crates
2. put new business logic in a reusable application-facing service layer
3. treat large command modules like `start.rs`, `mobile.rs`, `skills.rs`, and `inspect.rs` as adapters unless the task is explicitly a compatibility fix
4. migrate bounded shipped slices one at a time instead of mixing rewrite work with broad feature churn

The first proving slice is setup handoff reporting across onboarding state, inspection, route exposure, and Control UI rendering.

Use the current baseline before widening that slice:

```bash
cargo test -p openrustclaw-cli setup_handoff_summary -- --nocapture
cargo test -p openrustclaw-cli dashboard_includes_setup_handoff_panel -- --nocapture
mdbook build docs
```
