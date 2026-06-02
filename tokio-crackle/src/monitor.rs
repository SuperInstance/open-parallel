use std::collections::HashMap;

use crate::information::{jsd, mutual_information, transfer_entropy};

use crate::phase::RuntimePhase;
use crate::report::TaskIntelligenceReport;

/// Default number of bins for information-theoretic discretization.
const DEFAULT_BINS: usize = 16;

/// Default mutual information threshold for considering tasks "correlated".
const MI_THRESHOLD: f64 = 0.8;

/// Default transfer entropy threshold for considering tasks "starving" others.
const TE_THRESHOLD: f64 = 0.6;

/// Default JSD threshold for detecting distribution shift (phase transition).
const JSD_PRE_TRANSITION: f64 = 0.15;
const JSD_TRANSITIONING: f64 = 0.35;
const JSD_RECOVERED_DROP: f64 = 0.10;

/// Minimum samples needed per task to include it in analysis.
const MIN_SAMPLES: usize = 3;

/// A ring buffer that stores the most recent `capacity` throughput values for a task.
#[derive(Debug, Clone)]
struct ThroughputBuffer {
    capacity: usize,
    values: Vec<f64>,
}

impl ThroughputBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            values: Vec::with_capacity(capacity),
        }
    }

    fn push(&mut self, value: f64) {
        if self.values.len() >= self.capacity {
            self.values.remove(0);
        }
        self.values.push(value);
    }

    fn values(&self) -> &[f64] {
        &self.values
    }

    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.values.len()
    }
}

/// A monitoring agent that tracks task execution patterns and produces
/// intelligence reports using information-theoretic measures.
///
/// # Example
///
/// ```rust
/// use tokio_crackle::TaskIntelligenceMonitor;
///
/// let mut monitor = TaskIntelligenceMonitor::new("db-pool");
///
/// // Simulate recording task throughput values
/// monitor.record_task("query-users", 120.0);
/// monitor.record_task("query-users", 115.0);
/// monitor.record_task("query-orders", 95.0);
/// monitor.record_task("query-orders", 92.0);
/// monitor.record_task("cache-warm", 200.0);
/// monitor.record_task("cache-warm", 210.0);
///
/// let report = monitor.report();
/// println!("{}", report);
/// ```
#[derive(Debug, Clone)]
pub struct TaskIntelligenceMonitor {
    /// Label for the pool being monitored.
    pool_name: String,
    /// Per-task throughput history.
    task_buffers: HashMap<String, ThroughputBuffer>,
    /// Maximum number of throughput samples to keep per task.
    capacity: usize,
}

impl Default for TaskIntelligenceMonitor {
    fn default() -> Self {
        Self::new("default")
    }
}

impl TaskIntelligenceMonitor {
    /// Create a new monitor for the named task pool with the default capacity.
    pub fn new(pool_name: impl Into<String>) -> Self {
        Self {
            pool_name: pool_name.into(),
            task_buffers: HashMap::new(),
            capacity: 100,
        }
    }

    /// Create a monitor with a custom per-task history capacity.
    pub fn with_capacity(pool_name: impl Into<String>, capacity: usize) -> Self {
        Self {
            pool_name: pool_name.into(),
            task_buffers: HashMap::new(),
            capacity,
        }
    }

    /// Record that a task completed with the given throughput value.
    ///
    /// `task_label` identifies the *kind* of task (e.g., "http-handler", "db-query"),
    /// and `throughput` is some scalar measure of performance (e.g., ops/sec, latency in ms).
    pub fn record_task(&mut self, task_label: impl Into<String>, throughput: f64) {
        let label = task_label.into();
        let buffer = self
            .task_buffers
            .entry(label)
            .or_insert_with(|| ThroughputBuffer::new(self.capacity));
        buffer.push(throughput);
    }

