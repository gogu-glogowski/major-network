use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use mnp_core::lab::{LabCert, client_endpoint, send_hello};
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

    let cert = LabCert::read_der(&cert_path)
        .with_context(|| format!("pin lab cert {}", cert_path.display()))?;
    tracing::info!(addr = %connect, cert = %cert_path.display(), "mnp-client connecting");

    let endpoint = client_endpoint(&cert)?;
    let conn = endpoint
        .connect(connect, LAB_SERVER_NAME)
        .context("QUIC connect")?
        .await
        .context("QUIC handshake")?;
    send_hello(&conn).await?;
    tracing::info!("HELLO spike ok");
    endpoint.wait_idle().await;
    Ok(())
}

fn arg(n: usize) -> Option<String> {
    std::env::args().nth(n)
}
