//! Laboratory QUIC/TLS helpers for the v0.0.1 HELLO spike.
//!
//! This is **not** MNP identity. The client pins a throwaway cert minted by
//! the server. Skip-server-verification is forbidden.

use std::net::{IpAddr, Ipv6Addr, SocketAddr, UdpSocket};
use std::path::Path;
use std::sync::Arc;
use std::sync::Once;

use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use thiserror::Error;

use crate::frame::{Frame, FrameError, HEADER_LEN, MessageType, decode_header, encode};
use crate::{LAB_ALPN, LAB_SERVER_NAME};

static CRYPTO: Once = Once::new();

/// Install the rustls `aws-lc-rs` provider once. Call before building TLS config.
pub fn install_crypto_provider() {
    CRYPTO.call_once(|| {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("rustls CryptoProvider (aws-lc-rs)");
    });
}

#[derive(Debug, Error)]
pub enum LabError {
    #[error("IPv4 is not allowed in MNP v1 lab")]
    Ipv4,
    #[error("IPv4-mapped IPv6 is not allowed in MNP v1 lab")]
    Ipv4Mapped,
    #[error("certificate: {0}")]
    Cert(String),
    #[error("tls: {0}")]
    Tls(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("quic connect: {0}")]
    Connect(#[from] quinn::ConnectError),
    #[error("quic connection: {0}")]
    Connection(#[from] quinn::ConnectionError),
    #[error("quic write: {0}")]
    Write(#[from] quinn::WriteError),
    #[error("quic stream closed: {0}")]
    ClosedStream(#[from] quinn::ClosedStream),
    #[error("quic read: {0}")]
    ReadClosed(String),
    #[error("expected {0:?}, got {1:?}")]
    UnexpectedType(MessageType, MessageType),
    #[error("frame: {0}")]
    Frame(#[from] FrameError),
    #[error("{0}")]
    Other(String),
}

/// Throwaway self-signed cert: SAN DNS localhost + IP ::1.
#[derive(Clone)]
pub struct LabCert {
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
}

impl LabCert {
    pub fn mint() -> Result<Self, LabError> {
        let mut params = rcgen::CertificateParams::new(vec![LAB_SERVER_NAME.to_string()])
            .map_err(|e| LabError::Cert(e.to_string()))?;
        params
            .subject_alt_names
            .push(rcgen::SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        let key_pair = rcgen::KeyPair::generate().map_err(|e| LabError::Cert(e.to_string()))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| LabError::Cert(e.to_string()))?;
        Ok(Self {
            cert_der: cert.der().to_vec(),
            key_der: key_pair.serialize_der(),
        })
    }

    pub fn certificate(&self) -> CertificateDer<'static> {
        CertificateDer::from(self.cert_der.clone())
    }

    fn private_key(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key_der.clone()))
    }

    pub fn write_der(&self, path: &Path) -> Result<(), LabError> {
        std::fs::write(path, &self.cert_der)?;
        Ok(())
    }

    pub fn read_der(path: &Path) -> Result<Self, LabError> {
        let cert_der = std::fs::read(path)?;
        Ok(Self {
            cert_der,
            key_der: Vec::new(),
        })
    }
}

pub fn reject_v4_mapped(addr: SocketAddr) -> Result<(), LabError> {
    match addr {
        SocketAddr::V4(_) => Err(LabError::Ipv4),
        SocketAddr::V6(v6) if v6.ip().to_ipv4_mapped().is_some() => Err(LabError::Ipv4Mapped),
        SocketAddr::V6(_) => Ok(()),
    }
}

/// Bind a UDP socket that is IPv6-only (`IPV6_V6ONLY`) and refuses IPv4-mapped.
pub fn bind_ipv6_only(addr: SocketAddr) -> Result<UdpSocket, LabError> {
    reject_v4_mapped(addr)?;
    let socket = socket2::Socket::new(
        socket2::Domain::IPV6,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )?;
    socket.set_only_v6(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&socket2::SockAddr::from(addr))?;
    Ok(socket.into())
}

fn server_tls(cert: &LabCert) -> Result<ServerConfig, LabError> {
    let mut cfg = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_no_client_auth()
        .with_single_cert(vec![cert.certificate()], cert.private_key())
        .map_err(|e| LabError::Tls(e.to_string()))?;
    cfg.alpn_protocols = vec![LAB_ALPN.to_vec()];
    cfg.max_early_data_size = 0;
    Ok(cfg)
}

fn client_tls(cert: &LabCert) -> Result<ClientConfig, LabError> {
    let mut roots = RootCertStore::empty();
    roots
        .add(cert.certificate())
        .map_err(|e| LabError::Tls(e.to_string()))?;
    let mut cfg = ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
        .with_root_certificates(roots)
        .with_no_client_auth();
    cfg.alpn_protocols = vec![LAB_ALPN.to_vec()];
    cfg.enable_early_data = false;
    Ok(cfg)
}

pub fn server_endpoint(bind: SocketAddr, cert: &LabCert) -> Result<quinn::Endpoint, LabError> {
    install_crypto_provider();
    let sock = bind_ipv6_only(bind)?;
    let crypto =
        QuicServerConfig::try_from(server_tls(cert)?).map_err(|e| LabError::Tls(e.to_string()))?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    let runtime = quinn::default_runtime()
        .ok_or_else(|| LabError::Other("quinn runtime requires a Tokio context".into()))?;
    Ok(quinn::Endpoint::new(
        quinn::EndpointConfig::default(),
        Some(server_config),
        sock,
        runtime,
    )?)
}

pub fn client_endpoint(cert: &LabCert) -> Result<quinn::Endpoint, LabError> {
    install_crypto_provider();
    let sock = bind_ipv6_only(SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)))?;
    let crypto =
        QuicClientConfig::try_from(client_tls(cert)?).map_err(|e| LabError::Tls(e.to_string()))?;
    let client_config = quinn::ClientConfig::new(Arc::new(crypto));
    let runtime = quinn::default_runtime()
        .ok_or_else(|| LabError::Other("quinn runtime requires a Tokio context".into()))?;
    let mut endpoint = quinn::Endpoint::new(quinn::EndpointConfig::default(), None, sock, runtime)?;
    endpoint.set_default_client_config(client_config);
    Ok(endpoint)
}