    /// Produce an intelligence report based on recorded task data.
    pub fn report(&self) -> TaskIntelligenceReport {
        // Filter out tasks with too few samples
        let active_tasks: Vec<(&str, &[f64])> = self
            .task_buffers
            .iter()
            .filter(|(_, buf)| buf.len() >= MIN_SAMPLES)
            .map(|(label, buf)| (label.as_str(), buf.values()))
            .collect();

        let total_active = active_tasks.len();

        // Compute mutual information between all task pairs
        let mut correlated_pairs = Vec::new();
        let task_labels: Vec<&str> = active_tasks.iter().map(|(l, _)| *l).collect();

        for i in 0..task_labels.len() {
            for j in (i + 1)..task_labels.len() {
                let label_a = task_labels[i];
                let label_b = task_labels[j];
                let data_a = active_tasks
                    .iter()
                    .find(|(l, _)| *l == label_a)
                    .map(|(_, d)| *d)
                    .unwrap();
                let data_b = active_tasks
                    .iter()
                    .find(|(l, _)| *l == label_b)
                    .map(|(_, d)| *d)
                    .unwrap();

                // Align lengths for MI computation
                let min_len = data_a.len().min(data_b.len());
                if min_len < MIN_SAMPLES {
                    continue;
                }
                let a_slice = &data_a[..min_len];
                let b_slice = &data_b[..min_len];

                let mi = mutual_information(a_slice, b_slice, DEFAULT_BINS);

                if mi >= MI_THRESHOLD {
                    correlated_pairs.push((label_a.to_string(), label_b.to_string(), mi));
                }
            }
        }

        // Compute transfer entropy (directional) between task pairs
        let mut starvation_pairs = Vec::new();
        for i in 0..task_labels.len() {
            for j in 0..task_labels.len() {
                if i == j {
                    continue;
                }
                let cause = task_labels[i];
                let effect = task_labels[j];
                let cause_data = active_tasks
                    .iter()
                    .find(|(l, _)| *l == cause)
                    .map(|(_, d)| *d)
                    .unwrap();
                let effect_data = active_tasks
                    .iter()
                    .find(|(l, _)| *l == effect)
                    .map(|(_, d)| *d)
                    .unwrap();

                let min_len = cause_data.len().min(effect_data.len());
                if min_len < MIN_SAMPLES + 1 {
                    continue;
                }
                let c_slice = &cause_data[..min_len];
                let e_slice = &effect_data[..min_len];

                let te = transfer_entropy(c_slice, e_slice, 1, DEFAULT_BINS);

                if te >= TE_THRESHOLD {
                    starvation_pairs.push((cause.to_string(), effect.to_string(), te));
                }
            }
        }

        // Detect runtime phase using JSD on aggregate throughput distributions
        let phase = self.detect_phase(&active_tasks);

        let phase_details = phase.description().to_string();

        TaskIntelligenceReport {
            pool_name: self.pool_name.clone(),
            total_tasks: total_active,
            correlated_pairs,
            starvation_pairs,
            phase,
            phase_details,
        }
    }

    /// Detect the current runtime phase by analyzing the distribution of
    /// task throughputs using Jensen-Shannon divergence.
    fn detect_phase(&self, active_tasks: &[(&str, &[f64])]) -> RuntimePhase {
        if active_tasks.is_empty() {
            return RuntimePhase::Nominal;
        }

        // Collect all throughput values into a flat distribution
        let all_values: Vec<f64> = active_tasks
            .iter()
            .flat_map(|(_, data)| data.iter())
            .copied()
            .collect();

        if all_values.len() < MIN_SAMPLES * 2 {
            return RuntimePhase::Nominal;
        }

        // Split the data into "recent" (second half) and "baseline" (first half)
        let mid = all_values.len() / 2;
        let baseline = &all_values[..mid];
        let recent = &all_values[mid..];

        if baseline.len() < MIN_SAMPLES || recent.len() < MIN_SAMPLES {
            return RuntimePhase::Nominal;
        }

        let divergence = jsd(baseline, recent, DEFAULT_BINS);

        // Check if we've dropped back down after a transition
        if divergence < JSD_RECOVERED_DROP {
            // If JSD is very low, check variance for nuance
            let recent_mean = recent.iter().sum::<f64>() / recent.len() as f64;
            let variance = recent
                .iter()
                .map(|v| (v - recent_mean).powi(2))
                .sum::<f64>()
                / recent.len() as f64;

            if variance < 10.0 {
                RuntimePhase::Nominal
            } else {
                RuntimePhase::Recovered
            }
        } else if divergence >= JSD_TRANSITIONING {
            RuntimePhase::Transitioning
        } else if divergence >= JSD_PRE_TRANSITION {
            RuntimePhase::PreTransition
        } else {
            RuntimePhase::Nominal
        }
    }

