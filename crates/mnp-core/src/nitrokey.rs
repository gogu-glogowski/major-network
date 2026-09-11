//! Nitrokey 3A Mini identity backend — not wired.
//!
//! Architecture v1: token is proof-of-possession for HumanIdentity, not bulk
//! QUIC crypto. Concrete API (PIV / OpenPGP / FIDO2) is chosen only after
//! checking current 3A Mini firmware. This module refuses rather than faking
//! a signature.

pub use crate::identity::{IdentityError, Nitrokey3AMini};
