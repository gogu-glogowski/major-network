use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use mnp_core::LAB_LISTEN;
use mnp_core::lab::{LabCert, accept_hello, reject_v4_mapped, server_endpoint};

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

    let cert = LabCert::mint().context("mint throwaway lab cert")?;
    cert.write_der(&cert_path)
        .with_context(|| format!("write {}", cert_path.display()))?;
    tracing::info!(
        bind = %bind,
        cert = %cert_path.display(),
        "mnp-server lab TLS cert written (pin this from the client; not MNP identity)"
    );

    let endpoint = server_endpoint(bind, &cert)?;
    tracing::info!(local = %endpoint.local_addr()?, "listening IPv6-only");

    while let Some(incoming) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Err(err) = handle(incoming).await {
                tracing::error!("connection: {err:#}");
            }
        });
    }
    Ok(())
}

async fn handle(incoming: quinn::Incoming) -> Result<()> {
    let conn = incoming.await.context("QUIC handshake")?;
    reject_v4_mapped(conn.remote_address())?;
    tracing::info!(peer = %conn.remote_address(), "accepted");
    accept_hello(&conn).await?;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), conn.closed()).await;
    Ok(())
}

fn arg(n: usize) -> Option<String> {
    std::env::args().nth(n)
}
