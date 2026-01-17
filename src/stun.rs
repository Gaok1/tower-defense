use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const STUN_SERVERS_V4: [&str; 5] = [
    "stun.cloudflare.com:3478",
    "global.stun.twilio.com:3478",
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
];

const STUN_SERVERS_V6: [&str; 3] = [
    "stun.cloudflare.com:3478",
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
];

const STUN_TIMEOUT: Duration = Duration::from_secs(1);
const STUN_READ_TIMEOUT: Duration = Duration::from_millis(200);
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_SUCCESS: u16 = 0x0101;
const STUN_ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const STUN_ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

pub fn detect_public_endpoint(bind_addr: SocketAddr) -> Result<Option<SocketAddr>, String> {
    const OVERALL: Duration = Duration::from_secs(6);

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let res = detect_public_endpoint_inner(bind_addr);
        let _ = tx.send(res);
    });

    match rx.recv_timeout(OVERALL) {
        Ok(res) => res,
        Err(_) => Ok(None),
    }
}

fn detect_public_endpoint_inner(bind_addr: SocketAddr) -> Result<Option<SocketAddr>, String> {
    let socket =
        UdpSocket::bind(bind_addr).map_err(|err| format!("falha ao abrir UDP para STUN: {err}"))?;
    let _ = socket.set_read_timeout(Some(STUN_READ_TIMEOUT));
    detect_public_endpoint_on_socket(&socket, bind_addr)
}

fn detect_public_endpoint_on_socket(
    socket: &UdpSocket,
    bind_addr: SocketAddr,
) -> Result<Option<SocketAddr>, String> {
    let start = Instant::now();
    let servers = stun_server_list(bind_addr);
    if servers.is_empty() {
        return Err("STUN sem servidores".to_string());
    }

    let mut seed = txid_seed();
    let mut sent_requests: Vec<SentRequest> = Vec::new();
    let total_servers = servers.len();
    for server in servers {
        let server_addrs = match resolve_stun_server_addrs(server.as_str(), bind_addr) {
            Ok(addrs) => addrs,
            Err(_) => continue,
        };

        for server_addr in server_addrs {
            let txid = next_transaction_id(&mut seed);
            let request = build_stun_request(txid);
            if socket.send_to(&request, server_addr).is_ok() {
                sent_requests.push(SentRequest {
                    server: server.clone(),
                    addr: server_addr,
                    txid,
                });
                break;
            }
        }
    }

    if sent_requests.is_empty() {
        return Err("STUN sem endpoints alcançáveis".to_string());
    }

    let mut buf = [0u8; 1024];
    let mut recv_count = 0usize;
    while start.elapsed() < STUN_TIMEOUT {
        match socket.recv_from(&mut buf) {
            Ok((len, from)) => {
                recv_count += 1;
                if let Some(addr) = parse_stun_response(&buf[..len], &sent_requests, from) {
                    return Ok(Some(addr));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(err) if err.kind() == io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }

    if recv_count == 0 {
        return Ok(None);
    }

    Err(format!(
        "STUN sem resposta valida ({recv_count} recebidas / {total_servers} servidores)"
    ))
}

#[derive(Clone)]
struct SentRequest {
    server: String,
    addr: SocketAddr,
    txid: [u8; 12],
}

fn stun_server_list(bind_addr: SocketAddr) -> Vec<String> {
    match bind_addr {
        SocketAddr::V4(_) => STUN_SERVERS_V4.iter().map(|s| s.to_string()).collect(),
        SocketAddr::V6(_) => STUN_SERVERS_V6.iter().map(|s| s.to_string()).collect(),
    }
}

fn resolve_stun_server_addrs(server: &str, bind_addr: SocketAddr) -> io::Result<Vec<SocketAddr>> {
    let family = match bind_addr {
        SocketAddr::V4(_) => 4,
        SocketAddr::V6(_) => 6,
    };

    let addrs: Vec<SocketAddr> = server.to_socket_addrs()?.collect();
    let filtered = addrs
        .into_iter()
        .filter(|addr| match (family, addr) {
            (4, SocketAddr::V4(_)) => true,
            (6, SocketAddr::V6(_)) => true,
            _ => false,
        })
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "addr family mismatch",
        ))
    } else {
        Ok(filtered)
    }
}

