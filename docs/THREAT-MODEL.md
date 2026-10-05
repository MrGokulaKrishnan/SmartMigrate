# Smart Migrate threat model

## Assets to protect

- Host display content, microphone content, clipboard content, and transferred files.
- Device private keys, refresh tokens, one-time pairing codes, and TURN credentials.
- Host control authority and the trusted-device list.
- Audit evidence that excludes sensitive session content.

## Trust boundaries

1. Internet-facing client to API/signaling service.
2. Unpaired device to host pairing flow.
3. Paired client to host capability decision.
4. Transport payload to native capture, input, clipboard, and file adapters.
5. Local encrypted secret storage to application process.

## Controls required before an MVP capability is enabled

| Threat | Required control |
|---|---|
| Unauthorized remote control | Explicit host approval, granular permissions, visible host session indicator, immediate disconnect |
| Pairing-code guessing | Cryptographically random single-use codes, short expiry, rate limits, approval on both endpoints |
| Signaling impersonation | Authenticated WSS, device-bound identity, short-lived session authorization, audit events |
| Replay or reordered control requests | SMP versioning, session binding, monotonic sequence checks, timestamps and expiry |
| Token or key disclosure | OS-protected storage, redacted logs, no secrets in bundles, configuration, or source control |
| Malicious file delivery | Explicit acceptance, filename normalization, fixed destination policy, size limits, chunk integrity, no auto-execution |
| Path traversal | Reject paths and derive destination from trusted local policy rather than remote input |
| Frame or clipboard disclosure | Encrypted transport, opt-in clipboard, server does not persist content, least-privilege telemetry |
| Host abuse after trust changes | Revoke device terminates active sessions and invalidates credentials |

## Security acceptance gates

- Threat-model review before every new privileged adapter.
- Negative tests for authorization, expiry, sequence validation, and malformed payloads.
- Dependency and secret scanning in CI.
- No unencrypted RTP/RTCP, debug credential storage, hidden control sessions, or custom cryptography.
