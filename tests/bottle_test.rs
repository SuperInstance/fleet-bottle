use fleet_bottle::*;
use chrono::{Duration, Utc};

#[test]
fn broadcast_bottle() {
    let bottle = Bottle::builder("agent-alpha")
        .payload(BottlePayload::Text("All hands on deck".into()))
        .priority(Priority::High)
        .build();

    assert!(bottle.is_broadcast());
    assert!(!bottle.is_directed());
    assert_eq!(bottle.from, "agent-alpha");
    assert!(bottle.to.is_none());
    assert_eq!(bottle.priority, Priority::High);
}

#[test]
fn directed_bottle() {
    let bottle = Bottle::builder("alpha")
        .to("beta")
        .payload(BottlePayload::Text("Report status".into()))
        .build();

    assert!(!bottle.is_broadcast());
    assert!(bottle.is_directed());
    assert_eq!(bottle.to.as_ref().unwrap(), "beta");
}

#[test]
fn ttl_expiry() {
    let bottle = Bottle::builder("a")
        .to("b")
        .payload(BottlePayload::Text("ephemeral".into()))
        .ttl(Duration::seconds(60))
        .build();

    assert!(!bottle.is_expired(Utc::now()));
    let future = bottle.timestamp + Duration::seconds(120);
    assert!(bottle.is_expired(future));
    assert!(bottle.expires_at().is_some());
}

#[test]
fn no_ttl_never_expires() {
    let bottle = Bottle::builder("a")
        .payload(BottlePayload::Text("forever".into()))
        .build();

    assert!(!bottle.is_expired(Utc::now()));
    assert!(bottle.expires_at().is_none());
}

#[test]
fn metadata_attachment() {
    let bottle = Bottle::builder("a")
        .to("b")
        .payload(BottlePayload::Text("tagged".into()))
        .meta("region", serde_json::json!("us-west"))
        .meta("count", serde_json::json!(42))
        .build();

    assert_eq!(bottle.metadata.get("region").unwrap(), "us-west");
    assert_eq!(bottle.metadata.get("count").unwrap(), 42);
}

#[test]
fn priority_ordering() {
    assert!(Priority::Emergency > Priority::High);
    assert!(Priority::High > Priority::Normal);
    assert!(Priority::Normal > Priority::Low);
}

#[test]
fn command_payload() {
    let cmd = BottleCommand::new("restart")
        .arg("service-a")
        .arg("--force")
        .expect_ack();

    assert_eq!(cmd.name, "restart");
    assert_eq!(cmd.args, vec!["service-a", "--force"]);
    assert!(cmd.expects_ack);
}

#[test]
fn state_payload() {
    let state = BottleState::new("vector-index")
        .entry("vectors", "1012")
        .entry("dimensions", "384");

    assert_eq!(state.component, "vector-index");
    assert_eq!(state.data.len(), 2);
}

#[test]
fn consensus_payload() {
    let vote = ConsensusVote {
        round_id: "round-42".into(),
        proposal: "deploy v2".into(),
        vote: VoteValue::Yes,
    };

    let bottle = Bottle::builder("voter-1")
        .payload(BottlePayload::Consensus(vote))
        .build();

    let json = serde_json::to_vec(&bottle).unwrap();
    let decoded: Bottle = serde_json::from_slice(&json).unwrap();

    if let BottlePayload::Consensus(v) = decoded.payload {
        assert_eq!(v.round_id, "round-42");
        assert_eq!(v.vote, VoteValue::Yes);
    } else {
        panic!("expected Consensus payload");
    }
}

#[test]
fn alert_payload() {
    let alert = AlertMessage {
        severity: AlertSeverity::Critical,
        message: "Out of memory".into(),
        action: Some("restart worker".into()),
    };

    let bottle = Bottle::builder("monitor")
        .payload(BottlePayload::Alert(alert))
        .build();

    let json = serde_json::to_vec(&bottle).unwrap();
    let decoded: Bottle = serde_json::from_slice(&json).unwrap();

    if let BottlePayload::Alert(a) = decoded.payload {
        assert_eq!(a.severity, AlertSeverity::Critical);
        assert_eq!(a.action.unwrap(), "restart worker");
    } else {
        panic!("expected Alert payload");
    }
}

#[test]
fn json_wire_roundtrip() {
    let bottle = Bottle::builder("alpha")
        .to("beta")
        .payload(BottlePayload::Text("hello wire".into()))
        .priority(Priority::Emergency)
        .ttl(Duration::seconds(300))
        .meta("source", serde_json::json!("test"))
        .build();

    let encoded = protocol::encode(&bottle, WireFormat::Json).unwrap();
    let decoded = protocol::decode(&encoded).unwrap();

    assert_eq!(bottle, decoded);
}

#[test]
fn binary_wire_roundtrip() {
    let bottle = Bottle::builder("alpha")
        .to("beta")
        .payload(BottlePayload::Text("binary test".into()))
        .priority(Priority::High)
        .build();

    let encoded = protocol::encode(&bottle, WireFormat::BinaryV1).unwrap();
    let decoded = protocol::decode(&encoded).unwrap();

    assert_eq!(bottle, decoded);
}

#[test]
fn auto_detect_format() {
    let bottle = Bottle::builder("x")
        .payload(BottlePayload::Text("auto-detect".into()))
        .build();

    // JSON auto-detected
    let json_bytes = protocol::encode(&bottle, WireFormat::Json).unwrap();
    let decoded_json = protocol::decode(&json_bytes).unwrap();
    assert_eq!(bottle, decoded_json);

    // Binary auto-detected via magic bytes
    let bin_bytes = protocol::encode(&bottle, WireFormat::BinaryV1).unwrap();
    let decoded_bin = protocol::decode(&bin_bytes).unwrap();
    assert_eq!(bottle, decoded_bin);
}

#[test]
fn memory_transport_roundtrip() {
    let transport = MemoryTransport::new(WireFormat::Json);

    let b1 = Bottle::builder("a")
        .payload(BottlePayload::Text("one".into()))
        .build();
    let b2 = Bottle::builder("b")
        .to("c")
        .payload(BottlePayload::Text("two".into()))
        .priority(Priority::Low)
        .build();

    transport.send(&b1).unwrap();
    transport.send(&b2).unwrap();

    let received = transport.receive().unwrap();
    assert_eq!(received.len(), 2);
    assert_eq!(received[0], b1);
    assert_eq!(received[1], b2);

    // Buffer is drained
    let empty = transport.receive().unwrap();
    assert!(empty.is_empty());}
