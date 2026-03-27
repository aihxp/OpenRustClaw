# Quickstart Guide

Get up and running with OpenRustClaw in just 5 minutes. This guide walks you through the self-hosted first run, starting the server, having your first conversation, and making your first tool call.

---

## 🎯 What You'll Learn

By the end of this guide, you will have:

- ✅ Started the OpenRustClaw gateway server
- ✅ Had an interactive chat with the assistant
- ✅ Used the built-in memory tools
- ✅ Verified persisted session continuity
- ✅ Understood the default assistant workflow

---

## 🏁 Prerequisites

Before starting, ensure you have:

1. **Installed OpenRustClaw** (see [Installation](./installation.md))
2. **Configured API keys** in your `.env` file
3. **Built the project** with `cargo build --workspace`

Quick check:

```bash
openrustclaw doctor
# Should show no blocking first-start issues for your chosen path
```

OpenRustClaw currently supports four self-hosted deployment paths:

- `solo`
- `team`
- `company`
- `enterprise`

If you want guided setup instead, run `openrustclaw onboard`. The wizard asks which deployment path you want, offers `Standard`, `Advanced`, or `Custom` setup depth, writes that choice into the workspace product-mode contract, and only offers to launch the persisted assistant session after the post-setup health gate confirms provider and workspace readiness.

You can inspect or change that path later from `/control/ui` in the `Self-Hosted Product Mode` panel or through `GET/POST /control/self-hosted/product-mode`. The same dashboard now also exposes `Setup Handoff`, which shows whether setup is ready, blocked, or degraded plus the next action and retained bootstrap outcomes.

---

## 🚀 Step 1: Start the Server

OpenRustClaw now starts as a Rust-owned runtime by default. The Python sidecar is optional and only used for bounded compatibility workflows.

### Terminal 1: Start the Runtime

```bash
# From the project root
cargo run --bin openrustclaw -- start

# Or if installed globally
openrustclaw start
```

You should see:

```
[INFO] OpenRustClaw gateway starting...
[INFO] Database: sqlite://data/openrustclaw.db (WAL mode)
[INFO] Gateway listening on 127.0.0.1:18789
[INFO] Ready for connections
```

### Verify the Runtime

```bash
# Check the gateway port
lsof -i :18789
```

If you explicitly need the compatibility sidecar for a migration-only workflow, start it separately and point the runtime at it through the sidecar configuration. That is no longer part of the default quickstart path.

---

## 💬 Step 2: Interactive Chat

Now let's have a conversation with your assistant.

### Start the CLI Chat

```bash
# In a new terminal
cargo run --bin openrustclaw -- assistant --provider anthropic

# Or
openrustclaw assistant --provider anthropic
```

The `--provider` flag specifies which LLM to use. Options:
- `anthropic` — Claude models
- `openai` — GPT models
- `openrouter` — 400+ models via OpenRouter
- `ollama` — Local models

The default assistant lane keeps tool access narrow on purpose. On first run it only exposes the built-in memory tools needed for continuity: `memory_search` and `memory_store`.
`memory_store` is intentionally strict: it should only be used when you explicitly ask the assistant to remember something or when the fact is clearly durable enough to justify future recall.

### Your First Conversation

```
╔══════════════════════════════════════════════════════════╗
║           OpenRustClaw Interactive Chat                   ║
║  Provider: anthropic | Model: claude-sonnet-4-20250514   ║
║  Type 'exit' or press Ctrl+C to quit                      ║
╚══════════════════════════════════════════════════════════╝

You: Hello! What's your name?

Assistant: Hello! I'm an AI assistant powered by OpenRustClaw. 
I don't have a specific name, but you can call me whatever you like. 
How can I help you today?

You: 
```

### Try Some Questions

```
You: What can you do?

Assistant: In the default CLI chat path, I can already help with:

1. **Memory recall**: I can search what you've told me before
2. **Memory storage**: I can save useful facts for later sessions
3. **Workspace context**: I can use the workspace instructions OpenRustClaw loads for this repo

This chat also resumes the same persisted CLI session automatically, so context carries across runs.
```

---

## 🛠️ Step 3: Your First Tool Call

Let's use the memory system — the agent's most important capability.

### Store a Memory

