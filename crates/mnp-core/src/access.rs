//! Identity-aware service access. Not RDP. Not a VPN.

use thiserror::Error;

use crate::identity::Announce;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceId {
    Echo = 1,
    Ssh = 2,
    Rdp = 3,
    Vnc = 4,
}

impl TryFrom<u8> for ServiceId {
    type Error = AccessError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Echo),
            2 => Ok(Self::Ssh),
            3 => Ok(Self::Rdp),
            4 => Ok(Self::Vnc),
            other => Err(AccessError::UnknownService(other)),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AccessError {
    #[error("unknown service {0}")]
    UnknownService(u8),
    #[error("access denied for {0:?}")]
    Denied(ServiceId),
    #[error("bad access payload")]
    BadPayload,
}

/// Lab policy: which announce may call which service.
#[derive(Debug, Clone)]
pub struct Policy {
    allowed: Vec<(Announce, ServiceId)>,
}

impl Policy {
    /// Pinned peer may use echo only.
    pub fn lab_echo_for(who: &Announce) -> Self {
        Self {
            allowed: vec![(who.clone(), ServiceId::Echo)],
        }
    }

    pub fn allows(&self, who: &Announce, service: ServiceId) -> bool {
        self.allowed
            .iter()
            .any(|(ann, svc)| ann == who && *svc == service)
    }
}

pub fn encode_request(service: ServiceId) -> Vec<u8> {
    vec![service as u8]
}

pub fn decode_request(buf: &[u8]) -> Result<ServiceId, AccessError> {
    let b = buf.first().copied().ok_or(AccessError::BadPayload)?;
    ServiceId::try_from(b)
}

pub fn encode_grant(service: ServiceId) -> Vec<u8> {
    vec![service as u8]
}

pub fn decode_grant(buf: &[u8]) -> Result<ServiceId, AccessError> {
    decode_request(buf)
}

pub fn encode_deny(service: ServiceId, reason: u8) -> Vec<u8> {
    vec![service as u8, reason]
}

pub const ECHO_PING: &[u8] = b"ping";
pub const ECHO_PONG: &[u8] = b"pong";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{IdentityKeys, KIND_HUMAN};

    #[test]
    fn echo_allowed_ssh_denied() {
        let who = IdentityKeys::generate(KIND_HUMAN).unwrap().announce();
        let policy = Policy::lab_echo_for(&who);
        assert!(policy.allows(&who, ServiceId::Echo));
        assert!(!policy.allows(&who, ServiceId::Ssh));
        let other = IdentityKeys::generate(KIND_HUMAN).unwrap().announce();
        assert!(!policy.allows(&other, ServiceId::Echo));
    }
}
