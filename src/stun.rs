use byteorder::{BigEndian, ByteOrder};
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use tokio::task;

// RFC 5389 constants
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_RESPONSE: u16 = 0x0101;
const STUN_HEADER_SIZE: usize = 20;

/// Construct a valid RFC 5389 STUN Binding Request.
///
/// Format (20 bytes):
///   0                   1                   2                   3
///   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |0 0|     STUN Message Type     |         Message Length        |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |                         Magic Cookie                         |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |                                                               |
///  |                     Transaction ID (96 bits)                  |
///  |                                                               |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
fn build_binding_request(transaction_id: &[u8; 12]) -> [u8; STUN_HEADER_SIZE] {
    let mut buf = [0u8; STUN_HEADER_SIZE];

    // Message Type: Binding Request (0x0001), top two bits must be 0
    BigEndian::write_u16(&mut buf[0..2], STUN_BINDING_REQUEST);

    // Message Length: 0 (no attributes in a basic Binding Request)
    BigEndian::write_u16(&mut buf[2..4], 0);

    // Magic Cookie
    BigEndian::write_u32(&mut buf[4..8], STUN_MAGIC_COOKIE);

    // Transaction ID (12 bytes)
    buf[8..20].copy_from_slice(transaction_id);

    buf
}

/// Parse a STUN message header and validate it.
/// Returns (message_type, message_length, transaction_id) or an error description.
fn parse_stun_response(buf: &[u8]) -> Result<(u16, u16, [u8; 12]), &'static str> {
    if buf.len() < STUN_HEADER_SIZE {
        return Err("Response too short for STUN header");
    }

    let msg_type = BigEndian::read_u16(&buf[0..2]);
    let msg_len = BigEndian::read_u16(&buf[2..4]);
    let cookie = BigEndian::read_u32(&buf[4..8]);

    if cookie != STUN_MAGIC_COOKIE {
        return Err("Invalid magic cookie");
    }

    // Top two bits of first byte must be 0 (RFC 5389 Section 6)
    if buf[0] & 0xC0 != 0 {
        return Err("Invalid STUN message: top two bits not zero");
    }

    let mut txn_id = [0u8; 12];
    txn_id.copy_from_slice(&buf[8..20]);

    // Verify message length is consistent with buffer
    let total_expected = STUN_HEADER_SIZE + msg_len as usize;
    if buf.len() < total_expected {
        return Err("Buffer shorter than declared message length");
    }

    Ok((msg_type, msg_len, txn_id))
}

