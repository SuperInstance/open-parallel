# tokio-crackle

> **SuperInstance Enhancement: Task Intelligence**

Your async runtime has 847 tasks running. Some of them are correlated — when one
spikes, another spikes 200 ms later. You can't see this. Nobody can.

`tokio-crackle` makes it visible. It builds a correlation graph from task metrics.
Transfer entropy measures whether task A predicts task B's behavior.

## What It Shows

**Task `"db-pool"` → task `"http-handler"`: transfer entropy = 0.73.**
When `db-pool` latency spikes, `http-handler` latency spikes 180 ms later. 89% of
the time.

**Task `"cache-invalidator"` has JSD = 0.45 from its baseline.**
It's behaving differently than usual. Something changed.

**Your runtime has three task clusters that always spike together.**
They share a connection pool. The pool is the bottleneck — not any individual task.

**Runtime phases: Startup (0-2 s), Steady (2 s-4 h), Stressed (4 h+).**
Conservation laws differ per phase. `tokio-crackle` tracks which phase your
runtime is in.

## How It Works

| Measure | What It Detects | Range |
|---------|----------------|-------|
| **Mutual Information** | Tasks that always co-occur | [0, ∞) bits |
| **Transfer Entropy** | Directional information flow | [0, ∞) bits |
| **Jensen-Shannon Divergence** | Distribution shift | [0, log₂(bins)] bits |
| **Permutation Entropy** | Regularity in time series | [0, 1] normalized |
| **KL Divergence** | Asymmetric distribution distance | [0, ∞) bits |

## Quick Start

```rust
use tokio_crackle::TaskIntelligenceMonitor;

let mut monitor = TaskIntelligenceMonitor::new("my-pool");

// Record task throughput
monitor.record_task("http-handler", 142.5);
monitor.record_task("db-query", 89.3);
monitor.record_task("http-handler", 155.0);
monitor.record_task("db-query", 12.1);  // slow!

// Generate a report
let report = monitor.report();
println!("{}", report.summary());
```

## Example Output

```
Task pool "worker-pool-1" has 47 tasks. 3 are correlated (MI > 0.8).
1 is causing starvation in 5 others (TE > 0.6). Runtime phase: Transitioning.
```

See [INTEGRATION.md](./INTEGRATION.md) for detailed integration instructions.

## License

MIT — same as Tokio.
