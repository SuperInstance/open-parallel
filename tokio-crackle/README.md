# tokio-crackle

> **🏆 SuperInstance Enhancement: Task Intelligence**
> Your async runtime has structure. This shows you which tasks cause cascades.
> Same tokio. Self-aware tokio.

A Rust crate that monitors task execution patterns in Tokio-based applications using
information-theoretic measures from the [crackle-runtime](https://crates.io/crates/crackle-runtime)
framework.

## What It Does

- **Jensen-Shannon divergence**: Compares task throughput distributions to detect anomalous tasks
- **Transfer entropy**: Detects causal chains — "task A's completion CAUSES task B's starvation"
- **Mutual information**: Finds correlated tasks (always run together → probably should be one task)
- **Phase detection**: Identifies runtime phase transitions — Nominal → PreTransition → Transitioning → Recovered

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
// Output: Task pool "my-pool" has 2 tasks. ...
```

## Example Output

```
Task pool "worker-pool-1" has 47 tasks. 3 are correlated (MI > 0.8).
1 is causing starvation in 5 others (TE > 0.6). Runtime phase: Transitioning.
```

See [INTEGRATION.md](./INTEGRATION.md) for detailed integration instructions.

## Information-Theoretic Measures

| Measure | What It Detects | Range |
|---------|----------------|-------|
| **Mutual Information** | Task pairs that always co-occur | [0, ∞) bits |
| **Transfer Entropy** | Directional information flow (causation) | [0, ∞) bits |
| **Jensen-Shannon Divergence** | Distribution shift (phase transitions) | [0, log₂(bins)] bits |
| **Permutation Entropy** | Regularity/structure in time series | [0, 1] normalized |
| **KL Divergence** | Asymmetric distribution distance | [0, ∞) bits |

## License

MIT — same as Tokio.
