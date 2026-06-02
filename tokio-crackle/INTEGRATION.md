# Integration Guide: tokio-crackle with Tokio

This document explains how `tokio-crackle` integrates with the Tokio runtime to provide
task intelligence — monitoring task execution patterns using information-theoretic measures.

## Overview

`tokio-crackle` is a standalone crate that monitors task execution in Tokio-based
applications. It does **not** modify the Tokio runtime itself; instead, it provides
an observer API that you hook into your existing Tokio application.

## Integration Points

### 1. Basic Usage

Add `tokio-crackle` to your `Cargo.toml`:

```toml
[dependencies]
tokio-crackle = "0.1"
```

Then create a monitor and record task throughput:

```rust
use tokio_crackle::TaskIntelligenceMonitor;
use std::sync::Mutex;
use std::sync::Arc;

let monitor = Arc::new(Mutex::new(TaskIntelligenceMonitor::new("my-app")));

// In your tasks:
let mon = monitor.clone();
tokio::spawn(async move {
    // ... do work ...
    let throughput = /* measure throughput */;
    mon.lock().unwrap().record_task("my-task", throughput);
});
```

### 2. Periodic Reporting

Use a background task to check for issues:

```rust
use tokio::time::{interval, Duration};
use std::sync::{Arc, Mutex};

let monitor: Arc<Mutex<TaskIntelligenceMonitor>> = /* ... */;
let mon = monitor.clone();

tokio::spawn(async move {
    let mut ticker = interval(Duration::from_secs(60));
    loop {
        ticker.tick().await;
        let report = mon.lock().unwrap().report();
        if report.phase != RuntimePhase::Nominal {
            tracing::warn!("{}", report.summary());
        }
    }
});
```

### 3. Using Tokio's Runtime Metrics

If you enable Tokio's runtime metrics, you can feed them into the monitor:

```rust
use tokio::runtime::Handle;

fn poll_metrics(monitor: &mut TaskIntelligenceMonitor, handle: &Handle) {
    let metrics = handle.metrics();

    // Number of tasks spawned per worker
    for i in 0..metrics.num_workers() {
        let throughput = metrics.worker_poll_count(i) as f64;
        monitor.record_task(format!("worker-{}", i), throughput);
    }

    // Global throughput
    let global = metrics.worker_poll_count(metrics.num_workers()) as f64;
    monitor.record_task("global", global);
}
```

### 4. Detecting Starvation

When the monitor reports starvation (TE > 0.6), consider:

- Increasing worker threads
- Splitting the starving task into smaller subtasks
- Adjusting task priorities
- Using `tokio::task::unconstrained` for the starving producer

### 5. Detecting Correlated Tasks

When the monitor reports correlated tasks (MI > 0.8):

- Consider merging them into a single task
- Or verify they're genuinely independent (false positives possible with limited data)
- Add synchronization or batching between them

### 6. Phase Transitions

The monitor reports four phases:

| Phase | Meaning | Action |
|-------|---------|--------|
| Nominal | Normal operation | None |
| PreTransition | Some tasks slowing | Preemptive investigation |
| Transitioning | Cascade in progress | Immediate intervention |
| Recovered | Post-cascade stabilization | Verify and continue monitoring |

## Architecture

```
┌─────────────────────────────────────────┐
│           Your Tokio Application        │
│  ┌──────────┐  ┌──────────┐  ┌───────┐ │
│  │ Task A   │  │ Task B   │  │Task C │ │
│  └────┬─────┘  └────┬─────┘  └───┬───┘ │
│       │              │            │     │
│       └──────┬───────┴────────────┘     │
│              │ task throughput           │
│              ▼                           │
│  ┌──────────────────────┐               │
│  │ TaskIntelligence     │               │
│  │ Monitor              │               │
│  │   ├─ MI correlation  │               │
│  │   ├─ TE direction    │               │
│  │   └─ JSD phase       │               │
│  └──────────────────────┘               │
└─────────────────────────────────────────┘
          │ report()
          ▼
┌──────────────────────┐
│ TaskIntelligence     │
│ Report               │
│   - summary()        │
│   - detailed()       │
└──────────────────────┘
```

## API Design Notes

- **No unsafe code**: The crate uses `#![deny(unsafe_code)]` (via `crackle-runtime`).
- **No runtime hooks**: Works alongside any Tokio configuration.
- **Thread-safe**: Wrap in `Arc<Mutex<>>` or `Arc<RwLock<>>` for multi-threaded use.
- **Minimal dependencies**: Only `tokio` (path dep), and optional `serde`.

## License

Same as Tokio: MIT.
