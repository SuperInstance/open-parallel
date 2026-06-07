# Agents as Applications: open-parallel

## The Agent IS the Scheduler

In traditional systems, a scheduler is code that someone writes to schedule tasks. In open-parallel, **the agent itself is the scheduler**. It doesn't generate scheduling code — it performs spectral decomposition on its own dependency structure in real-time.

### How It Works

1. **Spectral Decomposition as Self-Knowledge**: The agent builds a dependency matrix from its own task graph, then decomposes it into eigenvalues and eigenvectors. The eigenvectors reveal the task structure; the eigenvalues reveal the criticality.

2. **Conservation Budget**: The total spectral energy is conserved. The agent cannot allocate more priority than exists in the system. This prevents priority inflation and ensures that scheduling decisions are physically meaningful.

3. **Bottleneck Detection via Weakest Eigenmode**: The smallest eigenvalue corresponds to the weakest structural mode. Tasks aligned with this eigenvector are bottlenecks — they constrain the entire system. The agent identifies these automatically.

4. **Deadline-Aware Execution**: Tasks carry explicit deadlines. The DeadlineExecutor enforces conservation law: `active_tasks ≤ budget`. No oversubscription. Tasks that can't meet deadlines are rejected rather than queued indefinitely.

### Architecture

```
Agent
  ├── SpectralScheduler
  │   ├── build_dependency_matrix(tasks) → Matrix
  │   ├── spectral_decompose(matrix) → Eigenvalues + Eigenvectors
  │   ├── schedule(tasks) → ScheduledTasks (priority-ranked)
  │   └── detect_bottleneck(matrix) → TaskId
  │
  └── DeadlineExecutor
      ├── spawn_with_deadline(task, deadline, budget) → JoinHandle
      ├── tick() → (promotes pending, expires deadlines)
      └── check_budget() → BudgetReport
```

### Why Spectral?

Cosine similarity and heuristic scoring can't capture the deep structure of task dependencies. Eigenvalue decomposition reveals:

- **Critical paths** (dominant eigenvectors)
- **Bottlenecks** (smallest eigenvalues)
- **Independent subgraphs** (eigenvector clustering)

The agent uses this mathematical structure to make optimal scheduling decisions without explicit priority rules.

### The Agent Loop

```rust
loop {
    let tasks = agent.collect_pending_tasks();
    let scheduled = scheduler.schedule(tasks);
    let bottleneck = scheduler.detect_bottleneck(&dep_matrix);

    for task in scheduled {
        executor.spawn_with_deadline(task.name, task.deadline, budget);
    }

    executor.tick(); // promote pending, expire missed
    let report = executor.check_budget();

    agent.adapt_from(report); // adjust strategy based on budget state
}
```

### Files

- `src/spectral_scheduler.rs` — SpectralScheduler with power iteration, eigenvalue decomposition, bottleneck detection
- `src/deadline_executor.rs` — DeadlineExecutor with conservation budget, deadline enforcement, task lifecycle
