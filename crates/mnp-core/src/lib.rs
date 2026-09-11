//! Shared MNP library. The protocol parser lives only here.
//!
//! v0.0.2: laboratory binary frames on one QUIC bi-di stream.

pub mod access;
pub mod frame;
pub mod identity;
pub mod lab;
pub mod nitrokey;
pub mod observer;
pub mod session;

/// Laboratory ALPN. Throwaway, not a v1 contract.
pub const LAB_ALPN: &[u8] = b"mnp-lab/0";

/// TLS server name used with the lab certificate (SAN DNS: localhost).
pub const LAB_SERVER_NAME: &str = "localhost";

/// Default lab listen address. IPv6 loopback, not a v1 port allocation.
pub const LAB_LISTEN: &str = "[::1]:4433";
