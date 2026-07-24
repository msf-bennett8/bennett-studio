//! Wire frame encoding for tunneled MySQL/Postgres byte streams (Phase 2).
//! See engine/src/sharing/multiplex/wire_frame.rs for the matching copy —
//! duplicated across crates since engine and relay don't share a common
//! internal library. Keep both in sync if this format ever changes.
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
