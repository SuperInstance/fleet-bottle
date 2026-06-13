# 🍾 fleet-bottle

Inter-agent messaging protocol for the SuperInstance fleet.

Bottles are immutable messages passed between agents — like messages in bottles between ships at sea.

## Quick Start

```rust
use fleet_bottle::*;

// Create a directed message
let bottle = Bottle::builder("agent-alpha")
    .to("agent-bravo")
    .payload(BottlePayload::Text("status: nominal".into()))
    .priority(Priority::High)
    .ttl(chrono::Duration::seconds(30))
    .build();

// Broadcast a discovery
let broadcast = Bottle::builder("scout")
    .payload(BottlePayload::Discovery(DiscoveryReport {
        subject: "new-crate".into(),
        details: vec!["found in registry".into()],
        confidence: 0.95,
    }))
    .build();
```

## Wire Format

Two formats are supported:

- **JSON** — human-readable, debuggable, CF Workers friendly
- **Binary V1** — framed with magic bytes (`F1 B0 71 1E`), auto-detected on decode

```rust
use fleet_bottle::protocol::{encode, decode, WireFormat};

let bytes = encode(&bottle, WireFormat::Json)?;
let restored = decode(&bytes)?;
```

## Transport

The `Transport` trait is transport-agnostic. Implement it for HTTP, WebSocket, file system, or stdio.

A `MemoryTransport` is included for testing:

```rust
use fleet_bottle::transport::MemoryTransport;

let transport = MemoryTransport::new(WireFormat::Json);
transport.send(&bottle)?;
let received = transport.receive()?;
```

## Design Principles

- **Immutable** — bottles cannot be modified after creation
- **TTL-aware** — optional time-to-live with expiry checking
- **Priority levels** — Low, Normal, High, Emergency
- **Transport-agnostic** — HTTP, WebSocket, file system, stdio, or custom
- **CF Workers compatible** — no filesystem deps in core
- **Serializable** — JSON and compact binary formats

## Payload Types

| Type | Purpose |
|------|---------|
| `Text` | Plain messages between agents |
| `Command` | Remote procedure calls with ack support |
| `State` | Key-value state snapshots |
| `Consensus` | Distributed voting |
| `Discovery` | Capability/finding announcements |
| `Alert` | Priority notifications requiring attention |
