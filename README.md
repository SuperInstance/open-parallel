*[TokioConf 2026 program and tickets are now available!](https://tokioconf.com)*

---

# Tokio

A runtime for writing reliable, asynchronous, and slim applications with
the Rust programming language. It is:

* **Fast**: Tokio's zero-cost abstractions give you bare-metal performance.
* **Reliable**: Tokio leverages Rust's ownership, type system, and concurrency model to reduce bugs and ensure thread safety.
* **Scalable**: Tokio has a minimal footprint, and handles backpressure and cancellation naturally.

This is the **SuperInstance fork**. It adds [`tokio-crackle`](./tokio-crackle) — task intelligence that detects correlated tasks, starvation cascades, and runtime phase transitions using information-theoretic measures. Everything else is upstream Tokio, untouched.

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/tokio.svg
[crates-url]: https://crates.io/crates/tokio
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tokio-rs/tokio/blob/master/LICENSE
[actions-badge]: https://github.com/tokio-rs/tokio/workflows/CI/badge.svg
[actions-url]: https://github.com/tokio-rs/tokio/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/500028886025895936.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/tokio

[Website](https://tokio.rs) |
[Guides](https://tokio.rs/tokio/tutorial) |
[API Docs](https://docs.rs/tokio/latest/tokio) |
[Chat](https://discord.gg/tokio)

---

## What's New: `tokio-crackle`

Your runtime has 847 tasks. When task A spikes, task B spikes 200ms later. You can't see that — until now.

`tokio-crackle` is an observer, not a runtime patch. You feed it task throughput values; it computes:

| Measure | What it finds |
|---------|--------------|
| Mutual Information | Tasks that always co-vary (MI > 0.8 → correlated) |
| Transfer Entropy | Directional causality — task A predicts task B (TE > 0.6 → starvation risk) |
| Jensen-Shannon Divergence | Distribution drift from baseline (phase transition detection) |
| Permutation Entropy | Regularity in throughput patterns |

**20 tests** across the crate — 11 unit tests on information-theoretic functions, 6 on the monitor, 3 integration tests.

### Quick Start

```toml
[dependencies]
tokio-crackle = "0.1"
```

```rust
use tokio_crackle::TaskIntelligenceMonitor;
use std::sync::{Arc, Mutex};

let monitor = Arc::new(Mutex::new(TaskIntelligenceMonitor::new("my-app")));

// In your tasks, record throughput after each poll:
mon.lock().unwrap().record_task("http-handler", 142.5);
mon.lock().unwrap().record_task("db-query", 89.3);

// Generate a report:
let report = mon.lock().unwrap().report();
println!("{}", report.summary());
```

### Output

```
Task pool "my-app" has 47 tasks. 3 are correlated (MI > 0.8).
1 is causing starvation in 5 others (TE > 0.6). Runtime phase: Transitioning.
```

Detailed output:

```
=== Task Intelligence Report: "my-app" ===
Tasks: 47
Correlated pairs (MI > 0.8):
  db-pool ↔ http-handler (MI = 0.73)
Starvation chains (TE > 0.6):
  db-pool → http-handler (TE = 0.68)
  db-pool → cache-refresh (TE = 0.61)
Runtime phase: Transitioning
  Distribution shifted significantly (JSD = 0.45 from baseline).
  Multiple task groups exhibiting starvation behavior.
```

### Runtime Phases

| Phase | What's happening |
|-------|-----------------|
| `Nominal` | Stable throughput distributions. Nothing to see. |
| `PreTransition` | Some tasks slowing. JSD creeping up. Time for preemptive action. |
| `Transitioning` | Cascade underway. Strong TE spikes. Intervene now. |
| `Recovered` | Stabilized after a transition. Returning to normal. |

### Integration with Tokio metrics

Wire Tokio's built-in runtime metrics into the monitor:

```rust
use tokio::runtime::Handle;

fn poll_metrics(monitor: &mut TaskIntelligenceMonitor, handle: &Handle) {
    let metrics = handle.metrics();
    for i in 0..metrics.num_workers() {
        let throughput = metrics.worker_poll_count(i) as f64;
        monitor.record_task(format!("worker-{}", i), throughput);
    }
}
```

See [`tokio-crackle/README.md`](./tokio-crackle/README.md) and [`tokio-crackle/INTEGRATION.md`](./tokio-crackle/INTEGRATION.md) for the full API.

---

## Overview

Tokio is an event-driven, non-blocking I/O platform for writing
asynchronous applications with the Rust programming language. At a high
level, it provides a few major components:

* A multithreaded, work-stealing based task [scheduler].
* A reactor backed by the operating system's event queue (epoll, kqueue, IOCP, etc.).
* Asynchronous [TCP and UDP][net] sockets.

[net]: https://docs.rs/tokio/latest/tokio/net/index.html
[scheduler]: https://docs.rs/tokio/latest/tokio/runtime/index.html

## Example

A basic TCP echo server:

```toml
[dependencies]
tokio = { version = "1.52.3", features = ["full"] }
```

```rust,no_run
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
```

More examples [here][examples]. For a larger real-world example, see [mini-redis].

[examples]: https://github.com/tokio-rs/tokio/tree/master/examples
[mini-redis]: https://github.com/tokio-rs/mini-redis/

See [docs][feature-flag-docs] for available feature flags.

[feature-flag-docs]: https://docs.rs/tokio/#feature-flags

## Getting Help

Check the [Guides] or [API documentation] first. If the answer isn't there, ask in [Discord][chat] or [GitHub Discussions][discussions].

[Guides]: https://tokio.rs/tokio/tutorial
[API documentation]: https://docs.rs/tokio/latest/tokio
[chat]: https://discord.gg/tokio
[discussions]: https://github.com/tokio-rs/tokio/discussions

## Contributing

We have a [contributing guide][guide].

[guide]: https://github.com/tokio-rs/tokio/blob/master/docs/contributing/README.md

## Related Projects

* [`axum`]: Web framework focused on ergonomics and modularity.
* [`hyper`]: Fast HTTP/1.1 and HTTP/2 implementation.
* [`tonic`]: gRPC over HTTP/2, focused on performance and interoperability.
* [`warp`]: Composable web server framework.
* [`tower`]: Modular components for networking clients and servers.
* [`tracing`]: Application-level tracing and async-aware diagnostics.
* [`mio`]: Low-level cross-platform I/O abstraction that powers Tokio.
* [`bytes`]: Efficient byte buffer utilities.
* [`loom`]: Testing tool for concurrent Rust code.

[`axum`]: https://github.com/tokio-rs/axum
[`warp`]: https://github.com/seanmonstar/warp
[`hyper`]: https://github.com/hyperium/hyper
[`tonic`]: https://github.com/hyperium/tonic
[`tower`]: https://github.com/tower-rs/tower
[`loom`]: https://github.com/tokio-rs/loom
[`tracing`]: https://github.com/tokio-rs/tracing
[`mio`]: https://github.com/tokio-rs/mio
[`bytes`]: https://github.com/tokio-rs/bytes

## Changelog

Each crate has its own changelog:

* `tokio` — [changelog](https://github.com/tokio-rs/tokio/blob/master/tokio/CHANGELOG.md)
* `tokio-util` — [changelog](https://github.com/tokio-rs/tokio/blob/master/tokio-util/CHANGELOG.md)
* `tokio-stream` — [changelog](https://github.com/tokio-rs/tokio/blob/master/tokio-stream/CHANGELOG.md)
* `tokio-macros` — [changelog](https://github.com/tokio-rs/tokio/blob/master/tokio-macros/CHANGELOG.md)
* `tokio-test` — [changelog](https://github.com/tokio-rs/tokio/blob/master/tokio-test/CHANGELOG.md)

## Supported Rust Versions

Tokio keeps a rolling MSRV of **at least** 6 months. Current MSRV: **1.71**.

| Releases | MSRV |
|----------|------|
| 1.48 – now | Rust 1.71 |
| 1.39 – 1.47 | Rust 1.70 |
| 1.30 – 1.38 | Rust 1.63 |

## Release Schedule

No fixed schedule. Roughly one minor release per month, patches as needed.

## Bug Patching Policy

LTS releases receive backported fixes for at least a year:

* `1.47.x` — LTS until September 2026 (MSRV 1.70)
* `1.51.x` — LTS until March 2027 (MSRV 1.71)

Pin to an LTS release:

```toml
tokio = { version = "~1.47", features = [...] }
```

## License

[MIT](https://github.com/tokio-rs/tokio/blob/master/LICENSE)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Tokio by you shall be licensed as MIT, without any additional terms or conditions.
