//! Software-key identity: announce / challenge / proof.
//!
//! See `docs/IDENTITY.md`. This is not Nitrokey and not MNP transport crypto.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use thiserror::Error;

pub const KIND_HUMAN: u8 = 1;
pub const KIND_PEER: u8 = 3;
pub const PK_LEN: usize = 32;
pub const SIG_LEN: usize = 64;
pub const NONCE_LEN: usize = 32;
pub const ANNOUNCE_LEN: usize = 1 + PK_LEN + PK_LEN;
pub const PROOF_LEN: usize = SIG_LEN + SIG_LEN;
pub const EXPORTER_LEN: usize = 32;
pub const EXPORTER_LABEL: &[u8] = b"EXPORTER-MNP-Identity";
const DOMAIN: &[u8] = b"MNP-IDENTITY-V0";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentityError {
    #[error("bad announce length {0}")]
    BadAnnounce(usize),
    #[error("bad challenge length {0}")]
    BadChallenge(usize),
    #[error("bad proof length {0}")]
    BadProof(usize),
    #[error("unknown principal kind {0}")]
    BadKind(u8),
    #[error("announce does not match pinned trust")]
    Untrusted,
    #[error("role/kind mismatch")]
    RoleMismatch,
    #[error("invalid public key")]
    BadPublicKey,
    #[error("signature failed")]
    BadSignature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announce {
    pub kind: u8,
    pub device_pk: [u8; PK_LEN],
    pub principal_pk: [u8; PK_LEN],
}

impl Announce {
    pub fn encode(&self) -> [u8; ANNOUNCE_LEN] {
        let mut out = [0u8; ANNOUNCE_LEN];
        out[0] = self.kind;
        out[1..33].copy_from_slice(&self.device_pk);
        out[33..65].copy_from_slice(&self.principal_pk);
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, IdentityError> {
        if buf.len() != ANNOUNCE_LEN {
            return Err(IdentityError::BadAnnounce(buf.len()));
        }
        let kind = buf[0];
        if kind != KIND_HUMAN && kind != KIND_PEER {
            return Err(IdentityError::BadKind(kind));
        }
        Ok(Self {
            kind,
            device_pk: buf[1..33].try_into().expect("pk"),
            principal_pk: buf[33..65].try_into().expect("pk"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub nonce: [u8; NONCE_LEN],
}

impl Challenge {
    pub fn fresh() -> Self {
        let mut nonce = [0u8; NONCE_LEN];
        rand::RngCore::fill_bytes(&mut OsRng, &mut nonce);
        Self { nonce }
    }

    pub fn encode(&self) -> [u8; NONCE_LEN] {
        self.nonce
    }

    pub fn decode(buf: &[u8]) -> Result<Self, IdentityError> {
        if buf.len() != NONCE_LEN {
            return Err(IdentityError::BadChallenge(buf.len()));
        }
        Ok(Self {
            nonce: buf.try_into().expect("nonce"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    pub device_sig: [u8; SIG_LEN],
    pub principal_sig: [u8; SIG_LEN],
}

impl Proof {
    pub fn encode(&self) -> [u8; PROOF_LEN] {
        let mut out = [0u8; PROOF_LEN];
        out[..SIG_LEN].copy_from_slice(&self.device_sig);
        out[SIG_LEN..].copy_from_slice(&self.principal_sig);
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, IdentityError> {
        if buf.len() != PROOF_LEN {
            return Err(IdentityError::BadProof(buf.len()));
        }
        Ok(Self {
            device_sig: buf[..SIG_LEN].try_into().expect("sig"),
            principal_sig: buf[SIG_LEN..].try_into().expect("sig"),
        })
    }
}

pub fn transcript(
    client_announce: &Announce,
    server_announce: &Announce,
    client_nonce: &[u8; NONCE_LEN],
    server_nonce: &[u8; NONCE_LEN],
    exporter: &[u8; EXPORTER_LEN],
) -> Vec<u8> {
    let mut t =
        Vec::with_capacity(DOMAIN.len() + 1 + ANNOUNCE_LEN * 2 + NONCE_LEN * 2 + EXPORTER_LEN);
    t.extend_from_slice(DOMAIN);
    t.push(0);
    t.extend_from_slice(&client_announce.encode());
    t.extend_from_slice(&server_announce.encode());
    t.extend_from_slice(client_nonce);
    t.extend_from_slice(server_nonce);
    t.extend_from_slice(exporter);
    t
}

pub struct IdentityKeys {
    pub kind: u8,
    device: SigningKey,
    principal: SigningKey,
}

impl IdentityKeys {
    pub fn generate(kind: u8) -> Result<Self, IdentityError> {
        if kind != KIND_HUMAN && kind != KIND_PEER {
            return Err(IdentityError::BadKind(kind));
        }
        Ok(Self {
            kind,
            device: SigningKey::generate(&mut OsRng),
            principal: SigningKey::generate(&mut OsRng),
        })
    }

    pub fn announce(&self) -> Announce {
        Announce {
            kind: self.kind,
            device_pk: self.device.verifying_key().to_bytes(),
            principal_pk: self.principal.verifying_key().to_bytes(),
        }
    }

    pub fn prove(&self, transcript: &[u8]) -> Proof {
        let device_sig = self.device.sign(transcript).to_bytes();
        let principal_sig = self.principal.sign(transcript).to_bytes();
        Proof {
            device_sig,
            principal_sig,
        }
    }
}

pub fn verify_proof(
    announce: &Announce,
    trust: &Announce,
    expected_kind: u8,
    transcript: &[u8],
    proof: &Proof,
) -> Result<(), IdentityError> {
    if announce.kind != expected_kind {
        return Err(IdentityError::RoleMismatch);
    }
    if announce != trust {
        return Err(IdentityError::Untrusted);
    }
    let device =
        VerifyingKey::from_bytes(&announce.device_pk).map_err(|_| IdentityError::BadPublicKey)?;
    let principal = VerifyingKey::from_bytes(&announce.principal_pk)
        .map_err(|_| IdentityError::BadPublicKey)?;
    let ds = Signature::from_bytes(&proof.device_sig);
    let ps = Signature::from_bytes(&proof.principal_sig);
    device
        .verify(transcript, &ds)
        .map_err(|_| IdentityError::BadSignature)?;
    principal
        .verify(transcript, &ps)
        .map_err(|_| IdentityError::BadSignature)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announce_round_trip() {
        let keys = IdentityKeys::generate(KIND_HUMAN).unwrap();
        let a = keys.announce();
        assert_eq!(Announce::decode(&a.encode()).unwrap(), a);
    }

    #[test]
    fn proof_verifies_with_pin() {
        let client = IdentityKeys::generate(KIND_HUMAN).unwrap();
        let server = IdentityKeys::generate(KIND_PEER).unwrap();
        let c_ann = client.announce();
        let s_ann = server.announce();
        let c_n = Challenge::fresh().nonce;
        let s_n = Challenge::fresh().nonce;
        let exp = [7u8; 32];
        let t = transcript(&c_ann, &s_ann, &c_n, &s_n, &exp);
        let proof = client.prove(&t);
        verify_proof(&c_ann, &c_ann, KIND_HUMAN, &t, &proof).unwrap();
    }

    #[test]
    fn proof_fails_if_unpinned() {
        let client = IdentityKeys::generate(KIND_HUMAN).unwrap();
        let other = IdentityKeys::generate(KIND_HUMAN).unwrap();
        let t = transcript(
            &client.announce(),
            &IdentityKeys::generate(KIND_PEER).unwrap().announce(),
            &[1; 32],
            &[2; 32],
            &[3; 32],
        );
        let proof = client.prove(&t);
        let err = verify_proof(
            &client.announce(),
            &other.announce(),
            KIND_HUMAN,
            &t,
            &proof,
        )
        .unwrap_err();
        assert_eq!(err, IdentityError::Untrusted);
    }

    #[test]
    fn decode_rejects_short_announce() {
        assert!(matches!(
            Announce::decode(&[0; 8]),
            Err(IdentityError::BadAnnounce(8))
        ));
    }
}
