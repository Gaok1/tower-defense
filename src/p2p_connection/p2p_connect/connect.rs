use std::time::Duration;

use quinn::{Connection, Endpoint};
use tokio::time::timeout;

#[derive(Clone, Debug)]
pub struct ConnectOptions {
    /// QUIC SNI/server_name passed to `Endpoint::connect`.
    pub server_name: &'static str,
    pub timeout: Duration,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            server_name: "p2p.local",
            timeout: Duration::from_secs(4),
        }
    }
}

#[derive(Debug)]
pub enum ConnectError {
    Start(quinn::ConnectError),
    Timeout,
    Failed(quinn::ConnectionError),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start(e) => write!(f, "erro ao iniciar conexão: {e}"),
            Self::Timeout => write!(f, "timeout ao conectar"),
            Self::Failed(e) => write!(f, "falha ao conectar: {e}"),
        }
    }
}
impl std::error::Error for ConnectError {}

pub async fn connect_peer(
    endpoint: &Endpoint,
    peer: std::net::SocketAddr,
    opts: &ConnectOptions,
) -> Result<Connection, ConnectError> {
    let connecting = endpoint
        .connect(peer, opts.server_name)
        .map_err(ConnectError::Start)?;

    match timeout(opts.timeout, connecting).await {
        Ok(Ok(conn)) => Ok(conn),
        Ok(Err(err)) => Err(ConnectError::Failed(err)),
        Err(_) => Err(ConnectError::Timeout),
    }
}

#[derive(Clone, Debug)]
pub struct ProbeOptions {
    pub server_name: &'static str,
    pub timeout: Duration,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            server_name: "p2p.local",
            timeout: Duration::from_secs(2),
        }
    }
}

#[derive(Debug)]
pub enum ProbeError {
    Start(quinn::ConnectError),
    Timeout,
    Failed(quinn::ConnectionError),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start(e) => write!(f, "erro ao iniciar teste: {e}"),
            Self::Timeout => write!(f, "timeout no teste"),
            Self::Failed(e) => write!(f, "falha no teste: {e}"),
        }
    }
}
impl std::error::Error for ProbeError {}

pub async fn quick_probe_peer(
    endpoint: &Endpoint,
    peer: std::net::SocketAddr,
    opts: &ProbeOptions,
) -> Result<Duration, ProbeError> {
    let started = std::time::Instant::now();
    let connecting = endpoint
        .connect(peer, opts.server_name)
        .map_err(ProbeError::Start)?;

    match timeout(opts.timeout, connecting).await {
        Ok(Ok(conn)) => {
            // We only care about handshake RTT; drop immediately.
            conn.close(0u32.into(), b"probe");
            Ok(started.elapsed())
        }
        Ok(Err(err)) => Err(ProbeError::Failed(err)),
        Err(_) => Err(ProbeError::Timeout),
    }
}
