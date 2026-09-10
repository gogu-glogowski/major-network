//! Shared MNP library. The protocol parser lives only here.
//!
//! v0.0.1: raw ASCII HELLO on a QUIC stream. No framing yet.

/// Laboratory ALPN. Throwaway, not a v1 contract.
pub const LAB_ALPN: &[u8] = b"mnp-lab/0";

/// v0.0.1 client payload. Exact bytes, no frame.
pub const HELLO: &[u8] = b"HELLO MNP";

/// v0.0.1 server payload. Exact bytes, no frame.
pub const HELLO_ACK: &[u8] = b"HELLO ACK";

/// TLS server name used with the lab certificate (SAN DNS: localhost).
pub const LAB_SERVER_NAME: &str = "localhost";
