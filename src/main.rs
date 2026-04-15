use clap::{Parser, Subcommand, ValueEnum};
use std::time::Duration;

mod stun;
mod turn;
mod report;

#[derive(Parser)]
#[command(
    name = "webrtc-benchmark",
    version,
    about = "Open-source benchmarking toolkit for WebRTC TURN/STUN servers",
    long_about = "Measure STUN binding latency, TURN allocation throughput, and more.\nBuilt by V100.ai — the AI video API built on 20 Rust microservices."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Benchmark STUN binding requests (RFC 5389)
    Stun {
        /// STUN server address (e.g. stun:your-server.com:3478)
        #[arg(long)]
        target: String,

        /// Number of concurrent users to simulate
        #[arg(long, default_value_t = 1)]
        users: u32,

        /// Test duration (e.g. 10s, 30s, 60s)
        #[arg(long, default_value = "10s")]
        duration: String,

        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },

    /// Benchmark TURN allocation requests
    Turn {
        /// TURN server address (e.g. turn:your-server.com:3478)
        #[arg(long)]
        target: String,

        /// Number of concurrent users to simulate
        #[arg(long, default_value_t = 1)]
        users: u32,

        /// Test duration (e.g. 10s, 30s, 60s)
        #[arg(long, default_value = "10s")]
        duration: String,

        /// TURN username
        #[arg(long)]
        username: Option<String>,

        /// TURN credential
        #[arg(long)]
        credential: Option<String>,

        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },

    /// Generate a summary report from previous benchmark results
    Report {
        /// Path to a JSON results file
        #[arg(long)]
        input: String,

        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

fn parse_duration(s: &str) -> Duration {
    let s = s.trim();
    if let Some(secs) = s.strip_suffix('s') {
        Duration::from_secs(secs.parse::<u64>().unwrap_or(10))
    } else if let Some(ms) = s.strip_suffix("ms") {
        Duration::from_millis(ms.parse::<u64>().unwrap_or(10_000))
    } else {
        Duration::from_secs(s.parse::<u64>().unwrap_or(10))
    }
}

/// Parse a target string like "stun:host:port" or "turn:host:port" into (host, port).
fn parse_target(target: &str) -> (String, u16) {
    let stripped = target
        .strip_prefix("stun:")
        .or_else(|| target.strip_prefix("turn:"))
        .unwrap_or(target);

    if let Some(idx) = stripped.rfind(':') {
        let host = stripped[..idx].to_string();
        let port = stripped[idx + 1..].parse::<u16>().unwrap_or(3478);
        (host, port)
    } else {
        (stripped.to_string(), 3478)
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Stun {
            target,
            users,
            duration,
            format,
        } => {
            let (host, port) = parse_target(&target);
            let dur = parse_duration(&duration);
            println!(
                "Benchmarking STUN server {}:{} with {} concurrent users for {:?}",
                host, port, users, dur
            );

            let results = stun::run_stun_benchmark(&host, port, users, dur).await;

            match format {
                OutputFormat::Table => report::print_stun_table(&results),
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&results).unwrap();
                    println!("{}", json);
                }
            }
        }

        Commands::Turn {
            target,
            users,
            duration,
            username,
            credential,
            format,
        } => {
            let (host, port) = parse_target(&target);
            let dur = parse_duration(&duration);
            println!(
                "Benchmarking TURN server {}:{} with {} concurrent users for {:?}",
                host, port, users, dur
            );

            let creds = match (username, credential) {
                (Some(u), Some(c)) => Some((u, c)),
                _ => None,
            };

            let results = turn::run_turn_benchmark(&host, port, users, dur, creds).await;

            match format {
                OutputFormat::Table => report::print_turn_table(&results),
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&results).unwrap();
                    println!("{}", json);
                }
            }
        }

        Commands::Report { input, format } => {
            let data = std::fs::read_to_string(&input).expect("Failed to read input file");
            // Try to parse as STUN results first, then TURN
            if let Ok(results) = serde_json::from_str::<stun::StunBenchmarkResults>(&data) {
                match format {
                    OutputFormat::Table => report::print_stun_table(&results),
                    OutputFormat::Json => {
                        let json = serde_json::to_string_pretty(&results).unwrap();
                        println!("{}", json);
                    }
                }
            } else if let Ok(results) =
                serde_json::from_str::<turn::TurnBenchmarkResults>(&data)
            {
                match format {
                    OutputFormat::Table => report::print_turn_table(&results),
                    OutputFormat::Json => {
                        let json = serde_json::to_string_pretty(&results).unwrap();
                        println!("{}", json);
                    }
                }
            } else {
                eprintln!("Error: could not parse input file as STUN or TURN results");
                std::process::exit(1);
            }
        }
    }
}
