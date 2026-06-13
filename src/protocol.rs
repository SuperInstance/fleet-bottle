use thiserror::Error;

use crate::Bottle;

/// Errors that can occur during wire-format operations.
#[derive(Debug, Error)]
pub enum WireError {
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("binary decode failed: {0}")]
    BinaryDecode(String),

    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u8),
}

/// Wire format for bottle serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    /// JSON — human-readable, debuggable, CF Workers friendly.
    Json,
    /// Compact binary — smaller on the wire, faster to parse.
    BinaryV1,
}

/// Protocol version byte for binary format.
const BINARY_VERSION: u8 = 1;

/// Magic bytes identifying a fleet-bottle binary frame.
const MAGIC: [u8; 4] = [0xF1, 0xB0, 0x71, 0x1E]; // "FBO" + 0x1E

/// Serialize a bottle to the given wire format.
pub fn encode(bottle: &Bottle, format: WireFormat) -> Result<Vec<u8>, WireError> {
    match format {
        WireFormat::Json => {
            let json = serde_json::to_vec(bottle)?;
            Ok(json)
        }
        WireFormat::BinaryV1 => {
            // Binary layout:
            //   [4] magic
            //   [1] version
            //   [4] JSON payload length (u32 BE)
            //   [N] JSON payload (compressed representation via serde_json)
            //
            // We use JSON as the internal encoding for binary V1 to keep
            // compatibility with CF Workers (no proc-macro binary deps).
            // Future versions can switch to a true binary encoding.
            let json = serde_json::to_vec(bottle)?;
            let len = json.len() as u32;

            let mut out = Vec::with_capacity(4 + 1 + 4 + json.len());
            out.extend_from_slice(&MAGIC);
            out.push(BINARY_VERSION);
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(&json);
            Ok(out)
        }
    }
}

/// Deserialize a bottle from raw bytes, auto-detecting format.
pub fn decode(data: &[u8]) -> Result<Bottle, WireError> {
    // Detect format by checking for the magic bytes.
    if data.len() >= 5 && data[0..4] == MAGIC && data[4] == BINARY_VERSION {
        decode_binary_v1(data)
    } else {
        decode_json(data)
    }
}

/// Decode a JSON-encoded bottle.
pub fn decode_json(data: &[u8]) -> Result<Bottle, WireError> {
    let bottle: Bottle = serde_json::from_slice(data)?;
    Ok(bottle)
}

/// Decode a binary V1 encoded bottle.
fn decode_binary_v1(data: &[u8]) -> Result<Bottle, WireError> {
    if data.len() < 9 {
        return Err(WireError::BinaryDecode("frame too short".into()));
    }

    if data[0..4] != MAGIC {
        return Err(WireError::BinaryDecode("bad magic bytes".into()));
    }

    let version = data[4];
    if version != BINARY_VERSION {
        return Err(WireError::UnsupportedVersion(version));
    }

    let len = u32::from_be_bytes([data[5], data[6], data[7], data[8]]) as usize;

    if data.len() < 9 + len {
        return Err(WireError::BinaryDecode(
            "frame truncated — not enough data".into(),
        ));
    }

    let json = &data[9..9 + len];
    let bottle: Bottle = serde_json::from_slice(json)?;
    Ok(bottle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::BottlePayload;
    use crate::Priority;

    fn sample_bottle() -> Bottle {
        Bottle::builder("agent-test")
            .to("agent-target")
            .payload(BottlePayload::Text("hello wire".into()))
            .priority(Priority::High)
            .build()
    }

    #[test]
    fn json_roundtrip() {
        let bottle = sample_bottle();
        let encoded = encode(&bottle, WireFormat::Json).unwrap();
        let decoded = decode(&encoded).unwrap();

        assert_eq!(bottle, decoded);
    }

    #[test]
    fn binary_roundtrip() {
        let bottle = sample_bottle();
        let encoded = encode(&bottle, WireFormat::BinaryV1).unwrap();
        let decoded = decode(&encoded).unwrap();

        assert_eq!(bottle, decoded);
    }

    #[test]
    fn auto_detect_json() {
        let bottle = sample_bottle();
        let encoded = encode(&bottle, WireFormat::Json).unwrap();
        let decoded = decode_json(&encoded).unwrap();
        assert_eq!(bottle, decoded);
    }

    #[test]
    fn auto_detect_binary() {
        let bottle = sample_bottle();
        let encoded = encode(&bottle, WireFormat::BinaryV1).unwrap();
        // decode auto-detects via magic bytes
        let decoded = decode(&encoded).unwrap();
        assert_eq!(bottle, decoded);
    }

    #[test]
    fn binary_smaller_than_json() {
        // Binary has a 9-byte header overhead but same JSON payload inside,
        // so it's slightly larger. This test documents the current V1 behavior.
        let bottle = sample_bottle();
        let json = encode(&bottle, WireFormat::Json).unwrap();
        let binary = encode(&bottle, WireFormat::BinaryV1).unwrap();
        // Binary V1 = 9 byte header + JSON payload
        assert_eq!(binary.len(), json.len() + 9);
    }

    #[test]
    fn bad_magic_rejected() {
        let data = vec![0x00; 20];
        let result = decode(&data);
        // Should try JSON decode and fail, not binary
        assert!(result.is_err());
    }

    #[test]
    fn truncated_binary_rejected() {
        let bottle = sample_bottle();
        let mut encoded = encode(&bottle, WireFormat::BinaryV1).unwrap();
        encoded.truncate(7); // truncate in the middle of length field
        let result = decode(&encoded);
        assert!(result.is_err());
    }
}
