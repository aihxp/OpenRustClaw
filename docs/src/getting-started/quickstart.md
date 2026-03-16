# Quickstart Guide

Get up and running with OpenRustClaw in just 5 minutes. This guide walks you through starting the server, having your first conversation, and making your first tool call.

---

## 🎯 What You'll Learn

By the end of this guide, you will have:

- ✅ Started the OpenRustClaw gateway server
- ✅ Had an interactive chat with the agent
- ✅ Made your first tool call (memory search)
- ✅ Viewed stored memories
- ✅ Understood the basic workflow

---

## 🏁 Prerequisites

Before starting, ensure you have:

1. **Installed OpenRustClaw** (see [Installation](./installation.md))
2. **Configured API keys** in your `.env` file
3. **Built the project** with `cargo build --workspace`

Quick check:

```bash
openrustclaw doctor
# Should show all checks passing
```

---

## 🚀 Step 1: Start the Server

OpenRustClaw consists of two main components:

1. **Rust Gateway** — WebSocket server and core runtime
2. **Python Sidecar** — LangGraph workflows and orchestration

### Terminal 1: Start the Python Sidecar

```bash
cd sidecar
source .venv/bin/activate
python src/server.py
```

You should see:

```
[INFO] Starting OpenRustClaw sidecar on port 50051
[INFO] Connected to LangSmith (project: openrustclaw)
[INFO] gRPC server ready
```

### Terminal 2: Start the Gateway

```bash
# In a new terminal, from the project root
cargo run --bin openrustclaw -- start

# Or if installed globally
openrustclaw start
```

You should see:

```
[INFO] OpenRustClaw gateway starting...
[INFO] Database: sqlite://data/openrustclaw.db (WAL mode)
[INFO] Connected to sidecar at localhost:50051
[INFO] Gateway listening on 127.0.0.1:18789
[INFO] Ready for connections
```

### Verify Both Are Running

```bash
# Check processes
lsof -i :18789  # Gateway
lsof -i :50051  # Sidecar
```

---

## 💬 Step 2: Interactive Chat

Now let's have a conversation with your agent!

### Start the CLI Chat

```bash
# In a new terminal
cargo run --bin openrustclaw -- chat --provider anthropic

# Or
openrustclaw chat --provider anthropic
```

The `--provider` flag specifies which LLM to use. Options:
- `anthropic` — Claude models
- `openai` — GPT models
- `openrouter` — 400+ models via OpenRouter
- `ollama` — Local models

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

Assistant: I can help you with a variety of tasks using my available tools:

1. **Memory**: I can store and retrieve information you share with me
2. **Search**: I can search through my memory to find relevant information
3. **Scheduling**: I can set reminders and schedule tasks
4. **Tools**: I can use various skills and integrations

What would you like to try?
```

---

## 🛠️ Step 3: Your First Tool Call

Let's use the memory system — the agent's most important capability.

### Store a Memory

```
You: Please remember that my name is Alice and I work as a software engineer.

Assistant: I'll store that information in my memory.

[tool: memory_store] {"content": "User's name is Alice. User works as a software engineer.", "memory_type": "semantic"}

✓ Stored successfully (id: a1b2c3d4...)

Got it! I've stored that information. Nice to meet you, Alice!
```

Notice what happened:
1. The agent decided to use the `memory_store` tool
2. It extracted the key information
3. The tool executed and returned a success response
4. The agent confirmed the action

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

You can also view memories directly:

```bash
# List all memories
openrustclaw memory list

# Search memories
openrustclaw memory search "software engineer"

# Show statistics
openrustclaw memory stats
```

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

Archive contains consolidated summaries. This is managed automatically by the LangGraph sidecar.

---

## 🔄 Step 5: Multi-Turn Conversation

Let's have a longer conversation to see context management:

```
You: I'm planning a trip to Japan next month

Assistant: That sounds exciting! Japan is a wonderful destination. 
What would you like to know or plan for your trip?

You: I want to visit Tokyo and Kyoto

Assistant: [tool: memory_store] {"content": "User is planning a trip to Japan. Destinations: Tokyo and Kyoto.", "memory_type": "episodic"}

Great choices! Tokyo and Kyoto offer very different experiences:
- Tokyo: Modern, bustling, cutting-edge technology and fashion
- Kyoto: Traditional, temples, gardens, geisha districts

How long will you be staying?

You: About 2 weeks

Assistant: [tool: memory_store] {"content": "User's Japan trip duration: 2 weeks", "memory_type": "episodic"}

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
- You're planning a 2-week trip to Japan
- You want to visit Tokyo and Kyoto
- You prefer programming in Rust
```

---

## 🎛️ Step 6: Try Different Providers

### Switch to OpenAI

```bash
# Exit the current chat (Ctrl+C or type 'exit')
# Then restart with a different provider
openrustclaw chat --provider openai --model gpt-4o
```

### Use OpenRouter for Model Selection

```bash
# List available models
openrustclaw models list --provider openrouter

# Use a specific model
openrustclaw chat --provider openrouter --model anthropic/claude-3.5-sonnet
```

### Try Local Models with Ollama

```bash
# Ensure Ollama is running
ollama run llama3.1

# Connect OpenRustClaw
openrustclaw chat --provider ollama --model llama3.1
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
2. **Stop the gateway**: Press `Ctrl+C` in the gateway terminal
3. **Stop the sidecar**: Press `Ctrl+C` in the sidecar terminal

---

## 🎯 What You've Learned

Congratulations! You've:

- ✅ Started the OpenRustClaw gateway and sidecar
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

### Use `--once` for Single Queries

```bash
# Quick one-off questions without interactive mode
openrustclaw chat --provider anthropic --once "What is the capital of France?"
```

### Enable Debug Logging

```bash
# See detailed logs
RUST_LOG=debug cargo run --bin openrustclaw -- chat
```

### Run Diagnostics Anytime

```bash
openrustclaw doctor --verbose
```

---

## 🆘 Getting Help

If you run into issues:

1. Run `openrustclaw doctor` to diagnose
2. Check logs in the gateway/sidecar terminals
3. See [Troubleshooting](./installation.md#troubleshooting)
4. File an issue on GitHub
