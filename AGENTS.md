# Smart Migrate repository rules

- Use **Smart Migrate** for every user-facing product reference.
- Use **MigRoute** for the Rust systems engine and engine-facing telemetry.
- The protocol remains **Smart Migrate Protocol (SMP/1)** unless an approved protocol-version change says otherwise.
- Never represent unimplemented remote-control, screen-capture, encryption, transfer, or pairing work as functional.
- Host-side authorization is the source of truth for every requested capability.
- No secret, device-private key, pairing token, screen frame, password, or raw user file path may reach logs.
- Keep platform-native boundaries explicit: Rust/Windows, Kotlin/Android, TypeScript/Web, and the server-side API must not rely on a privileged browser context.
- Update the relevant architecture, threat-model, and protocol docs with any public-boundary change.