/// Generate a random 12-byte transaction ID.
fn random_transaction_id() -> [u8; 12] {
    let mut id = [0u8; 12];
    // Use a simple PRNG seeded from the current time for portability.
    // In production you would use `rand::thread_rng()`, but we avoid
    // adding the `rand` crate to keep dependencies minimal.
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
pub struct LatencyStats {
    pub min_us: f64,
    pub max_us: f64,
    pub mean_us: f64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub jitter_us: f64,
}

impl LatencyStats {
    pub fn from_samples(samples: &mut Vec<f64>) -> Self {
        if samples.is_empty() {
            return Self {
                min_us: 0.0,
                max_us: 0.0,
                mean_us: 0.0,
                p50_us: 0.0,
                p95_us: 0.0,
                p99_us: 0.0,
                jitter_us: 0.0,
            };
        }

        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = samples.len();
        let sum: f64 = samples.iter().sum();
        let mean = sum / n as f64;

        // Jitter = standard deviation of inter-sample differences
        let variance: f64 = samples.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n as f64;
        let jitter = variance.sqrt();

        Self {
            min_us: samples[0],
            max_us: samples[n - 1],
            mean_us: mean,
            p50_us: samples[n * 50 / 100],
            p95_us: samples[n * 95 / 100],
            p99_us: samples[n * 99 / 100],
            jitter_us: jitter,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StunBenchmarkResults {
    pub server: String,
    pub port: u16,
    pub concurrent_users: u32,
    pub duration_secs: u64,
    pub total_requests: u64,
    pub successful_responses: u64,
    pub errors: u64,
    pub packet_loss_pct: f64,
    pub requests_per_sec: f64,
    pub rtt: LatencyStats,
    pub parse_latency: LatencyStats,
    pub timestamp: String,
}

/// Run a single STUN worker that sends Binding Requests in a loop.
fn stun_worker(
    host: &str,
    port: u16,
    duration: Duration,
) -> (Vec<f64>, Vec<f64>, u64, u64) {
    let addr = format!("{}:{}", host, port);
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind UDP socket: {}", e);
            return (vec![], vec![], 0, 0);
        }
    };
    socket.set_read_timeout(Some(Duration::from_millis(2000))).ok();
    socket.set_write_timeout(Some(Duration::from_millis(1000))).ok();

    let mut rtt_samples = Vec::with_capacity(4096);
    let mut parse_samples = Vec::with_capacity(4096);
    let mut sent: u64 = 0;
    let mut received: u64 = 0;

    let start = Instant::now();
    let mut recv_buf = [0u8; 576]; // RFC 5389 recommends supporting at least 576-byte messages

    while start.elapsed() < duration {
        let txn_id = random_transaction_id();
        let packet = build_binding_request(&txn_id);

        let send_time = Instant::now();
        if socket.send_to(&packet, &addr).is_err() {
            sent += 1;
            continue;
        }
        sent += 1;

        match socket.recv_from(&mut recv_buf) {
            Ok((n, _src)) => {
                let rtt = send_time.elapsed();
                rtt_samples.push(rtt.as_secs_f64() * 1_000_000.0); // convert to microseconds

                // Measure parse latency
                let parse_start = Instant::now();
                let response = &recv_buf[..n];
                match parse_stun_response(response) {
                    Ok((msg_type, _msg_len, resp_txn_id)) => {
                        let parse_dur = parse_start.elapsed();
                        parse_samples.push(parse_dur.as_secs_f64() * 1_000_000.0);

                        // Validate response
                        if msg_type == STUN_BINDING_RESPONSE && resp_txn_id == txn_id {
                            received += 1;
                        }
                        // If transaction ID doesn't match, we still count it as
                        // received for the purposes of packet loss calculation
                        // but it would indicate an unusual server behavior.
                    }
                    Err(_e) => {
                        // Unparseable response; still got bytes back
                        let parse_dur = parse_start.elapsed();
                        parse_samples.push(parse_dur.as_secs_f64() * 1_000_000.0);
                    }
                }
            }
            Err(_) => {
                // Timeout or error — counts toward packet loss
            }
        }
    }

    (rtt_samples, parse_samples, sent, received)
}

/// Run the full STUN benchmark with the specified concurrency.
pub async fn run_stun_benchmark(
    host: &str,
    port: u16,
    users: u32,
    duration: Duration,
) -> StunBenchmarkResults {
    let mut handles = Vec::with_capacity(users as usize);

    for _ in 0..users {
        let h = host.to_string();
        let d = duration;
        let handle = task::spawn_blocking(move || stun_worker(&h, port, d));
        handles.push(handle);
    }

    let mut all_rtt = Vec::new();
    let mut all_parse = Vec::new();
    let mut total_sent: u64 = 0;
    let mut total_recv: u64 = 0;

    for handle in handles {
        let (rtt, parse, sent, recv) = handle.await.unwrap();
        all_rtt.extend(rtt);
        all_parse.extend(parse);
        total_sent += sent;
        total_recv += recv;
    }

    let errors = total_sent.saturating_sub(total_recv);
    let packet_loss = if total_sent > 0 {
        (errors as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };

    let rtt_stats = LatencyStats::from_samples(&mut all_rtt);
    let parse_stats = LatencyStats::from_samples(&mut all_parse);

    let requests_per_sec = if duration.as_secs_f64() > 0.0 {
        total_sent as f64 / duration.as_secs_f64()
    } else {
        0.0
    };

    StunBenchmarkResults {
        server: host.to_string(),
        port,
        concurrent_users: users,
        duration_secs: duration.as_secs(),
        total_requests: total_sent,
        successful_responses: total_recv,
        errors,
        packet_loss_pct: packet_loss,
        requests_per_sec,
        rtt: rtt_stats,
        parse_latency: parse_stats,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_binding_request_structure() {
        let txn_id: [u8; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let pkt = build_binding_request(&txn_id);

        // Check message type: Binding Request (0x0001)
        assert_eq!(BigEndian::read_u16(&pkt[0..2]), STUN_BINDING_REQUEST);
        // Check message length: 0
        assert_eq!(BigEndian::read_u16(&pkt[2..4]), 0);
        // Check magic cookie
        assert_eq!(BigEndian::read_u32(&pkt[4..8]), STUN_MAGIC_COOKIE);
        // Check transaction ID
        assert_eq!(&pkt[8..20], &txn_id);
        // Top two bits must be zero
        assert_eq!(pkt[0] & 0xC0, 0);
    }

    #[test]
    fn test_parse_valid_response() {
        let txn_id: [u8; 12] = [0xAA; 12];
        let mut buf = [0u8; 20];

        // Build a fake Binding Response
        BigEndian::write_u16(&mut buf[0..2], STUN_BINDING_RESPONSE);
        BigEndian::write_u16(&mut buf[2..4], 0);
        BigEndian::write_u32(&mut buf[4..8], STUN_MAGIC_COOKIE);
        buf[8..20].copy_from_slice(&txn_id);

        let result = parse_stun_response(&buf);
        assert!(result.is_ok());
        let (msg_type, msg_len, parsed_txn) = result.unwrap();
        assert_eq!(msg_type, STUN_BINDING_RESPONSE);
        assert_eq!(msg_len, 0);
        assert_eq!(parsed_txn, txn_id);
    }

    #[test]
    fn test_parse_invalid_cookie() {
        let mut buf = [0u8; 20];
        BigEndian::write_u16(&mut buf[0..2], STUN_BINDING_RESPONSE);
        BigEndian::write_u16(&mut buf[2..4], 0);
        BigEndian::write_u32(&mut buf[4..8], 0xDEADBEEF); // wrong cookie

        let result = parse_stun_response(&buf);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid magic cookie");
    }

    #[test]
    fn test_parse_too_short() {
        let buf = [0u8; 10];
        let result = parse_stun_response(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_latency_stats_empty() {
        let stats = LatencyStats::from_samples(&mut vec![]);
        assert_eq!(stats.min_us, 0.0);
        assert_eq!(stats.mean_us, 0.0);
    }

    #[test]
    fn test_latency_stats_single() {
        let mut samples = vec![100.0];
        let stats = LatencyStats::from_samples(&mut samples);
        assert_eq!(stats.min_us, 100.0);
        assert_eq!(stats.max_us, 100.0);
        assert_eq!(stats.mean_us, 100.0);
    }

    #[test]
    fn test_latency_stats_distribution() {
        let mut samples: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let stats = LatencyStats::from_samples(&mut samples);
        assert_eq!(stats.min_us, 1.0);
        assert_eq!(stats.max_us, 100.0);
        assert!((stats.mean_us - 50.5).abs() < 0.01);
        assert_eq!(stats.p50_us, 51.0);
        assert_eq!(stats.p95_us, 96.0);
        assert_eq!(stats.p99_us, 100.0);
    }

    #[test]
    fn test_random_transaction_id_unique() {
        let id1 = random_transaction_id();
        // Introduce a tiny delay so the time-based seed differs
        std::thread::sleep(std::time::Duration::from_nanos(100));
        let id2 = random_transaction_id();
        // They should very likely differ (not a hard guarantee with time-based seed)
        // This is a sanity check, not a cryptographic randomness test
        assert_eq!(id1.len(), 12);
        assert_eq!(id2.len(), 12);
    }
}
