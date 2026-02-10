//! Length-prefixed framing for TCP messages.
//!
//! Wire format: `[u32 length (big-endian)][JSON payload]`

use std::io::{self, Read, Write};

use serde::{de::DeserializeOwned, Serialize};

/// Write a length-prefixed JSON message to a stream.
pub fn write_message<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> io::Result<()> {
    let payload = serde_json::to_vec(msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;

    Ok(())
}

/// Serialize a message into a complete length-prefixed frame (header + JSON payload).
///
/// Use with [`write_raw_frame`] to broadcast a pre-serialized message to multiple writers
/// without re-serializing for each one.
pub fn serialize_frame<T: Serialize>(msg: &T) -> io::Result<Vec<u8>> {
    let payload = serde_json::to_vec(msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let len = payload.len() as u32;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&payload);

    Ok(frame)
}

/// Write a pre-serialized frame (from [`serialize_frame`]) to a stream.
pub fn write_raw_frame<W: Write>(writer: &mut W, frame: &[u8]) -> io::Result<()> {
    writer.write_all(frame)?;
    writer.flush()?;
    Ok(())
}

/// Read a length-prefixed JSON message from a stream.
pub fn read_message<R: Read, T: DeserializeOwned>(reader: &mut R) -> io::Result<T> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Sanity check: reject messages larger than 100MB
    if len > 100_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("message too large: {} bytes", len),
        ));
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;

    serde_json::from_slice(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip_string() {
        let mut buf = Vec::new();
        write_message(&mut buf, &"hello world".to_string()).unwrap();

        let mut cursor = Cursor::new(buf);
        let result: String = read_message(&mut cursor).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn roundtrip_struct() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct TestMsg {
            id: u32,
            name: String,
        }

        let msg = TestMsg {
            id: 42,
            name: "test".to_string(),
        };

        let mut buf = Vec::new();
        write_message(&mut buf, &msg).unwrap();

        let mut cursor = Cursor::new(buf);
        let result: TestMsg = read_message(&mut cursor).unwrap();
        assert_eq!(result, msg);
    }

    #[test]
    fn serialize_frame_roundtrip() {
        let msg = "hello frame".to_string();
        let frame = serialize_frame(&msg).unwrap();

        let mut cursor = Cursor::new(frame);
        let result: String = read_message(&mut cursor).unwrap();
        assert_eq!(result, msg);
    }

    #[test]
    fn serialize_frame_matches_write_message() {
        #[derive(Debug, Serialize, serde::Deserialize)]
        struct TestMsg {
            id: u32,
            data: Vec<u8>,
        }

        let msg = TestMsg {
            id: 99,
            data: vec![1, 2, 3],
        };

        let frame = serialize_frame(&msg).unwrap();

        let mut buf = Vec::new();
        write_message(&mut buf, &msg).unwrap();

        assert_eq!(frame, buf);
    }

    #[test]
    fn write_raw_frame_roundtrip() {
        let msg = "raw frame test".to_string();
        let frame = serialize_frame(&msg).unwrap();

        let mut buf = Vec::new();
        write_raw_frame(&mut buf, &frame).unwrap();

        let mut cursor = Cursor::new(buf);
        let result: String = read_message(&mut cursor).unwrap();
        assert_eq!(result, msg);
    }
}
