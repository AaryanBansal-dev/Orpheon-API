# Contributing to Orpheon

Thank you for your interest in contributing to the Orpheon Protocol! We are building the operating system for the next generation of autonomous software, and we need your help.

## 🤝 Code of Conduct

This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). By participating, you are expected to uphold this code.

## 🛠️ Getting Started

1.  **Fork the repository** on GitHub.
2.  **Clone your fork** locally:
    ```bash
    git clone https://github.com/YOUR_USERNAME/orpheon.git
    cd orpheon
    ```
3.  **Install Rust**: Ensure you have the latest stable Rust toolchain.
    ```bash
    rustup update stable
    ```

## 🧪 Development Workflow

We use a standard feature-branch workflow.

1.  Create a new branch for your feature or fix:
    ```bash
    git checkout -b feature/amazing-new-planner
    ```
2.  **Write Code**: Implement your changes.
3.  **Run Tests**:
    ```bash
    cargo test
    ```
4.  **Format Code**:
    ```bash
    cargo fmt
    ```
5.  **Lint Code**:
    ```bash
    cargo clippy -- -D warnings
    ```

## 📝 Commit Guidelines

We use [Conventional Commits](https://www.conventionalcommits.org/):

*   `feat: add Zero-Knowledge Proof verifier`
*   `fix: resolve race condition in state store`
*   `docs: update CONTEXT.md with new features`
*   `perf: optimize A* heuristic calculation`

## ⚖️ Legal

By contributing to **Orpheon**, you agree that your contributions will be licensed under its dual MIT/Apache-2.0 license.
