//! # Fleet Bottle Protocol
//!
//! Inter-agent messaging library for the SuperInstance fleet.
//! Bottles are immutable messages passed between agents — like messages
//! in bottles between ships at sea.

pub mod bottle;
pub mod payload;
pub mod protocol;
pub mod transport;

pub use bottle::{Bottle, BottleBuilder, Priority};
pub use payload::{
    AlertMessage, AlertSeverity, BottleCommand, BottlePayload, BottleState,
    ConsensusVote, DiscoveryReport, VoteValue,
};
pub use protocol::{WireFormat, WireError, encode, decode};
pub use transport::{Transport, MemoryTransport};

/// Unique identifier for an agent in the fleet.
pub type AgentId = String;
