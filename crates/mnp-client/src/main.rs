use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use mnp_core::identity::{Announce, IdentityKeys, KIND_HUMAN};
use mnp_core::lab::{
    LabCert, client_endpoint, client_identity, send_hello, send_observer_report, tls_exporter,
};
use mnp_core::observer::Snapshot;
use mnp_core::session::Session;
use mnp_core::{LAB_LISTEN, LAB_SERVER_NAME};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().expect("info directive")),
        )
        .init();

    let connect: SocketAddr = arg(1)
        .unwrap_or_else(|| LAB_LISTEN.to_string())
        .parse()
        .context("connect address")?;
    let cert_path = PathBuf::from(arg(2).unwrap_or_else(|| "mnp-lab-cert.der".into()));
    let id_out = PathBuf::from(arg(3).unwrap_or_else(|| "mnp-lab-client.id".into()));
    let trust_path = arg(4).map(PathBuf::from);

    let cert = LabCert::read_der(&cert_path)
        .with_context(|| format!("pin lab cert {}", cert_path.display()))?;
    let keys = IdentityKeys::generate(KIND_HUMAN)?;
    std::fs::write(&id_out, keys.announce().encode())?;
    tracing::info!(id = %id_out.display(), "wrote client identity announce");

    let mut session = if trust_path.is_some() {
        Session::new_fail_closed()
    } else {
        tracing::warn!("no trust file (arg 4); LAB_AUTO_ACCEPT — not MNP identity");
        Session::new_lab()
    };

    tracing::info!(addr = %connect, cert = %cert_path.display(), "mnp-client connecting");
    let endpoint = client_endpoint(&cert)?;
    let conn = endpoint
        .connect(connect, LAB_SERVER_NAME)
        .context("QUIC connect")?
        .await
        .context("QUIC handshake")?;
    session.on_quic_connected()?;
    let (mut send, mut recv) = send_hello(&conn).await?;
    session.on_hello_ok()?;
    if let Some(path) = trust_path {
        let trust = Announce::decode(&std::fs::read(&path)?)?;
        let exp = tls_exporter(&conn)?;
        client_identity(&mut send, &mut recv, &keys, &trust, &exp).await?;
        session.on_identity_ok()?;
        send_observer_report(&conn, &Snapshot::from_host()).await?;
    }
    tracing::info!(state = ?session.state(), "client done");
    endpoint.wait_idle().await;
    Ok(())
}

fn arg(n: usize) -> Option<String> {
    std::env::args().nth(n)
}
