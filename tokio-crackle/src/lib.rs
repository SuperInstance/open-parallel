//! # tokio-crackle
//!
//! **Task intelligence for Tokio.**
//!
//! Monitors task execution patterns using information-theoretic measures from
//! [`crackle-runtime`] to detect:
//!
//! - **Correlated tasks** — tasks that always run together (probably should be one task)
//! - **Starvation cascades** — task A's completion causes task B's starvation
//! - **Runtime phase transitions** — anomalies in the task throughput distribution
//!
//! ## Usage
//!
//! ```rust,no_run
//! use tokio_crackle::TaskIntelligenceMonitor;
//!
//! let mut monitor = TaskIntelligenceMonitor::new("worker-pool-1");
//!
//! // Record task completions with throughput values
//! monitor.record_task("http-handler", 142.5);
//! monitor.record_task("db-query", 89.3);
//!
//! // Generate a report
//! let report = monitor.report();
//! println!("{}", report);
//! ```
//!
//! ## Output Example
//!
//! ```text
//! Task pool "worker-pool-1" has 47 tasks. 3 are correlated (MI > 0.8).
//! 1 is causing starvation in 5 others (TE > 0.6). Runtime phase: Transitioning.
//! ```

pub mod information;
mod monitor;
mod phase;
mod report;

pub use monitor::TaskIntelligenceMonitor;
pub use phase::RuntimePhase;
pub use report::TaskIntelligenceReport;
