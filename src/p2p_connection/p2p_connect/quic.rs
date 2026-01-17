use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use quinn::{Endpoint, EndpointConfig, ServerConfig};

use crate::p2p_connect::autotune::AutotuneConfig;
use crate::p2p_connect::stun;

pub type StunOutcome = Result<Option<SocketAddr>, String>;

#[derive(Clone, Debug)]
pub struct EndpointOptions {
    /// QUIC stream/connection flow-control target window (bytes).
    pub target_window: u64,

    /// Enable/disable and tune flow-control clamping behavior.
    pub autotune: AutotuneConfig,

    /// When the requested bind port is already in use, bind to port 0 instead.
    pub fallback_to_ephemeral_on_addr_in_use: bool,

    /// Whether to run STUN discovery during bind.
    pub enable_stun_discovery: bool,

    /// If true, will always stream STUN trace lines to the logger callback.
    /// If false, trace lines are only emitted when STUN fails.
    pub stun_verbose_trace: bool,
}

impl Default for EndpointOptions {
    fn default() -> Self {
        Self {
            target_window: 512 * 1024,
            autotune: AutotuneConfig::default(),
            fallback_to_ephemeral_on_addr_in_use: true,
            enable_stun_discovery: true,
            stun_verbose_trace: false,
        }
    }
}

#[derive(Debug)]
pub struct EndpointBuild {
    pub endpoint: Endpoint,
    /// DER-encoded certificate used by the embedded QUIC TLS config.
    pub cert_der: Vec<u8>,
    /// Result of STUN discovery (public endpoint), if enabled.
    pub stun: StunOutcome,
    /// Local socket address actually bound by the OS.
    pub local_addr: SocketAddr,
}

#[derive(Debug)]
pub enum EndpointBuildError {
    Io(io::Error),
    Quinn(quinn::EndpointError),
    Other(Box<dyn Error + Send + Sync>),
}

impl std::fmt::Display for EndpointBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Quinn(e) => write!(f, "quinn endpoint error: {e}"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}
impl Error for EndpointBuildError {}

impl From<io::Error> for EndpointBuildError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<quinn::EndpointError> for EndpointBuildError {
    fn from(value: quinn::EndpointError) -> Self {
        Self::Quinn(value)
    }
}

/// Build a QUIC endpoint bound to `bind_addr`, including server + default client config.
///
/// This is intentionally "connection-only" logic:
/// - binds UDP + configures QUIC/TLS
/// - (optional) performs STUN discovery to learn the public-mapped endpoint
/// It does NOT open streams or implement any application protocol.
pub fn make_endpoint(
    bind_addr: SocketAddr,
    opts: &EndpointOptions,
    mut log: Option<&mut dyn FnMut(String)>,
) -> Result<EndpointBuild, EndpointBuildError> {
    let cert = rcgen::generate_simple_self_signed(["p2p.local".into()])
        .map_err(|e| EndpointBuildError::Other(Box::new(e)))?;
    let cert_der = cert
        .serialize_der()
        .map_err(|e| EndpointBuildError::Other(Box::new(e)))?;
    let priv_key = cert.serialize_private_key_der();

    let transport = make_transport_config(opts.target_window, &opts.autotune);

    let server_config = make_server_config(cert_der.clone(), priv_key, transport.clone())
        .map_err(|e| EndpointBuildError::Other(e))?;
    let client_config = make_client_config(transport);

    let socket = match std::net::UdpSocket::bind(bind_addr) {
        Ok(socket) => socket,
        Err(err)
            if opts.fallback_to_ephemeral_on_addr_in_use
                && err.kind() == io::ErrorKind::AddrInUse
                && bind_addr.port() != 0 =>
        {
            let fallback = SocketAddr::new(bind_addr.ip(), 0);
            std::net::UdpSocket::bind(fallback)?
        }
        Err(err) => return Err(EndpointBuildError::Io(err)),
    };

    let local_addr = socket.local_addr()?;

    // STUN discovery (optional)
    let stun_result = if !opts.enable_stun_discovery {
        Ok(None)
    } else if opts.stun_verbose_trace {
        stun::detect_public_endpoint_on_socket_with_trace(&socket, local_addr, |line| {
            if let Some(cb) = log.as_deref_mut() {
                cb(line);
            }
        })
    } else {
        let mut trace_lines: Vec<String> = Vec::new();
        let result = stun::detect_public_endpoint_on_socket_with_trace(&socket, local_addr, |line| {
            trace_lines.push(line);
        });
        if result.is_err() {
            if let Some(cb) = log.as_deref_mut() {
                for line in trace_lines {
                    cb(line);
                }
            }
        }
        result
    };

    // Quinn reads directly from the UDP socket.
    let _ = socket.set_read_timeout(None);

    let mut endpoint = Endpoint::new(
        EndpointConfig::default(),
        Some(server_config),
        socket,
        Arc::new(quinn::TokioRuntime),
    )?;
    endpoint.set_default_client_config(client_config);

    Ok(EndpointBuild {
        endpoint,
        cert_der,
        stun: stun_result,
        local_addr,
    })
}

