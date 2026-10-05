# Smart Migrate architecture

## System boundary

```text
Windows Host ──┐
               ├── authenticated signaling API ── device/session records
Android Client ┘                 │
                                └── STUN/TURN credentials

Windows Host ══ encrypted direct or relayed session ══ Android Client
       │
       └── MigRoute: state, capabilities, authorization, protocol validation
```

## Platform ownership

| Boundary | Responsibility | Chosen direction |
|---|---|---|
| `crates/migroute` | Device identity validation, pairing state, capability/permission narrowing, SMP invariants | Rust |
| Windows host | Capture adapter, hardware encoder adapter, host permission UI, input adapter | Rust + Tauri 2 shell |
| Android client | Pairing UX, protected credential storage, video surface, explicit input controls | Kotlin + Jetpack Compose |
| API/signaling | Authentication, devices, short-lived session credentials, signaling authorization, audit metadata | Java 21 + Spring Boot direction |
| Media/data plane | WebRTC media and data channels, ICE/STUN/TURN, DTLS/SRTP | Standards-based transport |
| Web | Product information, releases, documentation, platform-aware downloads | TypeScript web application |

## MigRoute rules

MigRoute is not a transport or UI. It is the systems-engine policy boundary:

1. Parse and validate bounded, versioned SMP messages before platform adapters use them.
2. Treat host-side permission decisions as authoritative.
3. Allow a host to grant only a subset of requested capabilities.
4. Track expiry, replay/sequence checks, disconnect, and revocation as explicit states.
5. Pass typed decisions to capture, transport, input, clipboard, and transfer adapters; those adapters must not invent authorization.

## Non-goals of the foundation

No screen capture, input injection, encryption implementation, device discovery, file transfer, or network session is claimed to work in this repository yet. Each requires a dedicated native adapter, tests, and security review before it is exposed in a product UI.