```
You: Please remember that my name is Alice and I work as a software engineer.

Assistant: I'll store that information in my memory.

[tool: memory_store] {"content": "User's name is Alice. User works as a software engineer.", "basis": "explicit_user_request", "reason": "The user explicitly asked me to remember it.", "memory_type": "semantic"}

✓ Stored successfully (id: a1b2c3d4...)

Got it! I've stored that information. Nice to meet you, Alice!
```

Notice what happened:
1. The agent decided to use the `memory_store` tool
2. It extracted the key information
3. The tool declared why the write is allowed under the assistant memory policy
4. The tool executed and returned a success response
5. The stored memory now carries write-policy metadata for later inspection
6. The agent confirmed the action

### Search Your Memory

```
You: What do you know about me?

Assistant: Let me search my memory for information about you.

[tool: memory_search] {"query": "user information personal details"}

Found 1 result:
- User's name is Alice. User works as a software engineer. (score: 0.95)

Based on my memory, I know that:
- Your name is Alice
- You work as a software engineer

Is there anything else you'd like me to remember?
```

### Verify with CLI

You can inspect the persisted assistant state directly:

```bash
# See the active CLI session
openrustclaw session list

# Inspect one session and its history
openrustclaw session show <session-id>

# Inspect memory health and recent entries
openrustclaw memory stats
openrustclaw memory timeline --limit 10
```

`openrustclaw memory timeline` now shows assistant-created write policy summaries inline, for example `basis=explicit_user_request; reason=The user explicitly asked me to remember it.`

If you start `openrustclaw assistant` again, it will resume the same active CLI session for this workspace and user instead of starting from an empty in-memory transcript. `openrustclaw session show` now makes that continuity explicit by surfacing the assistant surface, persistence model, route binding, and restored history count before the raw metadata dump. The older `openrustclaw chat` command now reuses the same persisted assistant path.

If `openrustclaw start` is running, you can inspect the same continuity contract in Control UI at `/control/ui`: the Sessions table calls out assistant-managed surfaces directly, and Session Detail summarizes whether the session looks resumed before showing the raw JSON report.

The same dashboard also exposes `Self-Hosted Product Mode`, so you can confirm whether this workspace is currently running as `solo`, `team`, `company`, or `enterprise`, and review recent upgrade or downgrade receipts before changing the deployment path again.

The same Control UI now exposes the tool and coding audit path used for production trust checks:

```bash
# Runtime tool ledger written by operator and MCP flows
ls .claw/control/tool-executions.jsonl

# Cursor coding artifacts written by coding tools
openrustclaw cursor status
ls .claw/control/cursor-tool-runs
```

`/control/ui` renders both `Recent Tool Executions` and `Recent Coding Artifacts`, so operators can confirm what ran, whether it succeeded, and where the retained evidence lives before opening raw files. Tool executions point back to `.claw/control/tool-executions.jsonl`; coding artifacts point at per-run JSON records under `.claw/control/cursor-tool-runs/` and include a diff preview for mutating file tools when the workspace is in git.

The same trust-first inspection loop now covers communications:

```bash
# Check whether Gmail / voice lanes look configured
openrustclaw runtime services channels
```

When the runtime is up, `/control/ui` shows `Channel Readiness`, `Recent Email Activity`, and `Recent Voice Outcomes`. Use those summaries first, then drill into the deeper surfaces only when needed: Gmail ingress receipts are retained as `channels.gmail_pubsub.ingress` entries in `.claw/control/tool-executions.jsonl`, and voice outcomes are derived from the persisted session receipts under `.claw/voice/sessions/`.

---

## 📊 Step 4: Understanding Context

OpenRustClaw uses a 3-tier memory system. Let's see it in action:

### Core Memory

Core memory (~500 tokens) is always included in the prompt:

```
You: Add to my core memory: preferred_language = Rust

Assistant: [tool: core_memory_set] {"key": "preferred_language", "value": "Rust"}

✓ Core memory updated. This will be included in all future conversations.
```

### Recall Memory

Recall memory is searched on-demand:

```
You: Tell me about my programming preferences

Assistant: [tool: memory_search] {"query": "programming preferences languages"}

Based on my search:
- You prefer working in Rust (from core memory)
- You're a software engineer (from recall memory)
```

### Archive (Long-term)

Archive contains consolidated summaries. In the shipped runtime, this is managed by the Rust-owned runtime path; the optional sidecar is only used for bounded compatibility workflows.

