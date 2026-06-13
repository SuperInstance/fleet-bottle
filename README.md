# 🍾 fleet-bottle

Inter-agent messaging protocol for the **SuperInstance** fleet. Bottles are immutable messages that drift between agents (ships) until picked up.

## Concept

A **Bottle** is like a message in a bottle cast between ships in a fleet:

- **Immutable** — once built, contents don't change
- **Directed or broadcast** — send to a specific agent or the whole fleet
- **TTL-based expiry** — old bottles dissolve and are discarded
- **Priority levels** — Low, Normal, High, Emergency (ordered)
- **Transport-agnostic** — HTTP, WebSocket, filesystem, stdio, anything

## Usage

```rust
use fleet_bottle::*;
use chrono::Duration;

// Broadcast to the fleet
let bottle = Bottle::builder("agent-alpha")
    .payload(BottlePayload::Text("All hands on deck".into()))
    .priority(Priority::High)
    .ttl(Duration::seconds(300))
    .build();

// Direct message with a command
let dm = Bottle::builder("commander")
    .to("worker-1")
    .payload(BottlePayload::Command(
        BottleCommand::new("deploy")
            .arg("fleet-edge-worker")
            .arg("--production")
            .expect_ack(),
    ))
    .meta("trace_id", serde_json::json!("abc-123"))
    .build();

// Wire encoding
let bytes = encode(&bottle, WireFormat::Json)?;
let decoded = decode(&bytes)?; // auto-detects JSON vs binary

// In-memory transport (for testing)
let transport = MemoryTransport::new(WireFormat::BinaryV1);
transport.send(&bottle)?;
let received = transport.receive()?;
```

## Payload Types

| Type | Purpose |
|------|---------|
| `Text` | Plain text messages |
| `Command` | Instructions for the receiving agent |
| `State` | Key-value state snapshots |
| `Consensus` | Votes in distributed decisions |
| `Discovery` | Agent/service discovery reports |
| `Alert` | Warnings and critical alerts |

## Wire Formats

- **JSON** — human-readable, debuggable, CF Workers friendly
- **BinaryV1** — framed binary with magic bytes (`0xF1B0711E`), version header, length-prefixed JSON payload
- **Auto-detection** — `decode()` detects format from magic bytes

## Compatibility

- No filesystem dependencies in core
- Works in Cloudflare Workers (WASM-compatible)
- All serialization via `serde_json` — no proc-macro binary deps

## License

MIT
