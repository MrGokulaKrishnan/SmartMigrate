# Smart Migrate Protocol — SMP/1

SMP/1 is the versioned product protocol used by MigRoute. Transport selection is separate: low-latency media and live control should use secured WebRTC channels, while signaling uses authenticated WSS.

## Every envelope carries

- Protocol version
- Message type
- Session ID and optional request ID
- Monotonic sequence number where ordering matters
- Issued and expiry timestamps
- Bounded serialized payload

## Initial message families

| Family | Examples |
|---|---|
| Device | `DEVICE_HELLO`, `DEVICE_CAPABILITIES` |
| Pairing | `PAIR_REQUEST`, `PAIR_ACCEPT`, `PAIR_REJECT` |
| Session | `SESSION_CREATE`, `SESSION_ACCEPT`, `SESSION_REJECT`, `SESSION_END` |
| Transfer | `TRANSFER_START`, `TRANSFER_CHUNK`, `TRANSFER_ACK`, `TRANSFER_COMPLETE`, `TRANSFER_CANCEL` |
| Control | `INPUT_MOUSE`, `INPUT_KEYBOARD`, `CLIPBOARD_UPDATE` |
| Stream | `STREAM_START`, `STREAM_STOP`, `STREAM_CONFIG` |
| Health | `PING`, `PONG`, `ERROR` |

## Validation rules

1. Reject unknown protocol versions and message types.
2. Reject oversized, malformed, expired, replayed, and wrong-session payloads.
3. Verify authorization at the host on every privileged state transition.
4. High-rate pointer data evolves to a compact binary payload only after stable compatibility tests; it is never trusted merely because the transport is encrypted.
5. Compatibility changes require a new schema revision and documented negotiation behavior.

The initial protobuf contract is [packages/protocol/schema/smart_migrate.proto](../packages/protocol/schema/smart_migrate.proto).
