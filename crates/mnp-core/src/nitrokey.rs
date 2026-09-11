//! Nitrokey 3A Mini as HumanIdentity via OpenPGP (GnuPG).
//!
//! Private key stays on the token. `prove()` asks GPG to detach-sign the MNP
//! transcript; pinentry + touch happen there. Device key remains software.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

use crate::identity::{Announce, IdentityBackend, IdentityError, KIND_HUMAN, PK_LEN, Proof};

const GPG_USER: &str = "major@local";

#[derive(Debug)]
pub struct Nitrokey3AMini {
    device: SigningKey,
    human_pk: [u8; PK_LEN],
}

impl Nitrokey3AMini {
    /// Requires the OpenPGP card visible to GPG. Device seed is software, persisted at `device_seed`.
    pub fn connect(device_seed: &Path) -> Result<Self, IdentityError> {
        let status = gpg_out(&["--card-status"])?;
        if !status.contains("Nitrokey") {
            return Err(IdentityError::HardwareUnavailable(
                "gpg --card-status is not a Nitrokey".into(),
            ));
        }
        let exported = gpg_bytes(&["--export", "--export-options", "export-minimal", GPG_USER])?;
        if exported.is_empty() {
            return Err(IdentityError::HardwareUnavailable(
                "no OpenPGP public key for major@local".into(),
            ));
        }
        let packets = gpg_out_bytes(&["--list-packets", "--verbose", "-"], &exported)?;
        let human_pk = first_ed25519_pk(&packets)?;
        let device = load_or_create_device(device_seed)?;
        Ok(Self { device, human_pk })
    }

    fn gpg_sign(&self, transcript: &[u8]) -> Result<Vec<u8>, IdentityError> {
        let mut child = Command::new("gpg")
            .args([
                "--batch",
                "--yes",
                "--detach-sign",
                "--output",
                "-",
                "-u",
                GPG_USER,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| IdentityError::Gpg(e.to_string()))?;
        child
            .stdin
            .as_mut()
            .ok_or_else(|| IdentityError::Gpg("stdin".into()))?
            .write_all(transcript)
            .map_err(|e| IdentityError::Gpg(e.to_string()))?;
        let out = child
            .wait_with_output()
            .map_err(|e| IdentityError::Gpg(e.to_string()))?;
        if !out.status.success() {
            return Err(IdentityError::Gpg(
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            ));
        }
        if out.stdout.is_empty() {
            return Err(IdentityError::Gpg("empty signature".into()));
        }
        Ok(out.stdout)
    }
}

impl IdentityBackend for Nitrokey3AMini {
    fn kind(&self) -> u8 {
        KIND_HUMAN
    }

    fn announce(&self) -> Announce {
        Announce {
            kind: KIND_HUMAN,
            device_pk: self.device.verifying_key().to_bytes(),
            principal_pk: self.human_pk,
        }
    }

