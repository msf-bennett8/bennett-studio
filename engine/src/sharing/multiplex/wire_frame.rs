//! Wire frame encoding for tunneled MySQL/Postgres byte streams (Phase 2).
//! Binary WebSocket frames (not JSON/base64) carry raw wire-protocol bytes
//! to keep bulk-transfer overhead near zero — this is the path bulk
//! imports/exports and large result sets travel over. Only stream
//! lifecycle (open/opened/error/close) travels as JSON control messages
//! via the existing TunnelMessage enum.
//!
//! Frame layout: [1 byte type][1 byte stream_id_len][stream_id bytes][payload bytes]

pub const WIRE_FRAME_TYPE_DATA: u8 = 0x01;

pub fn encode_wire_frame(stream_id: &str, payload: &[u8]) -> Vec<u8> {
    let sid = stream_id.as_bytes();
    let mut out = Vec::with_capacity(2 + sid.len() + payload.len());
    out.push(WIRE_FRAME_TYPE_DATA);
    out.push(sid.len() as u8);
    out.extend_from_slice(sid);
    out.extend_from_slice(payload);
    out
}

/// Returns (msg_type, stream_id, payload), or None if malformed/truncated.
pub fn decode_wire_frame(frame: &[u8]) -> Option<(u8, &str, &[u8])> {
    if frame.len() < 2 {
        return None;
    }
    let msg_type = frame[0];
    let sid_len = frame[1] as usize;
    if frame.len() < 2 + sid_len {
        return None;
    }
    let stream_id = std::str::from_utf8(&frame[2..2 + sid_len]).ok()?;
    let payload = &frame[2 + sid_len..];
    Some((msg_type, stream_id, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let frame = encode_wire_frame("abc123", b"hello world");
        let (msg_type, stream_id, payload) = decode_wire_frame(&frame).unwrap();
        assert_eq!(msg_type, WIRE_FRAME_TYPE_DATA);
        assert_eq!(stream_id, "abc123");
        assert_eq!(payload, b"hello world");
    }
}
