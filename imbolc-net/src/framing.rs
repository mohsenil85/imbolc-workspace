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
}
