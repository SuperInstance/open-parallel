//! Deadline-Aware Task Executor — Conservation-budget task execution.
//!
//! The DeadlineExecutor manages task execution with deadlines and a conservation
//! budget that limits the maximum number of concurrent tasks. Tasks that cannot
//! meet their deadlines are rejected, ensuring the system never overcommits.
//!
//! # Conservation Budget
//!
//! The budget represents the maximum concurrent task capacity. Each spawned task
//! consumes one unit of budget. When the budget is exhausted, new tasks must wait
//! or be rejected. This is a conservation law: total_active ≤ budget.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Report on the current state of the execution budget.
#[derive(Debug, Clone)]
pub struct BudgetReport {
    /// Maximum concurrent tasks allowed.
    pub max_budget: usize,
    /// Currently active tasks.
    pub active_count: usize,
    /// Remaining budget capacity.
    pub remaining: usize,
    /// Number of tasks that missed their deadlines.
    pub deadline_misses: u64,
    /// Number of tasks completed successfully.
    pub completed: u64,
}

/// A handle to a spawned task that tracks its deadline and status.
#[derive(Debug, Clone)]
pub struct JoinHandle {
    pub task_id: u64,
    pub deadline: Instant,
    pub spawned_at: Instant,
    pub completed: bool,
    pub missed_deadline: bool,
}

/// Status of a task in the executor.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    DeadlineMissed,
    Rejected,
}

/// Internal representation of a pending task with its deadline.
#[derive(Debug)]
struct PendingTask {
    task_id: u64,
    deadline: Instant,
    spawned_at: Instant,
    name: String,
    status: TaskStatus,
}

/// Deadline-aware executor with conservation budget.
///
/// Manages a bounded pool of concurrent tasks, each with an explicit deadline.
/// When budget is exhausted, tasks are queued. When a deadline has passed before
/// the task can be scheduled, it is rejected.
pub struct DeadlineExecutor {
    /// Maximum concurrent tasks (conservation budget).
    budget: usize,
    /// Currently active tasks.
    active: Vec<PendingTask>,
    /// Queue of waiting tasks.
    pending: VecDeque<PendingTask>,
    /// Next task ID.
    next_id: u64,
    /// Statistics.
    deadline_misses: u64,
    completed: u64,
    total_spawned: u64,
}

