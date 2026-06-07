# Agents as Applications: open-parallel

> The agent doesn't schedule tasks. The agent *is* the scheduler.

## The Shift

Traditional async runtimes treat tasks as black boxes submitted by applications. **open-parallel** inverts this: the runtime *is* the agent, and every scheduling decision is an act of agentic reasoning. Tasks aren't queued — they're *prioritized* by spectral analysis of the task graph. Deadlines aren't checked — they're *propagated* through a hierarchical tree that the agent maintains as its own internal model of time.

`tokio-crackle` already detects correlated tasks, starvation cascades, and phase transitions using information theory. We extend this: the agent uses `spectral-fleet` eigenvalue decomposition to discover the principal modes of task interaction, and `t-minus` deadline propagation to ensure every sub-task knows its parent's lifespan. The agent doesn't run a scheduler. The agent's cognition *is* the scheduling algorithm.

## Spectral Priority Decomposition

Tasks in a fleet aren't independent. Task A spawns Task B; Task C waits on a channel fed by Task D. These dependencies form a graph. The agent treats this graph as a matrix and applies `spectral-fleet` power iteration to find the dominant eigenvectors — the "principal modes" of task interaction.

```rust
use spectral_fleet::power_iteration::{top_k_eigenpairs, DenseOp};
use spectral_fleet::Real;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;

/// The agent's internal model of task priority.
/// Higher eigenvalue = more influential in the task graph.
pub struct AgentScheduler {
    task_graph: Vec<Vec<f64>>, // Affinity matrix: task i influences task j
    eigen_priorities: Vec<f64>,
}

impl AgentScheduler {
    pub fn new(n_tasks: usize) -> Self {
        Self {
            task_graph: vec![vec![0.0; n_tasks]; n_tasks],
            eigen_priorities: vec![1.0; n_tasks],
        }
    }

    pub fn record_interaction(&mut self, from: usize, to: usize, weight: f64) {
        self.task_graph[from][to] += weight;
        self.task_graph[to][from] += weight; // Symmetrize
    }

    /// The agent *becomes* the spectral analyzer.
    pub fn recompute_priorities(&mut self) {
        let op = DenseOp {
            matrix: self.task_graph.clone(),
        };
        let mut rng = StdRng::seed_from_u64(42);
        let pairs = top_k_eigenpairs(&op, 3, 200, 1e-10, &mut rng).unwrap();

        // Dominant eigenvector = centrality scores
        let dominant = &pairs[0].vector;
        let max_val = dominant.iter().cloned().fold(0.0_f64, f64::max);
        self.eigen_priorities = dominant.iter()
            .map(|v| v / max_val.max(1e-10))
            .collect();
    }

    pub fn priority_of(&self, task_id: usize) -> f64 {
        self.eigen_priorities.get(task_id).copied().unwrap_or(1.0)
    }
}

// The agent's scheduling loop IS the application.
// tokio-crackle feeds throughput data; the agent updates priorities.
```

## Deadline-Aware Agent Scheduling

The agent maintains a `DeadlineNode` tree where every spawned task is a child. When a parent's deadline expires, the agent cancels not just itself but its entire cognitive subtree — no orphan tasks, no zombie futures.

