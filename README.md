# Fleet Bottle

**Fleet Bottle** is a Rust library implementing the Bottle Protocol — an immutable, priority-ordered, inter-agent messaging system for the SuperInstance fleet, supporting typed payloads (alerts, commands, consensus votes, discovery reports) with wire-format serialization and pluggable transports.

## Why It Matters

Agent fleets need reliable, typed, traceable inter-agent communication. Raw TCP or HTTP lacks the semantics that agent systems require: priority levels for urgent messages, typed payloads so receivers know how to handle content, immutable envelopes for audit trails, and consensus primitives for distributed decision-making. The Bottle Protocol borrows its metaphor from messages in bottles — once sealed and sent, a bottle is immutable and self-contained. It carries its own metadata (sender, recipient, priority, timestamp, unique ID) and a typed payload. This makes fleet communication debuggable: every message can be traced through its complete lifecycle, and the immutable envelope ensures no intermediary can tamper with content. The protocol is transport-agnostic — the same bottle can travel via memory channels, Unix sockets, HTTP, or carrier pigeon (in theory).

## How It Works

**Bottle structure:**
```
Bottle {
    id: UUIDv4,              // globally unique
    from: AgentId,           // sender
    to: AgentId,             // recipient (or "broadcast")
    priority: Priority,      // Low, Normal, High, Critical
    timestamp: DateTime<Utc>,
    payload: BottlePayload,  // typed content
    state: BottleState,      // Sent, Delivered, Read, Acknowledged
}
```

**Priority-based dispatch:**
| Priority | Latency Target | Use Case |
|----------|---------------|----------|
| Critical | < 100ms | Safety alerts, conservation violations |
| High | < 1s | Fleet commands, task assignments |
| Normal | < 5s | Telemetry, status updates |
| Low | Best-effort | Log sync, historical data |

**Typed payloads:**
```
enum BottlePayload {
    Alert(AlertMessage),      // severity + description
    Command(BottleCommand),   // actionable instruction
    ConsensusVote(ConsensusVote),  // proposal + vote value
    DiscoveryReport(DiscoveryReport),  // node discovery data
    StateChange(BottleState), // state transition notification
}
```

The type system ensures compile-time safety: a handler expecting `AlertMessage` cannot accidentally process a `Command`.

**Wire format:** Bottles serialize to compact JSON via `serde_json`. The wire format includes a version tag for forward compatibility:

```json
{"version": 1, "id": "uuid", "from": "agent-1", "to": "agent-2",
 "priority": "high", "timestamp": "2026-01-15T12:00:00Z",
 "payload": {"type": "alert", "severity": "warning", "message": "..."}}
```

**Transport abstraction:** The `Transport` trait decouples bottles from their delivery mechanism:

```rust
trait Transport {
    async fn send(&self, bottle: &Bottle) -> Result<(), TransportError>;
    async fn recv(&self) -> Result<Bottle, TransportError>;
}
```

`MemoryTransport` provides an in-memory implementation for testing and same-process communication. Production transports (Unix socket, HTTP, WebSocket) implement the same trait.

**Consensus votes:** Bottles carry `ConsensusVote` payloads with `VoteValue` (Accept/Reject/Abstain), enabling distributed consensus protocols via bottle exchange.

## Quick Start

```rust
use fleet_bottle::{BottleBuilder, Priority, Payload::Alert, AlertSeverity};

fn main() {
    let bottle = BottleBuilder::new()
        .from("agent-1")
        .to("fleet-orchestrator")
        .priority(Priority::High)
        .alert(AlertSeverity::Warning, "Avoidance ratio anomaly detected")
        .build();
    println!("Bottle {} queued: {:?}", bottle.id, bottle.priority);
}
```

## API

| Type | Description |
|------|-------------|
| `Bottle` | Immutable message envelope with metadata |
| `BottleBuilder` | Fluent constructor for bottles |
| `Priority` | Low, Normal, High, Critical |
| `BottlePayload` | Tagged union: Alert, Command, Vote, Discovery, State |
| `BottleState` | Sent, Delivered, Read, Acknowledged |
| `Transport` | Async send/recv trait |
| `MemoryTransport` | In-memory transport for testing |
| `encode` / `decode` | JSON wire format serialization |

## Architecture Notes

Fleet Bottle provides the **inter-agent communication protocol** for γ + η = C. Every conservation-law observation flows through bottles: γ-layer agents send their action distributions as bottles to η-layer analysis agents, which respond with conservation verdicts. The priority system ensures that conservation violations (Critical priority) preempt all other fleet communication.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

1. Apache Foundation (2024). *Avro 1.11 Specification: Schema Evolution*. (Wire-format versioning patterns.)
2. Lamport, L. (1978). "Time, Clocks, and the Ordering of Events in a Distributed System." *Communications of the ACM*, 21(7), 558–565. (Logical clocks for message ordering.)
3. Clement, L. et al. (2009). "Making Byzantine Fault Tolerant Systems Tolerate Byzantine Faults." *NSDI*. (Consensus protocol design.)

## License

MIT