fn parse_stun_response(
    payload: &[u8],
    sent: &[SentRequest],
    from: SocketAddr,
) -> Option<SocketAddr> {
    if payload.len() < 20 {
        return None;
    }

    let msg_type = u16::from_be_bytes([payload[0], payload[1]]);
    if msg_type != STUN_BINDING_SUCCESS {
        return None;
    }
    let msg_len = u16::from_be_bytes([payload[2], payload[3]]) as usize;
    let cookie = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
    if cookie != STUN_MAGIC_COOKIE {
        return None;
    }
    let txid = &payload[8..20];

    if !sent.iter().any(|s| s.txid == txid && s.addr == from) {
        return None;
    }

    let mut idx = 20usize;
    let total = 20 + msg_len.min(payload.len().saturating_sub(20));
    while idx + 4 <= total {
        let attr_type = u16::from_be_bytes([payload[idx], payload[idx + 1]]);
        let attr_len = u16::from_be_bytes([payload[idx + 2], payload[idx + 3]]) as usize;
        idx += 4;
        if idx + attr_len > payload.len() {
            break;
        }
        let value = &payload[idx..idx + attr_len];

        let mapped = match attr_type {
            STUN_ATTR_MAPPED_ADDRESS => parse_mapped_address(value),
            STUN_ATTR_XOR_MAPPED_ADDRESS => parse_xor_mapped_address(value, txid),
            _ => None,
        };
        if mapped.is_some() {
            return mapped;
        }

        let padded = (attr_len + 3) & !3;
        idx += padded;
    }
    None
}

fn parse_mapped_address(value: &[u8]) -> Option<SocketAddr> {
    if value.len() < 4 {
        return None;
    }
    let family = value[1];
    let port = u16::from_be_bytes([value[2], value[3]]);
    match family {
        0x01 if value.len() >= 8 => {
            let ip = Ipv4Addr::new(value[4], value[5], value[6], value[7]);
            Some(SocketAddr::new(IpAddr::V4(ip), port))
        }
        0x02 if value.len() >= 20 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&value[4..20]);
            Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port))
        }
        _ => None,
    }
}

fn parse_xor_mapped_address(value: &[u8], txid: &[u8]) -> Option<SocketAddr> {
    if value.len() < 4 {
        return None;
    }
    let family = value[1];
    let xor_port = u16::from_be_bytes([value[2], value[3]]) ^ (STUN_MAGIC_COOKIE >> 16) as u16;
    match family {
        0x01 if value.len() >= 8 => {
            let mut ip = [0u8; 4];
            ip.copy_from_slice(&value[4..8]);
            let cookie = STUN_MAGIC_COOKIE.to_be_bytes();
            for (b, c) in ip.iter_mut().zip(cookie.iter()) {
                *b ^= *c;
            }
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(ip)), xor_port))
        }
        0x02 if value.len() >= 20 => {
            let mut ip = [0u8; 16];
            ip.copy_from_slice(&value[4..20]);
            let mut xor_key = [0u8; 16];
            xor_key[..4].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
            xor_key[4..].copy_from_slice(txid);
            for (b, c) in ip.iter_mut().zip(xor_key.iter()) {
                *b ^= *c;
            }
            Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(ip)), xor_port))
        }
        _ => None,
    }
}

fn build_stun_request(txid: [u8; 12]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(20);
    buf.extend_from_slice(&STUN_BINDING_REQUEST.to_be_bytes());
    buf.extend_from_slice(&0u16.to_be_bytes());
    buf.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
    buf.extend_from_slice(&txid);
    buf
}

fn txid_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    now ^ 0xA5A5_5A5A
}

fn next_transaction_id(seed: &mut u64) -> [u8; 12] {
    let mut out = [0u8; 12];
    for chunk in out.chunks_exact_mut(4) {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        let v = (*seed as u32).to_be_bytes();
        chunk.copy_from_slice(&v);
    }
    out
}
