# Security Policy

## Reporting Security Issues

Smart Migrate and the MigRoute systems engine take security seriously. If you discover a vulnerability or security flaw, please report it responsibly so we can investigate and address it before public disclosure.

### Contact
Please send vulnerability reports to:
- **Email**: security@smartmigrate.org
- **GitHub Security Advisories**: Report privately via [GitHub Security Advisories](https://github.com/MrGokulaKrishnan/SmartMigrate/security/advisories/new)

### Guidelines
When reporting a security issue:
1. Provide a detailed summary with reproduction steps or Proof of Concept (PoC).
2. Specify the affected component (Windows Host, Android Client, MigRoute Engine, or Website).
3. Do **NOT** publish details or PoC publicly until coordinated disclosure is complete.
4. Adhere to repository security rules: no private keys, pairing tokens, passwords, screen frames, or raw user file paths should ever be shared in unencrypted public channels.

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Security Architecture & Guarantees

Smart Migrate follows strict zero-trust host-authorization principles:
- **Host Authorization as Source of Truth**: Remote capabilities (screen capture, clipboard, file transfer, input injection) must be explicitly granted by the host.
- **Smart Migrate Protocol (SMP/1)**: All network payloads follow authenticated envelope framing.
- **No Secret Leakage**: Cryptographic keys, pairing tokens, screen frames, and user file paths are strictly prevented from reaching persistent logs.
- **Transport Security**: TLS 1.3 is enforced for all cloud transit; local streaming listeners require high-entropy cryptographic session tokens.
