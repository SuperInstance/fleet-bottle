use crate::protocol::{self, WireError, WireFormat};
use crate::Bottle;

/// Transport abstraction for sending and receiving bottles.
///
/// Implement this trait to add new transport mechanisms (HTTP, WebSocket,
/// file system, stdio, etc.) to the fleet bottle protocol.
pub trait Transport {
    /// Transport-specific error type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Send a bottle over this transport.
    fn send(&self, bottle: &Bottle) -> Result<(), Self::Error>;

    /// Receive bottles that are available right now (non-blocking).
    fn receive(&self) -> Result<Vec<Bottle>, Self::Error>;
}

/// A transport that uses a shared byte buffer (for testing and stdio-style use).
pub struct MemoryTransport {
    wire_format: WireFormat,
    buffer: std::sync::Mutex<Vec<Vec<u8>>>,
}

impl MemoryTransport {
    pub fn new(wire_format: WireFormat) -> Self {
        Self {
            wire_format,
            buffer: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Inject a raw encoded bottle into the receive buffer (for testing).
    pub fn inject_raw(&self, data: Vec<u8>) {
        self.buffer.lock().unwrap().push(data);
    }

    /// Inject a bottle by encoding it with this transport's wire format.
    pub fn inject(&self, bottle: &Bottle) -> Result<(), WireError> {
        let data = protocol::encode(bottle, self.wire_format)?;
        self.buffer.lock().unwrap().push(data);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryTransportError {
    #[error("wire error: {0}")]
    Wire(#[from] WireError),
    #[error("lock poisoned")]
    LockPoisoned,
}

impl Transport for MemoryTransport {
    type Error = MemoryTransportError;

    fn send(&self, bottle: &Bottle) -> Result<(), Self::Error> {
        let data = protocol::encode(bottle, self.wire_format)?;
        self.buffer.lock().unwrap().push(data);
        Ok(())
    }

    fn receive(&self) -> Result<Vec<Bottle>, Self::Error> {
        let mut buf = self.buffer.lock().unwrap();
        let bottles: Result<Vec<Bottle>, _> = buf
            .drain(..)
            .map(|data| protocol::decode(&data).map_err(Into::into))
            .collect();
        bottles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::BottlePayload;
    use crate::Priority;

    #[test]
    fn memory_transport_json_roundtrip() {
        let transport = MemoryTransport::new(WireFormat::Json);

        let bottle = Bottle::builder("sender")
            .to("receiver")
            .payload(BottlePayload::Text("via memory".into()))
            .priority(Priority::Normal)
            .build();

        transport.send(&bottle).unwrap();
        let received = transport.receive().unwrap();

        assert_eq!(received.len(), 1);
        assert_eq!(received[0], bottle);
    }

    #[test]
    fn memory_transport_binary_roundtrip() {
        let transport = MemoryTransport::new(WireFormat::BinaryV1);

        let bottle = Bottle::builder("sender")
            .to("receiver")
            .payload(BottlePayload::Text("via binary".into()))
            .build();

        transport.send(&bottle).unwrap();
        let received = transport.receive().unwrap();

        assert_eq!(received.len(), 1);
        assert_eq!(received[0], bottle);
    }

    #[test]
    fn memory_transport_multiple_bottles() {
        let transport = MemoryTransport::new(WireFormat::Json);

        let b1 = Bottle::builder("a")
            .payload(BottlePayload::Text("one".into()))
            .build();
        let b2 = Bottle::builder("b")
            .payload(BottlePayload::Text("two".into()))
            .build();

        transport.send(&b1).unwrap();
        transport.send(&b2).unwrap();

        let received = transport.receive().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0], b1);
        assert_eq!(received[1], b2);
    }

    #[test]
    fn receive_drains_buffer() {
        let transport = MemoryTransport::new(WireFormat::Json);

        let bottle = Bottle::builder("a")
            .payload(BottlePayload::Text("once".into()))
            .build();

        transport.send(&bottle).unwrap();
        let _ = transport.receive().unwrap();
        let empty = transport.receive().unwrap();

        assert!(empty.is_empty());
    }
}
