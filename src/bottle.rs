use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use crate::payload::BottlePayload;
use crate::AgentId;

/// Message priority level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Emergency = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// A Bottle — an immutable inter-agent message.
///
/// Bottles are created via [`BottleBuilder`] and are immutable once built.
/// They carry a payload, optional TTL for expiry, and a priority level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bottle {
    /// Unique message identifier.
    pub id: Uuid,
    /// Sending agent.
    pub from: AgentId,
    /// Receiving agent. `None` means broadcast to all agents.
    pub to: Option<AgentId>,
    /// Message contents.
    pub payload: BottlePayload,
    /// When this bottle was created (UTC).
    pub timestamp: DateTime<Utc>,
    /// Time-to-live. The bottle is considered expired after `timestamp + ttl`.
    pub ttl: Option<Duration>,
    /// Delivery priority.
    pub priority: Priority,
    /// Arbitrary metadata attachments.
    pub metadata: HashMap<String, Value>,
}

impl Bottle {
    /// Create a new builder for a bottle from the given agent.
    pub fn builder(from: impl Into<AgentId>) -> BottleBuilder {
        BottleBuilder::new(from)
    }

    /// Check whether this bottle has expired relative to the given time.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        match self.ttl {
            Some(ttl) => now > self.timestamp + ttl,
            None => false,
        }
    }

    /// Returns the expiry time, if a TTL is set.
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.ttl.map(|ttl| self.timestamp + ttl)
    }

    /// Whether this bottle is addressed to a specific agent (not broadcast).
    pub fn is_directed(&self) -> bool {
        self.to.is_some()
    }

    /// Whether this is a broadcast bottle.
    pub fn is_broadcast(&self) -> bool {
        self.to.is_none()
    }
}

/// Builder for constructing [`Bottle`] instances.
pub struct BottleBuilder {
    from: AgentId,
    to: Option<AgentId>,
    payload: Option<BottlePayload>,
    ttl: Option<Duration>,
    priority: Priority,
    metadata: HashMap<String, Value>,
}

impl BottleBuilder {
    pub fn new(from: impl Into<AgentId>) -> Self {
        Self {
            from: from.into(),
            to: None,
            payload: None,
            ttl: None,
            priority: Priority::Normal,
            metadata: HashMap::new(),
        }
    }

    /// Address this bottle to a specific agent.
    pub fn to(mut self, agent: impl Into<AgentId>) -> Self {
        self.to = Some(agent.into());
        self
    }

    /// Set the payload.
    pub fn payload(mut self, payload: BottlePayload) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Set time-to-live.
    pub fn ttl(mut self, duration: Duration) -> Self {
        self.ttl = Some(duration);
        self
    }

    /// Set priority.
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Attach a metadata entry.
    pub fn meta(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Build the bottle. Panics if no payload was set.
    pub fn build(self) -> Bottle {
        let payload = self
            .payload
            .expect("BottleBuilder requires a payload — call .payload() before .build()");

        Bottle {
            id: Uuid::new_v4(),
            from: self.from,
            to: self.to,
            payload,
            timestamp: Utc::now(),
            ttl: self.ttl,
            priority: self.priority,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_directed_bottle() {
        let bottle = Bottle::builder("agent-alpha")
            .to("agent-bravo")
            .payload(BottlePayload::Text("status report".into()))
            .priority(Priority::High)
            .build();

        assert_eq!(bottle.from, "agent-alpha");
        assert_eq!(bottle.to.as_ref().unwrap(), "agent-bravo");
        assert!(bottle.is_directed());
        assert!(!bottle.is_broadcast());
        assert_eq!(bottle.priority, Priority::High);
    }

    #[test]
    fn build_broadcast_bottle() {
        let bottle = Bottle::builder("agent-alpha")
            .payload(BottlePayload::Discovery(crate::payload::DiscoveryReport {
                subject: "new-node".into(),
                details: vec!["joined fleet".into()],
                confidence: 1.0,
            }))
            .build();

        assert!(bottle.is_broadcast());
        assert!(!bottle.is_directed());
        assert!(bottle.to.is_none());
    }

    #[test]
    fn ttl_expiry() {
        let bottle = Bottle::builder("a")
            .to("b")
            .payload(BottlePayload::Text("ephemeral".into()))
            .ttl(Duration::seconds(60))
            .build();

        // Not expired yet (created just now).
        assert!(!bottle.is_expired(Utc::now()));

        // Expired after TTL.
        let future = bottle.timestamp + Duration::seconds(120);
        assert!(bottle.is_expired(future));

        // expires_at is set.
        assert!(bottle.expires_at().is_some());
    }

    #[test]
    fn no_ttl_never_expires() {
        let bottle = Bottle::builder("a")
            .payload(BottlePayload::Text("permanent".into()))
            .build();

        assert!(!bottle.is_expired(Utc::now()));
        assert!(bottle.expires_at().is_none());
    }

    #[test]
    fn metadata_attachment() {
        let bottle = Bottle::builder("a")
            .to("b")
            .payload(BottlePayload::Text("with-meta".into()))
            .meta("trace_id", Value::String("abc-123".into()))
            .meta("retry_count", Value::Number(3.into()))
            .build();

        assert_eq!(bottle.metadata.get("trace_id").unwrap(), "abc-123");
        assert_eq!(bottle.metadata.get("retry_count").unwrap(), 3);
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Emergency > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }
}