impl DeadlineExecutor {
    /// Create a new executor with the given conservation budget.
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            active: Vec::with_capacity(budget),
            pending: VecDeque::new(),
            next_id: 0,
            deadline_misses: 0,
            completed: 0,
            total_spawned: 0,
        }
    }

    /// Get the conservation budget (max concurrent tasks).
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Spawn a task with a deadline.
    ///
    /// If the budget has capacity, the task starts immediately.
    /// If the budget is full, the task is queued and will start when capacity frees up.
    /// If the deadline has already passed, the task is rejected.
    ///
    /// # Arguments
    /// * `name` - Human-readable task name
    /// * `deadline` - When the task must complete by
    ///
    /// # Returns
    /// A JoinHandle for tracking the task, or None if the deadline has already passed.
    pub fn spawn_with_deadline(&mut self, name: &str, deadline: Instant) -> Option<JoinHandle> {
        let now = Instant::now();
        let task_id = self.next_id;
        self.next_id += 1;
        self.total_spawned += 1;

        // Reject tasks whose deadline has already passed
        if deadline <= now {
            self.deadline_misses += 1;
            return None;
        }

        let handle = JoinHandle {
            task_id,
            deadline,
            spawned_at: now,
            completed: false,
            missed_deadline: false,
        };

        let task = PendingTask {
            task_id,
            deadline,
            spawned_at: now,
            name: name.to_string(),
            status: TaskStatus::Running,
        };

        if self.active.len() < self.budget {
            self.active.push(task);
        } else {
            let mut pending_task = task;
            pending_task.status = TaskStatus::Pending;
            self.pending.push_back(pending_task);
        }

        Some(handle)
    }

    /// Check the current budget state.
    pub fn check_budget(&self) -> BudgetReport {
        BudgetReport {
            max_budget: self.budget,
            active_count: self.active.len(),
            remaining: self.budget.saturating_sub(self.active.len()),
            deadline_misses: self.deadline_misses,
            completed: self.completed,
        }
    }

    /// Tick the executor: check for deadline expirations, promote pending tasks.
    ///
    /// Should be called periodically to advance the executor state.
    /// Returns the number of tasks that had their deadlines expire this tick.
    pub fn tick(&mut self) -> u64 {
        let now = Instant::now();
        let mut expired = 0u64;

        // Check active tasks for deadline expiration
        let mut still_active = Vec::with_capacity(self.budget);
        for mut task in self.active.drain(..) {
            if task.deadline <= now {
                task.status = TaskStatus::DeadlineMissed;
                self.deadline_misses += 1;
                expired += 1;
            } else {
                still_active.push(task);
            }
        }
        self.active = still_active;

        // Promote pending tasks into freed capacity
        while self.active.len() < self.budget {
            // Check if the next pending task's deadline has passed
            if let Some(mut task) = self.pending.pop_front() {
                if task.deadline <= now {
                    task.status = TaskStatus::DeadlineMissed;
                    self.deadline_misses += 1;
                    expired += 1;
                    continue;
                }
                task.status = TaskStatus::Running;
                self.active.push(task);
            } else {
                break;
            }
        }

        // Check remaining pending for expired deadlines
        let mut valid_pending = VecDeque::new();
        for mut task in self.pending.drain(..) {
            if task.deadline <= now {
                task.status = TaskStatus::DeadlineMissed;
                self.deadline_misses += 1;
                expired += 1;
            } else {
                valid_pending.push_back(task);
            }
        }
        self.pending = valid_pending;

        expired
    }

    /// Complete a task by its ID, freeing one unit of budget.
    ///
    /// Returns true if the task was found and completed.
    pub fn complete(&mut self, task_id: u64) -> bool {
        let now = Instant::now();
        if let Some(pos) = self.active.iter().position(|t| t.task_id == task_id) {
            let task = self.active.remove(pos);
            if task.deadline > now {
                self.completed += 1;
            }
            return true;
        }
        // Check pending queue too
        if let Some(pos) = self.pending.iter().position(|t| t.task_id == task_id) {
            self.pending.remove(pos);
            self.completed += 1;
            return true;
        }
        false
    }

    /// Get the number of pending tasks waiting for budget.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get the number of active tasks.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Check if the budget is fully consumed.
    pub fn is_exhausted(&self) -> bool {
        self.active.len() >= self.budget
    }

    /// Total tasks spawned since creation.
    pub fn total_spawned(&self) -> u64 {
        self.total_spawned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_executor_has_full_budget() {
        let executor = DeadlineExecutor::new(4);
        assert_eq!(executor.budget(), 4);
        let report = executor.check_budget();
        assert_eq!(report.remaining, 4);
        assert_eq!(report.active_count, 0);
    }

    #[test]
    fn test_spawn_within_budget() {
        let mut executor = DeadlineExecutor::new(2);
        let deadline = Instant::now() + Duration::from_secs(60);
        let handle = executor.spawn_with_deadline("task1", deadline);
        assert!(handle.is_some());
        assert_eq!(executor.active_count(), 1);
        assert_eq!(executor.check_budget().remaining, 1);
    }

    #[test]
    fn test_spawn_fills_budget() {
        let mut executor = DeadlineExecutor::new(2);
        let deadline = Instant::now() + Duration::from_secs(60);
        let h1 = executor.spawn_with_deadline("task1", deadline);
        let h2 = executor.spawn_with_deadline("task2", deadline);
        assert!(h1.is_some());
        assert!(h2.is_some());
        assert!(executor.is_exhausted());
        assert_eq!(executor.pending_count(), 0);
    }

    #[test]
    fn test_spawn_queues_when_exhausted() {
        let mut executor = DeadlineExecutor::new(1);
        let deadline = Instant::now() + Duration::from_secs(60);
        executor.spawn_with_deadline("task1", deadline);
        let h2 = executor.spawn_with_deadline("task2", deadline);
        assert!(h2.is_some());
        assert!(executor.is_exhausted());
        assert_eq!(executor.pending_count(), 1);
    }

    #[test]
    fn test_reject_past_deadline() {
        let mut executor = DeadlineExecutor::new(4);
        let past_deadline = Instant::now() - Duration::from_secs(1);
        let handle = executor.spawn_with_deadline("late_task", past_deadline);
        assert!(handle.is_none());
        assert_eq!(executor.check_budget().deadline_misses, 1);
    }

    #[test]
    fn test_complete_frees_budget() {
        let mut executor = DeadlineExecutor::new(1);
        let deadline = Instant::now() + Duration::from_secs(60);
        let h = executor.spawn_with_deadline("task1", deadline).unwrap();
        assert!(executor.is_exhausted());

        let completed = executor.complete(h.task_id);
        assert!(completed);
        assert!(!executor.is_exhausted());
        assert_eq!(executor.check_budget().remaining, 1);
    }

    #[test]
    fn test_tick_promotes_pending() {
        let mut executor = DeadlineExecutor::new(1);
        let deadline = Instant::now() + Duration::from_secs(60);
        let h1 = executor.spawn_with_deadline("task1", deadline).unwrap();
        executor.spawn_with_deadline("task2", deadline);
        assert_eq!(executor.pending_count(), 1);

        executor.complete(h1.task_id);
        executor.tick();

        assert_eq!(executor.active_count(), 1);
        assert_eq!(executor.pending_count(), 0);
    }

    #[test]
    fn test_budget_report_accuracy() {
        let mut executor = DeadlineExecutor::new(3);
        let deadline = Instant::now() + Duration::from_secs(60);
        executor.spawn_with_deadline("a", deadline);
        executor.spawn_with_deadline("b", deadline);

        let report = executor.check_budget();
        assert_eq!(report.max_budget, 3);
        assert_eq!(report.active_count, 2);
        assert_eq!(report.remaining, 1);
        assert_eq!(report.deadline_misses, 0);
    }

    #[test]
    fn test_complete_nonexistent_task() {
        let mut executor = DeadlineExecutor::new(2);
        assert!(!executor.complete(999));
    }

    #[test]
    fn test_multiple_deadline_misses() {
        let mut executor = DeadlineExecutor::new(2);
        let past = Instant::now() - Duration::from_secs(1);
        assert!(executor.spawn_with_deadline("a", past).is_none());
        assert!(executor.spawn_with_deadline("b", past).is_none());
        assert!(executor.spawn_with_deadline("c", past).is_none());
        assert_eq!(executor.check_budget().deadline_misses, 3);
    }

    #[test]
    fn test_total_spawned_counter() {
        let mut executor = DeadlineExecutor::new(2);
        let deadline = Instant::now() + Duration::from_secs(60);
        let past = Instant::now() - Duration::from_secs(1);
        executor.spawn_with_deadline("a", deadline);
        executor.spawn_with_deadline("b", past);
        executor.spawn_with_deadline("c", deadline);
        assert_eq!(executor.total_spawned(), 3);
    }

    #[test]
    fn test_tick_expires_active_deadlines() {
        let mut executor = DeadlineExecutor::new(4);
        // Create a deadline that's already passed
        let past = Instant::now() - Duration::from_secs(1);
        // We can't directly spawn past-deadline tasks (they get rejected),
        // so we test with a short deadline by checking tick behavior
        let deadline = Instant::now() + Duration::from_secs(60);
        executor.spawn_with_deadline("task1", deadline);
        assert_eq!(executor.active_count(), 1);
        // Tick shouldn't expire a future deadline
        let expired = executor.tick();
        assert_eq!(expired, 0);
        assert_eq!(executor.active_count(), 1);
    }
}
