use serde::{Deserialize, Serialize};

/// The contents of a bottle — what the message carries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum BottlePayload {
    /// Plain text message between agents.
    Text(String),
    /// A command to be executed by the receiving agent.
    Command(BottleCommand),
    /// State snapshot or update.
    State(BottleState),
    /// Consensus voting message for distributed decisions.
    Consensus(ConsensusVote),
    /// Discovery report — agent announcing capabilities or findings.
    Discovery(DiscoveryReport),
    /// Alert — high-priority notification requiring attention.
    Alert(AlertMessage),
}

/// A command sent to another agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BottleCommand {
    /// Command name (e.g. "deploy", "health_check").
    pub name: String,
    /// Positional arguments.
    pub args: Vec<String>,
    /// Whether the sender expects an acknowledgement.
    pub expects_ack: bool,
}

impl BottleCommand {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
            expects_ack: false,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn expect_ack(mut self) -> Self {
        self.expects_ack = true;
        self
    }
}

/// State data — key/value pairs representing agent state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BottleState {
    /// The component this state pertains to.
    pub component: String,
    /// State data as string key-value pairs.
    pub data: Vec<(String, String)>,
}

impl BottleState {
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            data: Vec::new(),
        }
    }

    pub fn entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.push((key.into(), value.into()));
        self
    }
}

/// A vote in a consensus round.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusVote {
    /// The consensus round identifier.
    pub round_id: String,
    /// The proposal being voted on.
    pub proposal: String,
    /// The vote value.
    pub vote: VoteValue,
}

/// Possible vote values.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VoteValue {
    Yes,
    No,
    Abstain,
}

/// Discovery report — an agent announcing what it found or can do.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryReport {
    /// What was discovered.
    pub subject: String,
    /// Details about the discovery.
    pub details: Vec<String>,
    /// Confidence level 0.0–1.0.
    pub confidence: f64,
}

/// Alert message — something that needs attention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertMessage {
    /// Alert severity.
    pub severity: AlertSeverity,
    /// What happened.
    pub message: String,
    /// Suggested action (if any).
    pub action: Option<String>,
}

/// Alert severity levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_builder() {
        let cmd = BottleCommand::new("deploy")
            .arg("fleet-edge-worker")
            .arg("--production")
            .expect_ack();
        assert_eq!(cmd.name, "deploy");
        assert_eq!(cmd.args, vec!["fleet-edge-worker", "--production"]);
        assert!(cmd.expects_ack);
    }

    #[test]
    fn state_builder() {
        let state = BottleState::new("vector-index")
            .entry("vectors", "1012")
            .entry("dimensions", "384");
        assert_eq!(state.component, "vector-index");
        assert_eq!(state.data.len(), 2);
    }

    #[test]
    fn payload_serde_roundtrip() {
        let payloads = vec![
            BottlePayload::Text("hello fleet".into()),
            BottlePayload::Command(BottleCommand::new("ping")),
            BottlePayload::State(BottleState::new("engine").entry("status", "ok")),
            BottlePayload::Consensus(ConsensusVote {
                round_id: "r1".into(),
                proposal: "deploy v2".into(),
                vote: VoteValue::Yes,
            }),
            BottlePayload::Discovery(DiscoveryReport {
                subject: "new-crate".into(),
                details: vec!["found in registry".into()],
                confidence: 0.95,
            }),
            BottlePayload::Alert(AlertMessage {
                severity: AlertSeverity::Warning,
                message: "high latency".into(),
                action: Some("check edge nodes".into()),
            }),
        ];

        for payload in &payloads {
            let json = serde_json::to_string(payload).unwrap();
            let back: BottlePayload = serde_json::from_str(&json).unwrap();
            assert_eq!(*payload, back);
        }
    }
}