async fn read_exact(recv: &mut quinn::RecvStream, want: usize) -> Result<Vec<u8>, LabError> {
    let mut buf = vec![0u8; want];
    let mut filled = 0;
    while filled < want {
        match recv
            .read(&mut buf[filled..])
            .await
            .map_err(|e| LabError::ReadClosed(e.to_string()))?
        {
            Some(n) => filled += n,
            None => {
                buf.truncate(filled);
                return Err(LabError::ReadClosed(format!(
                    "stream finished after {filled} bytes, wanted {want}"
                )));
            }
        }
    }
    Ok(buf)
}

async fn write_frame(send: &mut quinn::SendStream, frame: &Frame) -> Result<(), LabError> {
    let bytes = encode(frame)?;
    send.write_all(&bytes).await?;
    Ok(())
}

async fn read_frame(recv: &mut quinn::RecvStream) -> Result<Frame, LabError> {
    let header_buf = read_exact(recv, HEADER_LEN).await?;
    let header = match decode_header(&header_buf) {
        Ok(h) => h,
        Err(e @ FrameError::PayloadTooLarge(_)) => {
            return Err(e.into());
        }
        Err(e) => return Err(e.into()),
    };
    let payload = if header.length == 0 {
        Vec::new()
    } else {
        read_exact(recv, header.length as usize).await?
    };
    Ok(Frame {
        ty: header.ty,
        flags: header.flags,
        session: header.session,
        payload,
    })
}

/// Client: one bi-di stream, HELLO frame, HELLO_ACK frame. Stream stays open (no finish).
pub async fn send_hello(conn: &quinn::Connection) -> Result<(), LabError> {
    reject_v4_mapped(conn.remote_address())?;
    let (mut send, mut recv) = conn.open_bi().await?;
    write_frame(&mut send, &Frame::empty(MessageType::Hello)).await?;
    let ack = read_frame(&mut recv).await?;
    if ack.ty != MessageType::HelloAck {
        return Err(LabError::UnexpectedType(MessageType::HelloAck, ack.ty));
    }
    tracing::info!("received HELLO_ACK frame");
    Ok(())
}

/// Server: one bi-di stream, read HELLO frame, write HELLO_ACK. Stream stays open.
pub async fn accept_hello(conn: &quinn::Connection) -> Result<(), LabError> {
    reject_v4_mapped(conn.remote_address())?;
    let (mut send, mut recv) = conn.accept_bi().await?;
    let hello = match read_frame(&mut recv).await {
        Err(e @ LabError::Frame(FrameError::PayloadTooLarge(_))) => {
            let _ = write_frame(&mut send, &Frame::empty(MessageType::ErrorFrame)).await;
            return Err(e);
        }
        other => other?,
    };
    if hello.ty != MessageType::Hello {
        return Err(LabError::UnexpectedType(MessageType::Hello, hello.ty));
    }
    tracing::info!("received HELLO frame");
    write_frame(&mut send, &Frame::empty(MessageType::HelloAck)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_rejects_ipv4() {
        let err = bind_ipv6_only("127.0.0.1:0".parse().unwrap()).unwrap_err();
        assert!(matches!(err, LabError::Ipv4));
    }

    #[tokio::test]
    async fn hello_ack_over_quic_ipv6() {
        install_crypto_provider();
        let cert = LabCert::mint().expect("mint lab cert");
        let server = server_endpoint("[::1]:0".parse().unwrap(), &cert).expect("server bind");
        let addr = server.local_addr().expect("local addr");
        assert!(addr.is_ipv6());
        reject_v4_mapped(addr).expect("listen addr is real IPv6");

        let server_task = tokio::spawn(async move {
            let incoming = server.accept().await.expect("incoming");
            let conn = incoming.await.expect("server handshake");
            let result = accept_hello(&conn).await;
            // Do not drop the QUIC connection before the client reads ACK.
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            result
        });

        let client = client_endpoint(&cert).expect("client bind");
        let conn = client
            .connect(addr, LAB_SERVER_NAME)
            .expect("start connect")
            .await
            .expect("client handshake");
        send_hello(&conn).await.expect("HELLO/ACK");
        server_task.await.expect("join").expect("server HELLO");
    }
}
