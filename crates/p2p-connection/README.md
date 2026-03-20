# p2p-connection

A Rust library for authenticated peer-to-peer connections over QUIC with automatic NAT traversal.

Built on top of [Quinn](https://github.com/quinn-rs/quinn), it handles the boring parts of P2P networking: identity generation, mutual authentication, NAT hole-punching via STUN, heartbeats, automatic reconnection, and flow-control tuning.

---

## Features

- **QUIC transport** — multiplexed, encrypted, low-latency UDP streams via Quinn
- **Mutual Ed25519 authentication** — challenge-response handshake; each peer has a persistent keypair stored on disk
- **NAT traversal** — automatic public endpoint discovery via STUN (Cloudflare, Google, Twilio, and others)
- **Heartbeat & timeout detection** — configurable interval and miss threshold
- **Automatic reconnection** — exponential backoff when a peer drops
- **Flow-control autotuning** — BDP-aware send window adjustment based on live path metrics
- **Peer whitelisting** — optionally restrict connections to a known public key
- **IPv4 and IPv6** support

---

## Quick start

```toml
[dependencies]
p2p-connection = { path = "..." }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use p2p_connection::{P2pConfig, P2pEvent, P2pNode};

#[tokio::main]
async fn main() {
    let config = P2pConfig {
        bind_addr: "0.0.0.0:0".parse().unwrap(),   // OS picks a free port
        connect_to: Some("1.2.3.4:9000".parse().unwrap()),
        ..P2pConfig::default()
    };

    let (node, mut events) = P2pNode::start(config).await.unwrap();

    while let Some(event) = events.recv().await {
        match event {
            P2pEvent::Bound(addr) => println!("Listening on {addr}"),
            P2pEvent::PublicEndpoint(addr) => println!("Public endpoint: {addr}"),

            P2pEvent::PeerVerified { peer, peer_id } => {
                println!("Connected to {peer_id} at {peer}");
                node.send_data(peer, b"hello!".to_vec());
            }

            P2pEvent::DataReceived { from, payload } => {
                println!("Got {} bytes from {from}", payload.len());
                node.shutdown();
                break;
            }

            P2pEvent::PeerDisconnected(addr) | P2pEvent::PeerTimeout(addr) => {
                println!("Lost connection to {addr}");
            }

            _ => {}
        }
    }
}
```

---

## Concepts

### Identity

On first run, the library generates a persistent Ed25519 keypair and saves it under `~/.config/p2p_connection/` (or the path you set in `P2pConfig::app_dir`). Every subsequent run reuses this keypair. The **peer ID** is a 16-byte hex string derived from the SHA-256 of the public key.

### Handshake

After a QUIC connection is established, both sides perform a mutual challenge-response handshake over a dedicated bidirectional stream:

1. Both peers exchange their Ed25519 public keys and random 32-byte challenges.
2. Each peer signs the *other's* challenge and sends the signature back.
3. Both peers verify the received signature.

Only after both sides verify successfully is `PeerVerified` emitted. Unauthenticated or mismatched peers receive `PeerAuthFailed` and are dropped.

### NAT traversal

During startup, the node sends STUN Binding Requests to several public servers and emits a `PublicEndpoint` event with the mapped address. Share this address with the other party (out of band) and use it as `connect_to`. The STUN server list can be overridden via the `PASTA_P2P_STUN` environment variable.

### Event-driven API

`P2pNode::start` returns a channel receiver. All state changes arrive as `P2pEvent` variants; you drive your application by matching on them in a loop. Commands (connect, send, disconnect) are sent through the `P2pNode` handle methods.

---

## API reference

### `P2pConfig`

```rust
pub struct P2pConfig {
    /// Local UDP address to bind. Use port 0 for OS-assigned port.
    pub bind_addr: SocketAddr,            // default: 0.0.0.0:0

    /// Dial this peer immediately on startup.
    pub connect_to: Option<SocketAddr>,   // default: None

    /// Only accept connections from a peer whose public key matches this
    /// base64-encoded key (or peer ID hex). If None, any peer is accepted.
    pub expected_peer_key: Option<String>, // default: None

    /// Directory for storing the Ed25519 identity keypair.
    /// Defaults to ~/.config/p2p_connection/ (platform-specific).
    pub app_dir: Option<PathBuf>,         // default: None

    /// QUIC flow-control autotuning settings.
    pub autotune: AutotuneConfig,

    /// Reconnection and endpoint-mobility settings.
    pub mobility: MobilityConfig,

    /// How often to send a heartbeat ping to each peer.
    pub heartbeat_interval: Duration,     // default: 2s

    /// How many consecutive missed pings before declaring a peer timed out.
    pub heartbeat_max_misses: u32,        // default: 3

    /// Maximum time to wait for the authentication handshake to complete.
    pub handshake_timeout: Duration,      // default: 3s

    /// Maximum time to wait for a QUIC connection to be established.
    pub connect_timeout: Duration,        // default: 6s
}
```

### `P2pNode`

| Method | Description |
|--------|-------------|
| `P2pNode::start(config) -> (P2pNode, Receiver<P2pEvent>)` | Initialize the node and start listening |
| `node.connect_peer(addr)` | Dial a peer by address |
| `node.send_data(to, payload)` | Send raw bytes to a verified peer |
| `node.broadcast_data(payload)` | Send to all currently verified peers |
| `node.probe_peer(addr)` | Run a QUIC round-trip probe; result arrives as `ProbeResult` |
| `node.disconnect_peer(addr)` | Disconnect a specific peer gracefully |
| `node.shutdown()` | Disconnect all peers and stop the node |

All methods return `bool` — `false` means the node has already shut down.

### `P2pEvent`

| Variant | When it fires |
|---------|--------------|
| `Bound(SocketAddr)` | Node is bound and ready to accept connections |
| `PublicEndpoint(SocketAddr)` | STUN discovered the public address |
| `ObservedEndpoint(SocketAddr)` | A peer reported the address they see us from |
| `PeerConnecting(SocketAddr)` | Outbound connection attempt started |
| `PeerConnected(SocketAddr)` | QUIC connection established (handshake not yet done) |
| `PeerVerified { peer, peer_id }` | Mutual authentication succeeded — safe to send data |
| `PeerAuthFailed { peer, reason }` | Handshake failed or peer rejected by whitelist |
| `PeerDisconnected(SocketAddr)` | Peer closed the connection cleanly |
| `PeerTimeout(SocketAddr)` | Peer stopped responding to heartbeats |
| `DataReceived { from, payload }` | Received application bytes from a verified peer |
| `ProbeResult { peer, ok, message }` | Result of a `probe_peer` call |
| `Log(String)` | Diagnostic message (informational) |

### `P2pCommand`

Low-level alternative to the convenience methods above, sent via `node.send_command(cmd)`:

```rust
pub enum P2pCommand {
    ConnectPeer(SocketAddr),
    ProbePeer(SocketAddr),
    Rebind(SocketAddr),
    SendData { to: SocketAddr, payload: Vec<u8> },
    BroadcastData(Vec<u8>),
    Disconnect(SocketAddr),
    Shutdown,
}
```

---

## Configuration reference

### `MobilityConfig`

Controls automatic reconnection and endpoint-change detection.

```rust
pub struct MobilityConfig {
    /// Enable mobility features (reconnect + observation).
    pub enabled: bool,               // default: true

    /// How often to inform the peer of our observed remote address.
    pub observe_interval: Duration,  // default: 10s

    /// Automatically reconnect when a peer drops.
    pub reconnect_enabled: bool,     // default: true

    /// Initial backoff before first reconnect attempt.
    pub reconnect_initial: Duration, // default: 500ms

    /// Maximum backoff between reconnect attempts.
    pub reconnect_max: Duration,     // default: 10s

    /// Rebind to a new local port after this many consecutive failures.
    /// 0 = never rebind.
    pub rebind_after_failures: u32,  // default: 0
}
```

### `AutotuneConfig`

Controls dynamic QUIC send-window sizing based on measured bandwidth and RTT.

```rust
pub struct AutotuneConfig {
    /// Enable flow-control autotuning.
    pub enabled: bool,                   // default: true

    /// Scaling factor for BDP → window size (higher = more aggressive).
    pub gain: f64,                       // default: 1.5

    /// Minimum send window size in bytes.
    pub min_window: u64,                 // default: 256 KB

    /// Maximum send window size in bytes.
    pub max_window: u64,                 // default: 256 MB

    /// How often to sample path metrics and adjust the window.
    pub sample_interval: Duration,       // default: 500ms

    /// Decay factor for the max delivery-rate estimate (0–1).
    pub rate_decay: f64,                 // default: 0.9
}
```

---

## Environment variables

| Variable | Effect |
|----------|--------|
| `PASTA_P2P_STUN` | Comma-separated list of STUN servers to use, e.g. `stun.example.com:3478,stun2.example.com:3478`. Set to an empty value or `0`/`false`/`no` to disable STUN entirely. |
| `PASTA_P2P_STUN_TRACE` | Set to any value to enable verbose STUN tracing (server resolution, packet sends/receives). Useful for debugging NAT issues. |

---

## Example

A full working example with two nodes exchanging messages on loopback is included:

```bash
cargo run --example ping_pong -p p2p-connection
```

Expected output (order may vary):

```
[server] bound to 127.0.0.1:19000
[client] bound to 127.0.0.1:<port>
[client] PeerVerified: <peer_id>
[server] PeerVerified: <peer_id>
[server] received: hello from client
[client] received: hello from server
done.
```

---

## Security notes

- **TLS certificates are self-signed.** The QUIC layer provides encryption and authenticity at the transport level, but peer identity is verified at the application level via the Ed25519 handshake. Do not rely on TLS certificate validation for security.
- **Keys are stored unencrypted** in the app directory. Protect that directory appropriately if key compromise is a concern.
- **Peer whitelisting** (`expected_peer_key`) is recommended for applications that should only talk to a known counterpart.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| [quinn](https://crates.io/crates/quinn) | QUIC protocol implementation |
| [tokio](https://crates.io/crates/tokio) | Async runtime |
| [ring](https://crates.io/crates/ring) | Ed25519 signing and verification |
| [rcgen](https://crates.io/crates/rcgen) | Self-signed TLS certificate generation |
| [serde](https://crates.io/crates/serde) + [bincode](https://crates.io/crates/bincode) | Wire message serialization |
| [base64](https://crates.io/crates/base64) | Public key encoding |
| [get_if_addrs](https://crates.io/crates/get_if_addrs) | Local network interface detection |

---

## License

MIT
