use tabled::{Table, Tabled};

use crate::stun::StunBenchmarkResults;
use crate::turn::TurnBenchmarkResults;

#[derive(Tabled)]
struct StunSummaryRow {
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Value")]
    value: String,
}

#[derive(Tabled)]
struct LatencyRow {
    #[tabled(rename = "Percentile")]
    percentile: String,
    #[tabled(rename = "RTT (us)")]
    rtt_us: String,
    #[tabled(rename = "Parse (us)")]
    parse_us: String,
}

#[derive(Tabled)]
struct TurnSummaryRow {
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Value")]
    value: String,
}

#[derive(Tabled)]
struct TurnLatencyRow {
    #[tabled(rename = "Percentile")]
    percentile: String,
    #[tabled(rename = "Allocate RTT (us)")]
    rtt_us: String,
}

/// Print STUN benchmark results as a formatted table.
pub fn print_stun_table(results: &StunBenchmarkResults) {
    println!();
    println!("=== STUN Benchmark Results ===");
    println!();

    let summary = vec![
        StunSummaryRow {
            metric: "Server".into(),
            value: format!("{}:{}", results.server, results.port),
        },
        StunSummaryRow {
            metric: "Concurrent Users".into(),
            value: results.concurrent_users.to_string(),
        },
        StunSummaryRow {
            metric: "Duration".into(),
            value: format!("{}s", results.duration_secs),
        },
        StunSummaryRow {
            metric: "Total Requests".into(),
            value: format_number(results.total_requests),
        },
        StunSummaryRow {
            metric: "Successful Responses".into(),
            value: format_number(results.successful_responses),
        },
        StunSummaryRow {
            metric: "Errors".into(),
            value: format_number(results.errors),
        },
        StunSummaryRow {
            metric: "Packet Loss".into(),
            value: format!("{:.3}%", results.packet_loss_pct),
        },
        StunSummaryRow {
            metric: "Requests/sec".into(),
            value: format!("{:.1}", results.requests_per_sec),
        },
        StunSummaryRow {
            metric: "Timestamp".into(),
            value: results.timestamp.clone(),
        },
    ];

    let table = Table::new(summary).to_string();
    println!("{}", table);

    println!();
    println!("--- Latency Distribution ---");
    println!();

    let latency_rows = vec![
        LatencyRow {
            percentile: "min".into(),
            rtt_us: format!("{:.1}", results.rtt.min_us),
            parse_us: format!("{:.3}", results.parse_latency.min_us),
        },
        LatencyRow {
            percentile: "p50".into(),
            rtt_us: format!("{:.1}", results.rtt.p50_us),
            parse_us: format!("{:.3}", results.parse_latency.p50_us),
        },
        LatencyRow {
            percentile: "mean".into(),
            rtt_us: format!("{:.1}", results.rtt.mean_us),
            parse_us: format!("{:.3}", results.parse_latency.mean_us),
        },
        LatencyRow {
            percentile: "p95".into(),
            rtt_us: format!("{:.1}", results.rtt.p95_us),
            parse_us: format!("{:.3}", results.parse_latency.p95_us),
        },
        LatencyRow {
            percentile: "p99".into(),
            rtt_us: format!("{:.1}", results.rtt.p99_us),
            parse_us: format!("{:.3}", results.parse_latency.p99_us),
        },
        LatencyRow {
            percentile: "max".into(),
            rtt_us: format!("{:.1}", results.rtt.max_us),
            parse_us: format!("{:.3}", results.parse_latency.max_us),
        },
        LatencyRow {
            percentile: "jitter".into(),
            rtt_us: format!("{:.1}", results.rtt.jitter_us),
            parse_us: format!("{:.3}", results.parse_latency.jitter_us),
        },
    ];

    let latency_table = Table::new(latency_rows).to_string();
    println!("{}", latency_table);
    println!();
}

/// Print TURN benchmark results as a formatted table.
pub fn print_turn_table(results: &TurnBenchmarkResults) {
    println!();
    println!("=== TURN Benchmark Results ===");
    println!();

    let summary = vec![
        TurnSummaryRow {
            metric: "Server".into(),
            value: format!("{}:{}", results.server, results.port),
        },
        TurnSummaryRow {
            metric: "Concurrent Users".into(),
            value: results.concurrent_users.to_string(),
        },
        TurnSummaryRow {
            metric: "Duration".into(),
            value: format!("{}s", results.duration_secs),
        },
        TurnSummaryRow {
            metric: "Total Requests".into(),
            value: format_number(results.total_requests),
        },
        TurnSummaryRow {
            metric: "Successful Allocations".into(),
            value: format_number(results.successful_allocations),
        },
        TurnSummaryRow {
            metric: "Error Responses".into(),
            value: format_number(results.error_responses),
        },
        TurnSummaryRow {
            metric: "Timeouts".into(),
            value: format_number(results.timeouts),
        },
        TurnSummaryRow {
            metric: "Packet Loss".into(),
            value: format!("{:.3}%", results.packet_loss_pct),
        },
        TurnSummaryRow {
            metric: "Requests/sec".into(),
            value: format!("{:.1}", results.requests_per_sec),
        },
        TurnSummaryRow {
            metric: "Timestamp".into(),
            value: results.timestamp.clone(),
        },
    ];

    let table = Table::new(summary).to_string();
    println!("{}", table);

    println!();
    println!("--- Allocation RTT Distribution ---");
    println!();

    let latency_rows = vec![
        TurnLatencyRow {
            percentile: "min".into(),
            rtt_us: format!("{:.1}", results.allocation_rtt.min_us),
        },
        TurnLatencyRow {
            percentile: "p50".into(),
            rtt_us: format!("{:.1}", results.allocation_rtt.p50_us),
        },
        TurnLatencyRow {
            percentile: "mean".into(),
            rtt_us: format!("{:.1}", results.allocation_rtt.mean_us),
        },
        TurnLatencyRow {
            percentile: "p95".into(),
            rtt_us: format!("{:.1}", results.allocation_rtt.p95_us),
        },
        TurnLatencyRow {
            percentile: "p99".into(),
            rtt_us: format!("{:.1}", results.allocation_rtt.p99_us),
        },
        TurnLatencyRow {
            percentile: "max".into(),
            rtt_us: format!("{:.1}", results.allocation_rtt.max_us),
        },
        TurnLatencyRow {
            percentile: "jitter".into(),
            rtt_us: format!("{:.1}", results.allocation_rtt.jitter_us),
        },
    ];

    let latency_table = Table::new(latency_rows).to_string();
    println!("{}", latency_table);
    println!();
}

/// Format a number with comma separators for readability.
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1_000_000), "1,000,000");
        assert_eq!(format_number(123_456_789), "123,456,789");
    }
}
