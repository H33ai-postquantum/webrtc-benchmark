# webrtc-benchmark

Open-source benchmarking toolkit for WebRTC infrastructure. Measure STUN binding latency, TURN allocation throughput, and SFU fanout performance.

Built by [V100.ai](https://v100.ai) — the AI video API built on 20 Rust microservices.

## Install

```bash
cargo install webrtc-benchmark
```

## Usage

```bash
# Benchmark STUN binding requests
webrtc-benchmark stun --target stun:your-server.com:3478 --users 50 --duration 10s

# Benchmark TURN allocations  
webrtc-benchmark turn --target turn:your-server.com:3478 --users 20 --duration 10s

# Generate JSON report
webrtc-benchmark stun --target stun:server.com:3478 --format json
```

## What it measures

| Metric | Description |
|--------|-------------|
| STUN Binding RTT | Round-trip time for STUN Binding Request/Response |
| STUN Parse Latency | Time to parse a STUN message (local) |
| TURN Allocate RTT | Round-trip for TURN Allocate request |
| Packet Loss | % of requests with no response |
| Jitter | Variance in round-trip times |

## Sample Results

| Server | STUN RTT (p50) | STUN Parse | Packet Loss |
|--------|---------------|------------|-------------|
| coturn 4.6 | 1.2ms | 180ns | 0.01% |
| LiveKit 1.5 | 0.8ms | 95ns | 0.02% |
| V100 RustTURN | 0.4ms | 50ns | 0.00% |

> Benchmarks run on c8g.metal-48xl (Graviton4), same-region, UDP.

## Why we built this

We needed to benchmark our own [RustTURN server](https://v100.ai/blog/open-sourcing-rustturn) against coturn and LiveKit. Rather than keep the tool internal, we open-sourced it.

Read the full comparison: [Fastest WebRTC Server in 2026](https://v100.ai/blog/fastest-webrtc-server-2026)

## Built by V100

[V100](https://v100.ai) is an AI video platform built entirely in Rust. 20 microservices for transcription, editing, conferencing, broadcasting, and publishing — through one REST API.

- **Website:** [v100.ai](https://v100.ai)
- **Blog:** [v100.ai/blog](https://v100.ai/blog)  
- **Docs:** [docs.v100.ai](https://docs.v100.ai)
- **GitHub:** [github.com/H33ai-postquantum](https://github.com/H33ai-postquantum)

## Contributing

PRs welcome. Please open an issue first to discuss changes.

## License

MIT — see [LICENSE](LICENSE)

---

**H33 Products:** [H33-74](https://h33.ai) · [Auth1](https://auth1.ai) · [Chat101](https://chat101.ai) · [Cachee](https://cachee.ai) · [Z101](https://z101.ai) · [RevMine](https://revmine.ai) · [BotShield](https://h33.ai/botshield)

*Introducing H33-74. 74 bytes. Any computation. Post-quantum attested. Forever.*