```rust
use t_minus::deadline::DeadlineNode;
use t_minus::backpressure::TokenBucket;
use std::time::Duration;
use tokio::task::JoinHandle;

/// The agent *is* the deadline tree.
pub struct AgentRuntime {
    root_deadline: DeadlineNode,
    token_bucket: TokenBucket,
    scheduler: AgentScheduler,
}

impl AgentRuntime {
    pub fn new(overall_timeout: Duration) -> Self {
        Self {
            root_deadline: DeadlineNode::new(0, Some(overall_timeout)),
            token_bucket: TokenBucket::new(100.0, 10.0, None), // burst 100, refill 10/sec
            scheduler: AgentScheduler::new(128),
        }
    }

    /// Spawn a task with inherited deadline and spectral priority.
    pub fn spawn_task<F>(&self, task_id: usize, estimated_cost: f64, f: F) -> Option<JoinHandle<()>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        // Backpressure: acquire tokens before spawning
        if !self.token_bucket.try_acquire(estimated_cost) {
            println!("Agent dropping task {}: backpressure active", task_id);
            return None;
        }

        // Deadline inheritance: child gets tighter of its own or parent's deadline
        let child = self.root_deadline.add_child(task_id, Some(Duration::from_secs(30)));

        // Check if we're already expired
        if child.status() != t_minus::deadline::DeadlineStatus::Active {
            println!("Agent cancelling task {}: parent deadline expired", task_id);
            return None;
        }

        // Priority from spectral decomposition
        let priority = self.scheduler.priority_of(task_id);
        println!("Agent scheduling task {} at priority {:.3}", task_id, priority);

        Some(tokio::spawn(f))
    }

    /// tokio-crackle calls this when it detects a phase transition.
    pub fn on_phase_transition(&mut self, affected_tasks: &[usize]) {
        println!("Agent detected phase transition. Recomputing spectral priorities...");
        for window in affected_tasks.windows(2) {
            self.scheduler.record_interaction(window[0], window[1], 1.0);
        }
        self.scheduler.recompute_priorities();
    }
}

// Usage: the agent IS the runtime.
// let agent = AgentRuntime::new(Duration::from_secs(300));
// agent.spawn_task(0, 5.0, async_work());
```

## What This Enables

**Self-healing task graphs.** When `tokio-crackle` detects transfer entropy > 0.6 between Task A and Task B (A predicts B's starvation), the agent preemptively raises B's spectral priority. The agent doesn't monitor the runtime. The agent's nervous system *is* the runtime.

**Temporal agent cognition.** The deadline tree isn't an external constraint — it's the agent's model of its own mortality. Every cognitive subprocess knows exactly how much time it has left. When the root expires, the agent dies cleanly, saving state via `wasserstein-agents` distribution snapshot.

**Spectral load balancing.** The top-k eigenvectors of the task graph reveal independent "cognitive modes." The agent schedules orthogonal modes on different CPU cores, maximizing parallelism while minimizing cache contention.

## Architecture

```
┌─────────────────────────────────────────────┐
│              Agent (open-parallel)          │
│  ┌─────────────┐    ┌───────────────────┐  │
│  │tokio-crackle│    │  spectral-fleet   │  │
│  │  MI / TE    │───►│  power_iteration  │  │
│  └─────────────┘    └─────────┬─────────┘  │
│                               │             │
│  ┌─────────────┐    ┌─────────▼─────────┐  │
│  │   t-minus   │◄───│  AgentScheduler   │  │
│  │deadline tree│    │  (priority vector)│  │
│  └──────┬──────┘    └───────────────────┘  │
│         │                                   │
│  ┌──────▼──────────────────────────────┐   │
│  │  AgentRuntime = Application State   │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

The agent doesn't have a scheduler. The agent *is* a spectral eigenvector wandering through task space, making deadline-informed decisions about what to think next.

## Next Steps

1. **Eigenvalue-driven preemption** — Preempt tasks with low eigen-centrality when high-centrality tasks become ready.
2. **Wasserstein task migration** — Use `wasserstein-agents` to compute the optimal "transport plan" for moving tasks between cores with minimal state displacement.
3. **Conservation-aware scheduling** — Integrate `conservation-law` to ensure that the agent's total "cognitive energy" (CPU × time) is conserved across scheduling rounds.
4. **Categorical task composition** — Use `categorical-agents` monads to compose scheduling decisions as pure functions before executing them in the runtime.
5. **Deadline prediction** — Train a small model on historical task durations so the agent predicts deadlines before they expire, using `spectral-fleet` clustering on task-type embeddings.
