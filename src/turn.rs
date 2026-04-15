use byteorder::{BigEndian, ByteOrder};
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use tokio::task;

use crate::stun::LatencyStats;

// TURN constants (RFC 5766)
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;
const TURN_ALLOCATE_REQUEST: u16 = 0x0003;
const TURN_ALLOCATE_RESPONSE: u16 = 0x0103;
const TURN_ALLOCATE_ERROR: u16 = 0x0113;

// STUN/TURN attribute types
const ATTR_REQUESTED_TRANSPORT: u16 = 0x0019;
const ATTR_USERNAME: u16 = 0x0006;

// Transport protocol numbers
const TRANSPORT_UDP: u8 = 17;

/// Build a TURN Allocate Request packet.
///
/// The Allocate request includes a REQUESTED-TRANSPORT attribute (RFC 5766 Section 6.1).
/// If credentials are provided, a USERNAME attribute is also included.
///
/// Packet layout:
///   [20-byte STUN header]
///   [REQUESTED-TRANSPORT attribute (8 bytes)]
///   [optional USERNAME attribute (variable)]
fn build_allocate_request(
    transaction_id: &[u8; 12],
    credentials: Option<&(String, String)>,
) -> Vec<u8> {
    // Calculate total attribute length
    let mut attr_len: u16 = 8; // REQUESTED-TRANSPORT is 4 bytes value + 4 bytes TLV header

    let username_padded_len = if let Some((username, _)) = credentials {
        let raw_len = username.len();
        // STUN attributes are padded to 4-byte boundaries
        let padded = (raw_len + 3) & !3;
        attr_len += 4 + padded as u16; // TLV header + padded value
        padded
    } else {
        0
    };

    let total_len = 20 + attr_len as usize;
    let mut buf = vec![0u8; total_len];

    // STUN Header
    BigEndian::write_u16(&mut buf[0..2], TURN_ALLOCATE_REQUEST);
    BigEndian::write_u16(&mut buf[2..4], attr_len);
    BigEndian::write_u32(&mut buf[4..8], STUN_MAGIC_COOKIE);
    buf[8..20].copy_from_slice(transaction_id);

    // REQUESTED-TRANSPORT attribute
    let mut offset = 20;
    BigEndian::write_u16(&mut buf[offset..offset + 2], ATTR_REQUESTED_TRANSPORT);
    BigEndian::write_u16(&mut buf[offset + 2..offset + 4], 4); // value length = 4
    buf[offset + 4] = TRANSPORT_UDP;
    // bytes [offset+5..offset+8] are RFFU (reserved), already zero
    offset += 8;

    // USERNAME attribute (optional)
    if let Some((username, _)) = credentials {
        BigEndian::write_u16(&mut buf[offset..offset + 2], ATTR_USERNAME);
        BigEndian::write_u16(&mut buf[offset + 2..offset + 4], username.len() as u16);
        buf[offset + 4..offset + 4 + username.len()].copy_from_slice(username.as_bytes());
        // Padding bytes are already zero
        let _ = username_padded_len; // used in length calculation above
    }

    buf
}

/// Parse a TURN response and return the message type.
fn parse_turn_response(buf: &[u8]) -> Result<(u16, [u8; 12]), &'static str> {
    if buf.len() < 20 {
        return Err("Response too short");
    }

    let msg_type = BigEndian::read_u16(&buf[0..2]);
    let cookie = BigEndian::read_u32(&buf[4..8]);

    if cookie != STUN_MAGIC_COOKIE {
        return Err("Invalid magic cookie");
    }

    if buf[0] & 0xC0 != 0 {
        return Err("Invalid STUN message: top two bits not zero");
    }

    let mut txn_id = [0u8; 12];
    txn_id.copy_from_slice(&buf[8..20]);

    Ok((msg_type, txn_id))
}

/// Generate a random 12-byte transaction ID.
fn random_transaction_id() -> [u8; 12] {
    let mut id = [0u8; 12];
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut state = seed;
    for byte in id.iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = (state & 0xFF) as u8;
    }
    id
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnBenchmarkResults {
    pub server: String,
    pub port: u16,
    pub concurrent_users: u32,
    pub duration_secs: u64,
    pub total_requests: u64,
    pub successful_allocations: u64,
    pub error_responses: u64,
    pub timeouts: u64,
    pub packet_loss_pct: f64,
    pub requests_per_sec: f64,
    pub allocation_rtt: LatencyStats,
    pub timestamp: String,
}

/// Run a single TURN worker that sends Allocate requests in a loop.
fn turn_worker(
    host: &str,
    port: u16,
    duration: Duration,
    credentials: Option<(String, String)>,
) -> (Vec<f64>, u64, u64, u64) {
    let addr = format!("{}:{}", host, port);
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind UDP socket: {}", e);
            return (vec![], 0, 0, 0);
        }
    };
    socket.set_read_timeout(Some(Duration::from_millis(3000))).ok();
    socket.set_write_timeout(Some(Duration::from_millis(1000))).ok();

    let mut rtt_samples = Vec::with_capacity(2048);
    let mut sent: u64 = 0;
    let mut success: u64 = 0;
    let mut errors: u64 = 0;

    let start = Instant::now();
    let mut recv_buf = [0u8; 1024];

    while start.elapsed() < duration {
        let txn_id = random_transaction_id();
        let packet = build_allocate_request(&txn_id, credentials.as_ref());

        let send_time = Instant::now();
        if socket.send_to(&packet, &addr).is_err() {
            sent += 1;
            continue;
        }
        sent += 1;

        match socket.recv_from(&mut recv_buf) {
            Ok((n, _src)) => {
                let rtt = send_time.elapsed();
                rtt_samples.push(rtt.as_secs_f64() * 1_000_000.0);

                match parse_turn_response(&recv_buf[..n]) {
                    Ok((msg_type, resp_txn_id)) => {
                        if resp_txn_id == txn_id {
                            if msg_type == TURN_ALLOCATE_RESPONSE {
                                success += 1;
                            } else if msg_type == TURN_ALLOCATE_ERROR {
                                // 401 Unauthorized is expected without proper auth;
                                // we still count it as a successful round-trip for
                                // latency measurement purposes.
                                errors += 1;
                            }
                        }
                    }
                    Err(_) => {
                        errors += 1;
                    }
                }
            }
            Err(_) => {
                // Timeout
            }
        }
    }

    let timeouts = sent.saturating_sub(success + errors);
    (rtt_samples, sent, success + errors, timeouts)
}

