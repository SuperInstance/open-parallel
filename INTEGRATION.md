# Integration Guide: Ternary Protocol in Open Parallel (Tokio)

> How the ternary-protocol communicates over async channels, and how ensemble methods route tasks in the Tokio runtime.

## Overview

Open Parallel (fork of Tokio) integrates task intelligence via `tokio-crackle` — a crate that monitors task execution patterns using information-theoretic measures. Ternary signals flow over Tokio's async channels to classify runtime health and route work accordingly.

## Ternary Crate

| Crate | Location | Role |
|-------|----------|------|
| `tokio-crackle` | `tokio-crackle/` | Task intelligence monitor: detects correlated tasks, starvation cascades, and runtime phase transitions |

### Ternary Signals in tokio-crackle

Each task's throughput is classified into ternary outcomes:

- **+1 (Choose/Nominal)** — task is healthy, poll count within expected range
- **0 (Unknown/PreTransition)** — task throughput is shifting, may be early cascade signal
- **-1 (Avoid/Starving)** — task is starving, poll count dropped significantly

## Integration Points

### 1. Ternary-Protocol over Async Channels

Task health signals propagate via Tokio's `mpsc` channels:

```rust
use tokio::sync::mpsc;
use tokio_crackle::{TaskIntelligenceMonitor, TaskIntelligenceReport, RuntimePhase};

// Create a channel for ternary health signals
let (tx, mut rx) = mpsc::channel::<TaskIntelligenceReport>(32);

// Spawn a monitor that sends reports periodically
let monitor = Arc::new(Mutex::new(TaskIntelligenceMonitor::new("my-service")));
let mon = monitor.clone();
tokio::spawn(async move {
    let mut ticker = tokio::time::interval(Duration::from_secs(10));
    loop {
        ticker.tick().await;
        let report = mon.lock().unwrap().report();
        tx.send(report).await.ok();
    }
});

// Consumer reacts to ternary phase signals
tokio::spawn(async move {
    while let Some(report) = rx.recv().await {
        match report.phase {
            RuntimePhase::Nominal       => { /* +1: all good */ }
            RuntimePhase::PreTransition => { /*  0: watch closely */ }
            RuntimePhase::Transitioning => { /* -1: cascade in progress */ }
            RuntimePhase::Recovered     => { /* stabilized, verify */ }
        }
    }
});
```

**Where it connects:** Any Tokio application's task spawning layer. The monitor is a passive observer — it reads Tokio's runtime metrics and classifies them.

### 2. Ensemble Methods for Task Routing

Multiple task intelligence monitors can vote on routing decisions using ensemble ternary consensus:

```rust
use tokio_crackle::TaskIntelligenceMonitor;
use std::sync::{Arc, Mutex};

/// Ensemble of monitors — each watches a different subsystem
struct EnsembleRouter {
    monitors: Vec<Arc<Mutex<TaskIntelligenceMonitor>>>,
    threshold: f64, // consensus threshold (e.g., 0.6 = 60% agreement)
}

impl EnsembleRouter {
    /// Route a task based on ensemble ternary vote
    fn route(&self, task_name: &str) -> TernaryDecision {
        let votes: Vec<Trit> = self.monitors.iter().map(|m| {
            let report = m.lock().unwrap().report();
            // Classify each monitor's view as a trit
            match report.phase {
                RuntimePhase::Nominal       => Trit::Choose,
                RuntimePhase::PreTransition => Trit::Unknown,
                RuntimePhase::Transitioning => Trit::Avoid,
                RuntimePhase::Recovered     => Trit::Unknown,
            }
        }).collect();

        let choose = votes.iter().filter(|v| **v == Trit::Choose).count() as f64;
        let ratio = choose / votes.len() as f64;

        if ratio >= self.threshold {
            TernaryDecision::Accept
        } else {
            TernaryDecision::Reject
        }
    }
}
```

**Where it connects:** Task routing in high-concurrency Tokio services — when multiple subsystems report health, the ensemble decides whether to accept new work, shed load, or redirect.

### 3. Feeding Tokio Runtime Metrics

The monitor ingests Tokio's built-in runtime metrics:

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

**Where it connects:** Tokio's `runtime::Metrics` API — zero-cost when enabled, provides per-worker poll counts that feed into the ternary classifier.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Your Tokio Application                                  │
│                                                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                 │
│  │ Worker 0 │ │ Worker 1 │ │ Worker N │  Tokio runtime   │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘  metrics API    │
│       │             │             │                       │
│       └──────┬──────┴─────────────┘                       │
│              │ poll counts                                 │
│              ▼                                            │
│  ┌──────────────────────────┐                             │
│  │ TaskIntelligenceMonitor  │  tokio-crackle              │
│  │  ├─ MI correlation       │                             │
│  │  ├─ TE direction         │                             │
│  │  └─ JSD phase detection  │                             │
│  └──────────┬───────────────┘                             │
│             │ mpsc channel                                 │
│             ▼                                             │
│  ┌──────────────────────────┐                             │
│  │ EnsembleRouter           │                             │
│  │  (ternary vote on tasks) │                             │
│  └──────────────────────────┘                             │
└──────────────────────────────────────────────────────────┘
```

## Committed Files

- `d16006af` — `tokio-crackle/` — full crate implementation: TaskIntelligenceMonitor, RuntimePhase detection, MI/TE/JSD computations
- `6a9ba84b` — `tokio-crackle/INTEGRATION.md` — crate-level integration guide (already existed)
- `6a9ba84b` — `tokio-crackle/README.md` — polished README

## Key Design Constraints

- **No unsafe code:** `#![deny(unsafe_code)]` via crackle-runtime
- **No runtime hooks:** Pure observer — doesn't modify Tokio internals
- **Thread-safe:** Wrap in `Arc<Mutex<>>` or `Arc<RwLock<>>` for multi-worker access
- **Minimal dependencies:** Only `tokio` (path dep) and optional `serde`

## Extending the Integration

1. **Custom phase detectors:** Add new `RuntimePhase` variants with domain-specific detection logic
2. **Integration with `tokio-stream`:** Wrap monitor reports as a `Stream<Item = TaskIntelligenceReport>`
3. **Integration with `tokio-util`:** Use `CancellationToken` to gracefully shut down monitoring tasks
4. **Metrics export:** Serialize reports via the `serde` feature for Prometheus/OpenTelemetry ingestion
