# Smart Migrate delivery plan

## Phase 0 — complete foundation

- Product, design, security, protocol, and environment decisions documented.
- Supplied logo placed in canonical media paths.
- MigRoute authorization primitives tested.
- Website shell communicates only verified current scope.

## Milestone 1 — application shells

- Tauri Windows shell with a custom Smart Migrate title bar and non-privileged dashboard.
- Kotlin/Compose Android shell with pairing entry and a safe disconnected state.
- Spring Boot API skeleton with health endpoint, device model, and secure configuration boundary.
- CI formatting, linting, unit tests, dependency scanning, and secret scanning.

## Milestone 2 — trusted pairing and signaling

- Persistent per-device identity in OS-keystore-backed storage.
- Short-lived QR/numeric pairing with explicit host approval.
- Authenticated WSS signaling, device revocation, and session audit metadata.

## Milestone 3 — view-only remote session

- Native Windows Graphics Capture adapter, hardware-encoder capability detection, WebRTC video, Android hardware decode.
- LAN path first; resilient disconnect/error states.

## Milestone 4 — controlled input and resilience

- Explicit host-authorized mouse and keyboard input.
- Sequence/replay enforcement, reconnection, metrics, and TURN fallback.

## Later milestones

- Audio, resumable transfer, opt-in clipboard, multi-monitor, privacy mode, advanced diagnostics, and signed release pipelines.

No milestone is complete until it has working code, tests, operational error states, documentation, and security review notes.
