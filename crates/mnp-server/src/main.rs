use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use mnp_core::LAB_LISTEN;
use mnp_core::identity::{Announce, IdentityKeys, KIND_PEER};
use mnp_core::lab::{
    LabCert, accept_hello, reject_v4_mapped, server_endpoint, server_identity, tls_exporter,
};
use mnp_core::session::Session;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().expect("info directive")),
        )
        .init();

    let bind: SocketAddr = arg(1)
        .unwrap_or_else(|| LAB_LISTEN.to_string())
        .parse()
        .context("bind address")?;
    let cert_path = PathBuf::from(arg(2).unwrap_or_else(|| "mnp-lab-cert.der".into()));
    let id_out = PathBuf::from(arg(3).unwrap_or_else(|| "mnp-lab-server.id".into()));
    let trust_path = arg(4).map(PathBuf::from);

    let cert = LabCert::mint().context("mint throwaway lab cert")?;
    cert.write_der(&cert_path)
        .with_context(|| format!("write {}", cert_path.display()))?;
    let keys = Arc::new(IdentityKeys::generate(KIND_PEER)?);
    std::fs::write(&id_out, keys.announce().encode())?;
    tracing::info!(
        bind = %bind,
        cert = %cert_path.display(),
        id = %id_out.display(),
        "mnp-server lab cert + identity announce written"
    );

    let trust = match &trust_path {
        Some(path) => Some(Arc::new(Announce::decode(&std::fs::read(path)?)?)),
        None => {
            tracing::warn!("no trust file (arg 4); LAB_AUTO_ACCEPT — not MNP identity");
            None
        }
    };

    let endpoint = server_endpoint(bind, &cert)?;
    tracing::info!(local = %endpoint.local_addr()?, "listening IPv6-only");

    while let Some(incoming) = endpoint.accept().await {
        let keys = Arc::clone(&keys);
        let trust = trust.clone();
        tokio::spawn(async move {
            if let Err(err) = handle(incoming, keys, trust).await {
                tracing::error!("connection: {err:#}");
            }
        });
    }
    Ok(())
}

async fn handle(
    incoming: quinn::Incoming,
    keys: Arc<IdentityKeys>,
    trust: Option<Arc<Announce>>,
) -> Result<()> {
    let mut session = if trust.is_some() {
        Session::new_fail_closed()
    } else {
        Session::new_lab()
    };
    let conn = incoming.await.context("QUIC handshake")?;
    reject_v4_mapped(conn.remote_address())?;
    session.on_quic_connected()?;
    tracing::info!(peer = %conn.remote_address(), "accepted");
    let (mut send, mut recv) = accept_hello(&conn).await?;
    session.on_hello_ok()?;
    if let Some(trust) = trust {
        let exp = tls_exporter(&conn)?;
        server_identity(&mut send, &mut recv, &keys, &trust, &exp).await?;
        session.on_identity_ok()?;
    }
    tracing::info!(state = ?session.state(), "session ready");
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), conn.closed()).await;
    session.on_goodbye_or_loss();
    Ok(())
}

fn arg(n: usize) -> Option<String> {
    std::env::args().nth(n)
}
