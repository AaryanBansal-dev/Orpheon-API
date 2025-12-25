# Orpheon Protocol
> **The Intent-Native Interaction Standard for Autonomous Systems**

[![Build Status](https://img.shields.io/github/actions/workflow/status/orpheon-protocol/core/ci.yml?branch=main)](https://github.com/orpheon-protocol/core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Discord](https://img.shields.io/discord/1234567890?label=discord&color=5865F2)](https://discord.gg/orpheon)

**Orpheon** replaces the rigid Request/Response paradigm with a negotiable, intent-based interaction model designed for AI agents, autonomous orchestrators, and high-reliability systems.

[**📚 Read the Full Context & Specification**](./CONTEXT.md)

---

## 🚀 Why Orpheon?

| Feature | Traditional APIs (REST/GraphQL) | Orpheon Protocol |
| :--- | :--- | :--- |
| **Interaction** | **Imperative** ("Do step X") | **Declarative** ("Achieve outcome Y") |
| **Responsibility** | Client orchestrates everything | Server plans and executes |
| **State** | Polling / Webhooks | Real-time State Subscriptions |
| **Failures** | Manual retry logic | Automatic Compensation & Rollback |
| **Time** | Instantaneous only | **Temporal** (Schedule, Simulate, Replay) |

## 📦 Installation

Add Orpheon to your `Cargo.toml`:

```toml
[dependencies]
orpheon = "0.1.0"
```

## ⚡ Quick Start

Define an **Intent** and submit it to the network:

```rust
use orpheon::prelude::*;

#[tokio::main]
async fn main() -> Result<(), OrpheonError> {
    let client = OrpheonClient::connect("https://api.mainnet.orpheon.network").await?;

    // 1. Define what you want
    let intent = Intent::builder()
        .kind("provision_gpu_cluster")
        .constraint("count", 8)
        .constraint("type", "H100")
        .budget(Budget::usd(100.0))
        .build();

    // 2. Submit intent and subscribe to updates
    let mut plan_stream = client.submit(intent).await?;

    while let Some(event) = plan_stream.next().await {
        match event {
            EventType::Negotiating(opt) => println!(" negotiating: {:?}", opt),
            EventType::Executing(step) => println!(" executing: {:?}", step),
            EventType::Complete(artifact) => {
                println!("✅ Done! Proof: {}", artifact.merkle_root);
                break;
            }
        }
    }
    
    Ok(())
}
```

## Development

### Web

```bash
cargo run -p orpheon-node
```

```bash
bun dev
```

## 🛠️ Features (The 100+ Matrix)

Orpheon supports over 100 advanced capabilities across 7 spheres:

*   **Cognitive**: Recursive Intents, Probabilistic Branching, LLM Integration.
*   **Network**: P2P Gossip, Federation, DTN, Edge Offloading.
*   **Trust**: Zero-Knowledge Proofs, Quantum-Resistant Crypto.
*   **Temporal**: Time-Travel Querying, Speculative Simulation.
*   **Economic**: Dynamic Markets, Resource Bonding Curves.
*   **Developer**: Visual Debugger, Chaos Injection.
*   **Hardware**: FPGA Acceleration, BCI Triggers.

See [CONTEXT.md](./CONTEXT.md) for the full matrix.

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines on how to get started.

## 🔒 Security

Security is our top priority. See [SECURITY.md](./SECURITY.md) for our reporting policy and PGP keys.

## 📄 License

Dual-licensed under MIT and Apache 2.0. See [LICENSE](./LICENSE) for details.