    fn prove(&self, transcript: &[u8]) -> Result<Proof, IdentityError> {
        let device_sig = self.device.sign(transcript).to_bytes();
        let principal_sig = self.gpg_sign(transcript)?;
        Ok(Proof {
            device_sig,
            principal_sig,
        })
    }
}

pub fn verify_openpgp(
    transcript: &[u8],
    sig: &[u8],
    expected_pk: &[u8; PK_LEN],
) -> Result<(), IdentityError> {
    let dir = tempfile_dir()?;
    let data_path = dir.join("mnp-transcript");
    let sig_path = dir.join("mnp-transcript.sig");
    std::fs::write(&data_path, transcript).map_err(|e| IdentityError::Gpg(e.to_string()))?;
    std::fs::write(&sig_path, sig).map_err(|e| IdentityError::Gpg(e.to_string()))?;
    let child = Command::new("gpg")
        .args([
            "--batch",
            "--status-fd",
            "1",
            "--verify",
            sig_path.to_str().unwrap(),
            data_path.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| IdentityError::Gpg(e.to_string()))?;
    let out = child
        .wait_with_output()
        .map_err(|e| IdentityError::Gpg(e.to_string()))?;
    let _ = std::fs::remove_dir_all(&dir);
    let status = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !status.contains("VALIDSIG") {
        return Err(IdentityError::BadSignature);
    }
    let fpr = status
        .lines()
        .find_map(|l| {
            l.strip_prefix("[GNUPG:] VALIDSIG ")
                .and_then(|r| r.split_whitespace().next())
        })
        .ok_or(IdentityError::BadSignature)?;
    let exported = gpg_bytes(&["--export", "--export-options", "export-minimal", fpr])?;
    let packets = gpg_out_bytes(&["--list-packets", "--verbose", "-"], &exported)?;
    let pk = first_ed25519_pk(&packets)?;
    if &pk != expected_pk {
        return Err(IdentityError::Untrusted);
    }
    Ok(())
}

fn load_or_create_device(path: &Path) -> Result<SigningKey, IdentityError> {
    if path.exists() {
        let raw = std::fs::read(path).map_err(|e| IdentityError::Gpg(e.to_string()))?;
        if raw.len() != 32 {
            return Err(IdentityError::BadPublicKey);
        }
        let arr: [u8; 32] = raw.try_into().expect("32");
        return Ok(SigningKey::from_bytes(&arr));
    }
    let sk = SigningKey::generate(&mut OsRng);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, sk.to_bytes()).map_err(|e| IdentityError::Gpg(e.to_string()))?;
    Ok(sk)
}

fn first_ed25519_pk(list_packets: &str) -> Result<[u8; PK_LEN], IdentityError> {
    let mut in_primary = false;
    for line in list_packets.lines() {
        if line.contains(":public key packet:") {
            in_primary = true;
            continue;
        }
        if in_primary && line.trim_start().starts_with("pkey[1]:") {
            let hex = line
                .split_whitespace()
                .nth(1)
                .ok_or(IdentityError::BadPublicKey)?;
            let bytes = unhex(hex).map_err(|_| IdentityError::BadPublicKey)?;
            if bytes.len() == 33 && bytes[0] == 0x40 {
                return bytes[1..]
                    .try_into()
                    .map_err(|_| IdentityError::BadPublicKey);
            }
            return Err(IdentityError::BadPublicKey);
        }
        if in_primary && line.starts_with(':') {
            break;
        }
    }
    Err(IdentityError::HardwareUnavailable(
        "no Ed25519 public key packet".into(),
    ))
}

fn gpg_out(args: &[&str]) -> Result<String, IdentityError> {
    let out = Command::new("gpg")
        .args(args)
        .output()
        .map_err(|e| IdentityError::Gpg(e.to_string()))?;
    if !out.status.success() {
        return Err(IdentityError::HardwareUnavailable(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn gpg_bytes(args: &[&str]) -> Result<Vec<u8>, IdentityError> {
    let out = Command::new("gpg")
        .args(args)
        .output()
        .map_err(|e| IdentityError::Gpg(e.to_string()))?;
    if !out.status.success() {
        return Err(IdentityError::Gpg(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(out.stdout)
}

fn gpg_out_bytes(args: &[&str], stdin: &[u8]) -> Result<String, IdentityError> {
    let mut child = Command::new("gpg")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| IdentityError::Gpg(e.to_string()))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| IdentityError::Gpg("stdin".into()))?
        .write_all(stdin)
        .map_err(|e| IdentityError::Gpg(e.to_string()))?;
    let out = child
        .wait_with_output()
        .map_err(|e| IdentityError::Gpg(e.to_string()))?;
    if !out.status.success() {
        return Err(IdentityError::Gpg(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn tempfile_dir() -> Result<std::path::PathBuf, IdentityError> {
    let dir = std::env::temp_dir().join(format!("mnp-gpg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| IdentityError::Gpg(e.to_string()))?;
    Ok(dir)
}

fn unhex(s: &str) -> Result<Vec<u8>, ()> {
    if !s.len().is_multiple_of(2) {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{IdentityKeys, SIG_LEN};

    #[test]
    fn parse_ed25519_from_exported_pubkey() {
        let path = dirs_pub();
        if !path.exists() {
            return;
        }
        let raw = std::fs::read(&path).unwrap();
        // gpg --list-packets wants binary; file is armored
        let packets = gpg_out(&["--list-packets", "--verbose", path.to_str().unwrap()]).unwrap();
        let pk = first_ed25519_pk(&packets).unwrap();
        assert_ne!(pk, [0u8; 32]);
        let _ = raw;
    }

    fn dirs_pub() -> std::path::PathBuf {
        Path::new(&std::env::var("HOME").unwrap_or_else(|_| ".".into())).join("major-nitrokey.pub")
    }

    #[test]
    fn software_keys_still_impl_backend() {
        let k = IdentityKeys::generate(KIND_HUMAN).unwrap();
        assert_eq!(k.kind(), KIND_HUMAN);
        assert_eq!(
            IdentityKeys::prove(&k, &[1, 2, 3]).principal_sig.len(),
            SIG_LEN
        );
    }
}