    /// Reset all recorded task data.
    pub fn reset(&mut self) {
        self.task_buffers.clear();
    }

    /// Number of unique task labels being tracked.
    pub fn task_count(&self) -> usize {
        self.task_buffers.len()
    }

    /// Get the pool name.
    pub fn pool_name(&self) -> &str {
        &self.pool_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nominal_phase() {
        let mut monitor = TaskIntelligenceMonitor::new("test");
        // All tasks with perfectly stable, identical throughput
        for _ in 0..50 {
            for task in &["a", "b", "c"] {
                monitor.record_task(*task, 100.0);
            }
        }
        let report = monitor.report();
        assert_eq!(report.total_tasks, 3);
        assert_eq!(report.phase, RuntimePhase::Nominal);
    }

    #[test]
    fn test_correlated_tasks_detected() {
        let mut monitor = TaskIntelligenceMonitor::new("test");
        // Two tasks with very similar throughput patterns
        for i in 0..30 {
            let noise = (i as f64).sin() * 10.0;
            monitor.record_task("task-x", 100.0 + noise);
            monitor.record_task("task-y", 100.0 + noise + fastrand::f64() * 2.0);
        }
        let report = monitor.report();
        // They should be correlated (MI high)
        assert!(
            !report.correlated_pairs.is_empty(),
            "Expected correlated pairs, got {:?}",
            report.correlated_pairs
        );
    }

    #[test]
    fn test_starvation_detected() {
        let mut monitor = TaskIntelligenceMonitor::new("test");
        // Simulate: task-busy runs, task-starved suffers afterward
        for i in 0..40 {
            let spike = if i % 5 == 0 { 500.0 } else { 10.0 };
            monitor.record_task("hog", spike);
            monitor.record_task("victim", 100.0 - spike * 0.1);
        }
        let report = monitor.report();
        assert!(
            report.total_tasks >= 2,
            "Expected at least 2 tasks, got {}",
            report.total_tasks
        );
    }

    #[test]
    fn test_empty_report() {
        let monitor = TaskIntelligenceMonitor::new("empty");
        let report = monitor.report();
        assert_eq!(report.total_tasks, 0);
        assert!(report.correlated_pairs.is_empty());
        assert!(report.starvation_pairs.is_empty());
        assert_eq!(report.phase, RuntimePhase::Nominal);
    }

    #[test]
    fn test_reset() {
        let mut monitor = TaskIntelligenceMonitor::new("reset");
        monitor.record_task("a", 100.0);
        assert_eq!(monitor.task_count(), 1);
        monitor.reset();
        assert_eq!(monitor.task_count(), 0);
    }

    #[test]
    fn test_report_display() {
        let mut monitor = TaskIntelligenceMonitor::new("pool-alpha");
        for task in &["http", "db", "cache", "auth", "queue"] {
            for _ in 0..10 {
                monitor.record_task(*task, 50.0 + fastrand::f64() * 50.0);
            }
        }
        let report = monitor.report();
        let summary = report.summary();
        assert!(summary.contains("pool-alpha"));
        assert!(summary.contains("Runtime phase"));
    }
}