---

## 🔄 Step 5: Multi-Turn Conversation

Let's have a longer conversation to see context management:

```
You: I'm planning a trip to Japan next month

Assistant: That sounds exciting! Japan is a wonderful destination. 
What would you like to know or plan for your trip?

You: I want to visit Tokyo and Kyoto

Great choices! Tokyo and Kyoto offer very different experiences:
- Tokyo: Modern, bustling, cutting-edge technology and fashion
- Kyoto: Traditional, temples, gardens, geisha districts

How long will you be staying?

You: About 2 weeks

Perfect! Two weeks gives you plenty of time to explore both cities thoroughly.
You might also consider day trips to:
- From Tokyo: Nikko, Kamakura, or Hakone
- From Kyoto: Nara, Osaka, or Arashiyama

Would you like me to store any specific plans or preferences?

You: What was I talking about earlier?

Assistant: Let me search our conversation history...

[tool: memory_search] {"query": "trip Japan Tokyo Kyoto"}

Earlier in our conversation, you told me:
- Your name is Alice
- You work as a software engineer
- You prefer programming in Rust
```

The assistant should not silently persist temporary travel details just because they appeared in chat. If you do want those plans remembered across sessions, ask explicitly:

```
You: Please remember that I am planning a 2-week trip to Japan with stops in Tokyo and Kyoto.

Assistant: [tool: memory_store] {"content": "User is planning a 2-week trip to Japan with stops in Tokyo and Kyoto.", "basis": "explicit_user_request", "reason": "The user explicitly asked me to remember the travel plan.", "memory_type": "episodic"}
```

---

## 🎛️ Step 6: Try Different Providers

### Switch to OpenAI

```bash
# Exit the current chat (Ctrl+C or type 'exit')
# Then restart with a different provider
openrustclaw assistant --provider openai --model gpt-4o
```

### Use OpenRouter for Model Selection

```bash
# List available models
openrustclaw models list --provider openrouter

# Use a specific model
openrustclaw assistant --provider openrouter --model anthropic/claude-3.5-sonnet
```

### Try Local Models with Ollama

```bash
# Ensure Ollama is running
ollama run llama3.1

# Connect OpenRustClaw
openrustclaw assistant --provider ollama --model llama3.1
```

---

## 📈 Step 7: Monitor in LangSmith

If you configured `LANGSMITH_API_KEY` in your `.env`, you can view traces:

1. Visit [smith.langchain.com](https://smith.langchain.com)
2. Select your project (default: "openrustclaw")
3. View traces of your conversations

Each trace shows:
- Input/output messages
- Tool calls and results
- Token usage and latency
- Full conversation context

---

## 🛑 Step 8: Stop the Server

When you're done:

1. **Exit the chat**: Type `exit` or press `Ctrl+C`
2. **Stop the runtime**: Press `Ctrl+C` in the runtime terminal
3. **Stop the compatibility sidecar**: Press `Ctrl+C` there only if you explicitly started it

---

## 🎯 What You've Learned

Congratulations! You've:

- ✅ Started the OpenRustClaw runtime
- ✅ Had an interactive chat with context awareness
- ✅ Made tool calls (memory_store, memory_search)
- ✅ Used the 3-tier memory system
- ✅ Switched between different LLM providers
- ✅ Observed traces in LangSmith

---

## 🚀 Next Steps

Ready to go deeper?

1. **[Create Your First Agent](./first-agent.md)** — Build a custom agent with specific tools
2. **[Architecture Overview](../architecture/overview.md)** — Understand how everything works
3. **[Memory System Guide](../guides/memory.md)** — Master the 3-tier memory system

---

## 💡 Tips

### Start the Assistant Quickly

```bash
# Launch the persisted assistant session
openrustclaw assistant --provider anthropic
```

### Enable Debug Logging

```bash
# See detailed logs
RUST_LOG=debug cargo run --bin openrustclaw -- assistant
```

### Run Diagnostics Anytime

```bash
openrustclaw doctor --verbose
```

---

## 🆘 Getting Help

If you run into issues:

1. Run `openrustclaw doctor` to diagnose
2. Check logs in the runtime terminal and, if you explicitly enabled it, the compatibility sidecar terminal
3. See [Troubleshooting](./installation.md#troubleshooting)
4. File an issue on GitHub
