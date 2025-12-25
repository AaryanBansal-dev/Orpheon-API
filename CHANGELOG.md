# Changelog

All notable changes to the **Orpheon Protocol** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Capability Matrix**: A catalog of 100+ new features including Cognitive, Network, and Trust spheres.
- **Protocol Specification**: Comprehensive `CONTEXT.md` defining the Rust primitives (`Intent`, `Plan`, `Artifact`).
- **Rust SDK**: Initial scaffold for `orpheon-sdk` crate.
- **Documentation**: Added `README.md`, `CONTRIBUTING.md`, `SECURITY.md`.

### Changed
- Refactored entire interaction model from REST-like to Intent-Native.
- Migrated core planning logic to Async Rust (`tokio`).

## [0.1.0-alpha] - 2024-12-25
### Initial Release
- Basic "Concept" release.
- A* Planner prototype.
- In-memory state store.
