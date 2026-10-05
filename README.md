# Smart Migrate

**Smart Migrate** is a security-first, cross-device connectivity platform for approved device pairing, remote assistance, screen streaming, and resumable transfer workflows.

Its systems engine is **MigRoute**. MigRoute owns capability negotiation, pairing state, permission boundaries, and the transport-facing contract. It never gives a client more authority than the host grants.

## Current foundation

This repository begins with Phase 0 and Milestone 1 foundations:

- A single Smart Migrate brand system and supplied logo asset.
- Architecture, threat-model, protocol, environment, and milestone documentation.
- A responsive product landing-page shell.
- A dependency-free Rust `migroute` crate with typed pairing and permission primitives plus tests.
- Platform and service boundaries ready for the Windows host, Android client, and signaling API.

The repository deliberately does **not** present unfinished screen capture, remote control, or transfer capability as working. Those remain gated by the milestone plan and explicit security review.

## Layout

```text
apps/             Product-facing applications
  website/        Responsive marketing and download shell
  windows-host/   Future Tauri/Rust Windows host boundary
  android-client/ Future Kotlin/Compose Android client boundary
assets/media/     Canonical supplied Smart Migrate logo
crates/migroute/  Rust systems-engine foundation
packages/protocol/ Versioned protobuf contract
services/api/     Future authenticated signaling and device API boundary
shared/design-tokens/ Cross-surface visual tokens
docs/             Product, architecture, security, and delivery decisions
```

## Validate the engine

```powershell
cargo test -p migroute
```

## Brand naming

- Product: **Smart Migrate**
- Engine: **MigRoute**
- Protocol contract: **Smart Migrate Protocol (SMP/1)**

See [docs/SPEC.md](docs/SPEC.md) for the current build specification and [docs/MILESTONES.md](docs/MILESTONES.md) for the implementation order.