fn make_transport_config(target_window: u64, autotune: &AutotuneConfig) -> quinn::TransportConfig {
    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(Duration::from_secs(5)));
    if let Ok(timeout) = quinn::IdleTimeout::try_from(Duration::from_secs(60)) {
        transport.max_idle_timeout(Some(timeout));
    }

    configure_flow_control(&mut transport, target_window, autotune);

    // Conservative defaults for mixed networks.
    transport.datagram_receive_buffer_size(Some(2 * 1024 * 1024));
    transport.max_concurrent_uni_streams(quinn::VarInt::from_u32(256));
    transport.max_concurrent_bidi_streams(quinn::VarInt::from_u32(64));

    transport
}

fn configure_flow_control(
    transport: &mut quinn::TransportConfig,
    target_window: u64,
    autotune: &AutotuneConfig,
) {
    let clamped = autotune.clamp_target(target_window);
    let flow_window = clamped.min(quinn::VarInt::MAX.into_inner());
    let flow_window =
        quinn::VarInt::from_u64(flow_window).expect("flow control window within QUIC varint bounds");
    transport.stream_receive_window(flow_window);
    transport.receive_window(flow_window);
    transport.send_window(clamped);
}

fn make_server_config(
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    transport: quinn::TransportConfig,
) -> Result<ServerConfig, Box<dyn Error + Send + Sync>> {
    use quinn::rustls;

    let cert = rustls::pki_types::CertificateDer::from(cert_der);
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(key_der.into());

    let mut crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;
    crypto.alpn_protocols = vec![b"p2p/1".to_vec()];

    let mut server_config = ServerConfig::with_crypto(Arc::new(crypto));
    server_config.transport_config(Arc::new(transport));
    Ok(server_config)
}

fn make_client_config(mut transport: quinn::TransportConfig) -> quinn::ClientConfig {
    use quinn::rustls::{self, client::danger, DigitallySignedStruct, SignatureScheme};

    #[derive(Debug)]
    struct SkipVerifier;

    impl danger::ServerCertVerifier for SkipVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::pki_types::CertificateDer<'_>,
            _intermediates: &[rustls::pki_types::CertificateDer<'_>],
            _server_name: &rustls::pki_types::ServerName<'_>,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<danger::ServerCertVerified, rustls::Error> {
            Ok(danger::ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
            Ok(danger::HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
            Ok(danger::HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ED25519,
            ]
        }
    }

    // We only need encryption (preventing trivial on-path snooping); authentication
    // is handled at the application layer in this project.
    let mut roots = rustls::RootCertStore::empty();
    let mut tls = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipVerifier))
        .with_no_client_auth();

    tls.enable_sni = false;
    tls.alpn_protocols = vec![b"p2p/1".to_vec()];

    transport
        .max_idle_timeout(Some(quinn::IdleTimeout::try_from(Duration::from_secs(60)).unwrap()));
    let mut client_config = quinn::ClientConfig::new(Arc::new(tls));
    client_config.transport_config(Arc::new(transport));
    client_config
}
