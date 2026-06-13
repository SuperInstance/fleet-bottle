//! Integration tests for the fleet-bottle library.

use chrono::{Duration, Utc};
use fleet_bottle::*;
use fleet_bottle::payload::{VoteValue, AlertSeverity};
use serde_json::Value;

#[test]
fn full_lifecycle_directed_message() {
    // Agent Alpha sends a command to Agent Bravo.
    let bottle = Bottle::builder("agent-alpha")
        .to("agent-bravo")
        .payload(BottlePayload::Command(
            BottleCommand::new("health_check").expect_ack(),
        ))
        .priority(Priority::High)
        .ttl(Duration::seconds(30))
        .meta("trace_id", Value::String("trace-001".into()))
        .build();

    // Serialize to JSON.
    let json = protocol::encode(&bottle, WireFormat::Json).unwrap();

    // Simulate transport — decode on the other side.
    let received = protocol::decode(&json).unwrap();

    assert_eq!(received.id, bottle.id);
    assert_eq!(received.from, "agent-alpha");
    assert_eq!(received.to.as_ref().unwrap(), "agent-bravo");
    assert!(received.is_directed());
    assert!(!received.is_expired(Utc::now()));
    assert_eq!(received.metadata.get("trace_id").unwrap(), "trace-001");

    if let BottlePayload::Command(cmd) = &received.payload {
        assert_eq!(cmd.name, "health_check");
        assert!(cmd.expects_ack);
    } else {
        panic!("expected Command payload");
    }
}

#[test]
fn broadcast_discovery() {
    let bottle = Bottle::builder("scout-1")
        .payload(BottlePayload::Discovery(DiscoveryReport {
            subject: "new-fleet-node".into(),
            details: vec!["endpoint: https://new-node.fleet.dev".into()],
            confidence: 0.92,
        }))
        .priority(Priority::Normal)
        .build();

    assert!(bottle.is_broadcast());

    // Roundtrip through binary.
    let binary = protocol::encode(&bottle, WireFormat::BinaryV1).unwrap();
    let decoded = protocol::decode(&binary).unwrap();
    assert_eq!(bottle, decoded);
}

#[test]
fn consensus_voting_flow() {
    let votes: Vec<Bottle> = ["alpha", "bravo", "charlie"]
        .iter()
        .map(|agent| {
            Bottle::builder(*agent)
                .to("coordinator")
                .payload(BottlePayload::Consensus(ConsensusVote {
                    round_id: "round-42".into(),
                    proposal: "upgrade-fleet-v2".into(),
                    vote: if *agent == "charlie" {
                        VoteValue::No
                    } else {
                        VoteValue::Yes
                    },
                }))
                .build()
        })
        .collect();

    assert_eq!(votes.len(), 3);

    // All bottles survive JSON roundtrip.
    for vote in &votes {
        let json = protocol::encode(vote, WireFormat::Json).unwrap();
        let decoded = protocol::decode(&json).unwrap();
        assert_eq!(*vote, decoded);
    }
}

#[test]
fn alert_with_expiry() {
    let bottle = Bottle::builder("monitor")
        .payload(BottlePayload::Alert(AlertMessage {
            severity: AlertSeverity::Critical,
            message: "vector index corruption detected".into(),
            action: Some("restart indexing pipeline".into()),
        }))
        .priority(Priority::Emergency)
        .ttl(Duration::minutes(5))
        .build();

    // Not expired immediately.
    assert!(!bottle.is_expired(Utc::now()));

    // Expired after TTL.
    assert!(bottle.is_expired(bottle.timestamp + Duration::minutes(10)));

    assert_eq!(bottle.priority, Priority::Emergency);
}

#[test]
fn transport_memory_full_flow() {
    use fleet_bottle::transport::MemoryTransport;

    let transport = MemoryTransport::new(WireFormat::Json);

    // Send multiple bottles.
    let b1 = Bottle::builder("a")
        .to("b")
        .payload(BottlePayload::Text("msg-1".into()))
        .build();
    let b2 = Bottle::builder("a")
        .to("b")
        .payload(BottlePayload::Text("msg-2".into()))
        .build();

    transport.send(&b1).unwrap();
    transport.send(&b2).unwrap();

    let received = transport.receive().unwrap();
    assert_eq!(received.len(), 2);
    assert_eq!(received[0], b1);
    assert_eq!(received[1], b2);

    // Buffer is drained.
    assert!(transport.receive().unwrap().is_empty());
}

#[test]
fn state_snapshot() {
    let bottle = Bottle::builder("vector-worker")
        .to("controller")
        .payload(BottlePayload::State(
            BottleState::new("vector-index")
                .entry("vectors", "1012")
                .entry("dimensions", "384")
                .entry("model", "bge-small-en-v1.5"),
        ))
        .build();

    let json = protocol::encode(&bottle, WireFormat::Json).unwrap();
    let decoded = protocol::decode(&json).unwrap();

    if let BottlePayload::State(state) = &decoded.payload {
        assert_eq!(state.component, "vector-index");
        assert_eq!(state.data.len(), 3);
    } else {
        panic!("expected State payload");
    }
}