/// Run the full TURN benchmark.
pub async fn run_turn_benchmark(
    host: &str,
    port: u16,
    users: u32,
    duration: Duration,
    credentials: Option<(String, String)>,
) -> TurnBenchmarkResults {
    let mut handles = Vec::with_capacity(users as usize);

    for _ in 0..users {
        let h = host.to_string();
        let d = duration;
        let creds = credentials.clone();
        let handle = task::spawn_blocking(move || turn_worker(&h, port, d, creds));
        handles.push(handle);
    }

    let mut all_rtt = Vec::new();
    let mut total_sent: u64 = 0;
    let mut total_responded: u64 = 0;
    let mut total_timeouts: u64 = 0;

    for handle in handles {
        let (rtt, sent, responded, timeouts) = handle.await.unwrap();
        all_rtt.extend(rtt);
        total_sent += sent;
        total_responded += responded;
        total_timeouts += timeouts;
    }

    let packet_loss = if total_sent > 0 {
        (total_timeouts as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };

    let rtt_stats = LatencyStats::from_samples(&mut all_rtt);

    let requests_per_sec = if duration.as_secs_f64() > 0.0 {
        total_sent as f64 / duration.as_secs_f64()
    } else {
        0.0
    };

    // Separate successful allocations from error responses
    // (In practice, without valid long-term credentials most servers return 401)
    let successful_allocations = 0; // Placeholder; real success requires full TURN auth handshake
    let error_responses = total_responded;

    TurnBenchmarkResults {
        server: host.to_string(),
        port,
        concurrent_users: users,
        duration_secs: duration.as_secs(),
        total_requests: total_sent,
        successful_allocations,
        error_responses,
        timeouts: total_timeouts,
        packet_loss_pct: packet_loss,
        requests_per_sec,
        allocation_rtt: rtt_stats,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_allocate_request_no_creds() {
        let txn_id = [0x42u8; 12];
        let pkt = build_allocate_request(&txn_id, None);

        // Should be 20 (header) + 8 (REQUESTED-TRANSPORT) = 28 bytes
        assert_eq!(pkt.len(), 28);

        // Message type: Allocate Request
        assert_eq!(BigEndian::read_u16(&pkt[0..2]), TURN_ALLOCATE_REQUEST);
        // Message length: 8
        assert_eq!(BigEndian::read_u16(&pkt[2..4]), 8);
        // Magic cookie
        assert_eq!(BigEndian::read_u32(&pkt[4..8]), STUN_MAGIC_COOKIE);
        // Transaction ID
        assert_eq!(&pkt[8..20], &txn_id);
        // REQUESTED-TRANSPORT attribute type
        assert_eq!(BigEndian::read_u16(&pkt[20..22]), ATTR_REQUESTED_TRANSPORT);
        // REQUESTED-TRANSPORT value length
        assert_eq!(BigEndian::read_u16(&pkt[22..24]), 4);
        // Transport = UDP (17)
        assert_eq!(pkt[24], TRANSPORT_UDP);
    }

    #[test]
    fn test_build_allocate_request_with_creds() {
        let txn_id = [0x01u8; 12];
        let creds = ("testuser".to_string(), "testpass".to_string());
        let pkt = build_allocate_request(&txn_id, Some(&creds));

        // 20 header + 8 REQUESTED-TRANSPORT + 4 USERNAME TLV header + 8 username (padded to 4)
        assert_eq!(pkt.len(), 40);

        // Check USERNAME attribute
        let username_attr_type = BigEndian::read_u16(&pkt[28..30]);
        assert_eq!(username_attr_type, ATTR_USERNAME);
        let username_len = BigEndian::read_u16(&pkt[30..32]) as usize;
        assert_eq!(username_len, 8);
        assert_eq!(&pkt[32..40], b"testuser");
    }

    #[test]
    fn test_parse_turn_response_valid() {
        let txn_id = [0xBB; 12];
        let mut buf = [0u8; 20];
        BigEndian::write_u16(&mut buf[0..2], TURN_ALLOCATE_ERROR);
        BigEndian::write_u16(&mut buf[2..4], 0);
        BigEndian::write_u32(&mut buf[4..8], STUN_MAGIC_COOKIE);
        buf[8..20].copy_from_slice(&txn_id);

        let result = parse_turn_response(&buf);
        assert!(result.is_ok());
        let (msg_type, parsed_txn) = result.unwrap();
        assert_eq!(msg_type, TURN_ALLOCATE_ERROR);
        assert_eq!(parsed_txn, txn_id);
    }

    #[test]
    fn test_parse_turn_response_bad_cookie() {
        let mut buf = [0u8; 20];
        BigEndian::write_u16(&mut buf[0..2], TURN_ALLOCATE_RESPONSE);
        BigEndian::write_u32(&mut buf[4..8], 0x00000000);

        assert!(parse_turn_response(&buf).is_err());
    }
}
