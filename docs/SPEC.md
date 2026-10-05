# Smart Migrate — master specification

## Product identity

- **Product:** Smart Migrate
- **Engine:** MigRoute
- **Protocol:** Smart Migrate Protocol, SMP/1
- **Mission:** Secure, explicit-consent connectivity between a Windows host and approved Windows or Android clients for remote assistance, screen streaming, and later resumable transfer.

## Product rules

Build a production-oriented, modular product. Do not ship a fake remote session, fake encryption indicator, fake pairing approval, or a privileged browser implementation.

- The Windows host has final authority over device trust and each capability.
- Pairing requires a short-lived QR or numeric code plus explicit approval.
- Permissions are granular: view screen, mouse, keyboard, send files, receive files, clipboard, audio, and controller. Nothing is implicitly granted.
- The host shows a visible live-session indicator and can end a session immediately.
- Prefer direct encrypted connectivity, with standards-based STUN/TURN fallback and authenticated WSS signaling.
- Use established cryptography and secure platform storage; never design custom encryption.
- Capturing, encoding, injecting input, and secret storage remain behind native, reviewed adapters.

## Architecture and stack

| Area | Direction |
|---|---|
| MigRoute engine | Rust; typed protocol validation, pairing/session state, capability negotiation, authorization rules |
| Windows Host | Rust + Tauri 2 shell, Windows Graphics Capture, hardware encode adapters, controlled input adapters |
| Android Client | Kotlin, Jetpack Compose, Android Keystore, native video surface |
| Backend | Java 21 + Spring Boot, PostgreSQL, Redis, WSS signaling, OpenAPI, Flyway |
| Real-time transport | WebRTC with ICE, STUN/TURN, DTLS/SRTP and secured data channels |
| Web | TypeScript, responsive product/download experience |

## Required quality bar

- Strict types, bounded input, cancellation, backpressure, structured and privacy-safe logging.
- Unit, integration, protocol, security, network-impairment, native-platform, installer, and upgrade tests as each component becomes real.
- No secret in Git, configuration templates, frontend bundles, or logs.
- Features that have not passed their acceptance criteria appear unavailable, not simulated.

## Delivery order

The authoritative staged plan is in [MILESTONES.md](MILESTONES.md). Build and review each stage before enabling the next privileged capability.
