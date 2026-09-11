//! Network Observer MVP — application module, not a wire layer.
//!
//! Scope is **not** frozen. This snapshot is hostname, OS, uptime, IPv6
//! addresses. Routes/DNS/neighbors stay out until we need them.

use std::net::Ipv6Addr;
use std::time::Duration;

use thiserror::Error;

pub const OBSERVER_VERSION: u8 = 1;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ObserverError {
    #[error("bad observer payload")]
    BadPayload,
    #[error("unsupported observer version {0}")]
    BadVersion(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addr {
    pub iface: String,
    pub addr: Ipv6Addr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub hostname: String,
    pub os: String,
    pub uptime: Duration,
    pub ipv6: Vec<Addr>,
}

impl Snapshot {
    pub fn from_host() -> Self {
        Self {
            hostname: hostname(),
            os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            uptime: uptime(),
            ipv6: ipv6_addrs(),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ObserverError> {
        let mut out = Vec::new();
        out.push(OBSERVER_VERSION);
        push_str(&mut out, &self.hostname)?;
        push_str(&mut out, &self.os)?;
        out.extend_from_slice(
            &u64::try_from(self.uptime.as_secs())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        let n = u16::try_from(self.ipv6.len()).map_err(|_| ObserverError::BadPayload)?;
        out.extend_from_slice(&n.to_le_bytes());
        for a in &self.ipv6 {
            push_str(&mut out, &a.iface)?;
            out.extend_from_slice(&a.addr.octets());
        }
        Ok(out)
    }

    pub fn decode(buf: &[u8]) -> Result<Self, ObserverError> {
        if buf.is_empty() {
            return Err(ObserverError::BadPayload);
        }
        let version = buf[0];
        if version != OBSERVER_VERSION {
            return Err(ObserverError::BadVersion(version));
        }
        let mut rest = &buf[1..];
        let hostname = take_str(&mut rest)?;
        let os = take_str(&mut rest)?;
        if rest.len() < 8 + 2 {
            return Err(ObserverError::BadPayload);
        }
        let uptime_secs = u64::from_le_bytes(rest[..8].try_into().expect("u64"));
        rest = &rest[8..];
        let n = u16::from_le_bytes(rest[..2].try_into().expect("u16")) as usize;
        rest = &rest[2..];
        let mut ipv6 = Vec::with_capacity(n);
        for _ in 0..n {
            let iface = take_str(&mut rest)?;
            if rest.len() < 16 {
                return Err(ObserverError::BadPayload);
            }
            let octets: [u8; 16] = rest[..16].try_into().expect("ip");
            rest = &rest[16..];
            ipv6.push(Addr {
                iface,
                addr: Ipv6Addr::from(octets),
            });
        }
        Ok(Self {
            hostname,
            os,
            uptime: Duration::from_secs(uptime_secs),
            ipv6,
        })
    }
}

fn push_str(out: &mut Vec<u8>, s: &str) -> Result<(), ObserverError> {
    let bytes = s.as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| ObserverError::BadPayload)?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

fn take_str(buf: &mut &[u8]) -> Result<String, ObserverError> {
    if buf.len() < 2 {
        return Err(ObserverError::BadPayload);
    }
    let len = u16::from_le_bytes(buf[..2].try_into().expect("u16")) as usize;
    *buf = &buf[2..];
    if buf.len() < len {
        return Err(ObserverError::BadPayload);
    }
    let s = std::str::from_utf8(&buf[..len]).map_err(|_| ObserverError::BadPayload)?;
    *buf = &buf[len..];
    Ok(s.to_string())
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn uptime() -> Duration {
    let raw = std::fs::read_to_string("/proc/uptime").unwrap_or_default();
    let secs = raw
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    Duration::from_secs(secs as u64)
}

fn ipv6_addrs() -> Vec<Addr> {
    let Ok(text) = std::fs::read_to_string("/proc/net/if_inet6") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(hex) = parts.next() else { continue };
        let Some(iface) = parts.nth(4) else { continue };
        if hex.len() != 32 {
            continue;
        }
        let mut octets = [0u8; 16];
        let mut bad = false;
        for i in 0..16 {
            if let Ok(b) = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16) {
                octets[i] = b;
            } else {
                bad = true;
                break;
            }
        }
        if bad {
            continue;
        }
        out.push(Addr {
            iface: iface.to_string(),
            addr: Ipv6Addr::from(octets),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trip() {
        let snap = Snapshot {
            hostname: "lab".into(),
            os: "linux-x86_64".into(),
            uptime: Duration::from_secs(42),
            ipv6: vec![Addr {
                iface: "lo".into(),
                addr: Ipv6Addr::LOCALHOST,
            }],
        };
        let decoded = Snapshot::decode(&snap.encode().unwrap()).unwrap();
        assert_eq!(decoded, snap);
    }

    #[test]
    fn from_host_has_os() {
        let snap = Snapshot::from_host();
        assert!(snap.os.contains("linux") || !snap.os.is_empty());
    }
}
