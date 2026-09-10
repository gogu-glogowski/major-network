//! v0.0.2 laboratory frame. Throwaway layout, not a v1 contract.
//!
//! MAGIC | VERSION | TYPE | FLAGS | LENGTH | SESSION | PAYLOAD
//!  4B        1B       1B     2B      4B       8B       N
//!
//! `SESSION` is always 0 through v0.0.3. No allocator here.

use thiserror::Error;

pub const LAB_MAGIC: [u8; 4] = *b"MNP1";
pub const LAB_VERSION: u8 = 0;
pub const HEADER_LEN: usize = 20;
pub const LAB_MAX_PAYLOAD: u32 = 65_536;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Hello = 0x01,
    HelloAck = 0x02,
    ErrorFrame = 0x03,
    Goodbye = 0x04,
}

impl TryFrom<u8> for MessageType {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, FrameError> {
        match value {
            0x01 => Ok(Self::Hello),
            0x02 => Ok(Self::HelloAck),
            0x03 => Ok(Self::ErrorFrame),
            0x04 => Ok(Self::Goodbye),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub ty: MessageType,
    pub flags: u16,
    pub length: u32,
    pub session: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub ty: MessageType,
    pub flags: u16,
    pub session: u64,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn empty(ty: MessageType) -> Self {
        Self {
            ty,
            flags: 0,
            session: 0,
            payload: Vec::new(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FrameError {
    #[error("header too short: {0} bytes")]
    HeaderTooShort(usize),
    #[error("bad magic: {0:?}")]
    BadMagic([u8; 4]),
    #[error("unsupported version: {0}")]
    BadVersion(u8),
    #[error("unknown TYPE {0}")]
    UnknownType(u8),
    #[error("LENGTH {0} exceeds LAB_MAX_PAYLOAD")]
    PayloadTooLarge(u32),
    #[error("payload too short: want {want} got {got}")]
    PayloadTooShort { want: u32, got: usize },
}

/// Encode a frame. `SESSION` is forced to 0. Does not allocate a payload over the lab cap.
pub fn encode(frame: &Frame) -> Result<Vec<u8>, FrameError> {
    let len =
        u32::try_from(frame.payload.len()).map_err(|_| FrameError::PayloadTooLarge(u32::MAX))?;
    if len > LAB_MAX_PAYLOAD {
        return Err(FrameError::PayloadTooLarge(len));
    }
    let mut out = Vec::with_capacity(HEADER_LEN + frame.payload.len());
    out.extend_from_slice(&LAB_MAGIC);
    out.push(LAB_VERSION);
    out.push(frame.ty as u8);
    out.extend_from_slice(&frame.flags.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend_from_slice(&frame.payload);
    Ok(out)
}

/// Decode only the 20-byte header. Rejects oversize LENGTH **without** allocating payload.
pub fn decode_header(buf: &[u8]) -> Result<FrameHeader, FrameError> {
    if buf.len() < HEADER_LEN {
        return Err(FrameError::HeaderTooShort(buf.len()));
    }
    let magic: [u8; 4] = buf[0..4].try_into().expect("header slice");
    if magic != LAB_MAGIC {
        return Err(FrameError::BadMagic(magic));
    }
    let version = buf[4];
    if version != LAB_VERSION {
        return Err(FrameError::BadVersion(version));
    }
    let ty = MessageType::try_from(buf[5])?;
    let flags = u16::from_le_bytes(buf[6..8].try_into().expect("flags"));
    let length = u32::from_le_bytes(buf[8..12].try_into().expect("length"));
    if length > LAB_MAX_PAYLOAD {
        return Err(FrameError::PayloadTooLarge(length));
    }
    let session = u64::from_le_bytes(buf[12..20].try_into().expect("session"));
    Ok(FrameHeader {
        ty,
        flags,
        length,
        session,
    })
}

pub fn decode(buf: &[u8]) -> Result<Frame, FrameError> {
    let header = decode_header(buf)?;
    let rest = &buf[HEADER_LEN..];
    if rest.len() < header.length as usize {
        return Err(FrameError::PayloadTooShort {
            want: header.length,
            got: rest.len(),
        });
    }
    Ok(Frame {
        ty: header.ty,
        flags: header.flags,
        session: header.session,
        payload: rest[..header.length as usize].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_hello() {
        let frame = Frame::empty(MessageType::Hello);
        let bytes = encode(&frame).unwrap();
        assert_eq!(bytes.len(), HEADER_LEN);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.ty, MessageType::Hello);
        assert_eq!(decoded.session, 0);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn header_too_short() {
        let err = decode_header(&[0; 5]).unwrap_err();
        assert_eq!(err, FrameError::HeaderTooShort(5));
    }

    #[test]
    fn unknown_type() {
        let mut bytes = encode(&Frame::empty(MessageType::Hello)).unwrap();
        bytes[5] = 0x99;
        let err = decode_header(&bytes).unwrap_err();
        assert_eq!(err, FrameError::UnknownType(0x99));
    }

    #[test]
    fn oversize_length_does_not_allocate_payload() {
        let mut header = encode(&Frame::empty(MessageType::Hello)).unwrap();
        header[8..12].copy_from_slice(&(LAB_MAX_PAYLOAD + 1).to_le_bytes());
        let err = decode_header(&header).unwrap_err();
        assert_eq!(err, FrameError::PayloadTooLarge(LAB_MAX_PAYLOAD + 1));
        assert!(decode(&header).is_err());
    }

    #[test]
    fn encode_rejects_oversize_payload() {
        let frame = Frame {
            ty: MessageType::ErrorFrame,
            flags: 0,
            session: 0,
            payload: vec![0; (LAB_MAX_PAYLOAD as usize) + 1],
        };
        let err = encode(&frame).unwrap_err();
        assert_eq!(err, FrameError::PayloadTooLarge(LAB_MAX_PAYLOAD + 1));
    }

    #[test]
    fn encode_forces_session_zero() {
        let frame = Frame {
            ty: MessageType::Goodbye,
            flags: 1,
            session: 42,
            payload: b"x".to_vec(),
        };
        let decoded = decode(&encode(&frame).unwrap()).unwrap();
        assert_eq!(decoded.session, 0);
        assert_eq!(decoded.flags, 1);
        assert_eq!(decoded.payload, b"x");
    }
}
