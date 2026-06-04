# Future Integration: open-parallel (Tokio Fork)

## Current State
The SuperInstance fork of Tokio — the Rust async runtime. Adds `tokio-crackle`: task intelligence that detects correlated tasks, starvation cascades, and runtime phase transitions using information-theoretic measures. Everything else is upstream Tokio, untouched.

> **Note:** This is a fork of the Tokio project. We preserve upstream code and add SuperInstance-specific features.

## Integration Opportunities

### With AsyncConstruct (construct-core Layer 2)
AsyncConstruct requires an async runtime. Tokio IS that runtime. tokio-crackle adds fleet-specific intelligence: when multiple rooms' tasks are correlated (all waiting on the same LLM proxy), crackle detects the bottleneck and alerts the Forgemaster. When a starvation cascade begins (one room hogs all compute), crackle catches it before other rooms starve.

### With room-as-codespace
Each Codespace runs a Tokio runtime managing the room's async tasks: LLM proxy calls, ternary-protocol message processing, skill loading/unloading, and cell grid computation. tokio-crackle monitors task health and detects when the room's event loop is stressed.

### With fastloop-guard
tokio-crackle's correlated task detection and fastloop-guard's repeated-query detection are complementary. When crackle detects correlated tasks AND guard detects repeated queries, the system automatically caches the repeated result — eliminating both the correlation and the repetition.

## Our Integration (Not Upstream Changes)
We do NOT modify Tokio's core runtime. Our addition is:
- `tokio-crackle` crate: information-theoretic task analysis
- SuperInstance task configuration presets for room workloads
- Metrics bridge to fleet monitoring

## Potential in Mature Systems
Every room's async runtime includes tokio-crackle. The fleet operates at massive scale because crackle prevents starvation cascades before they happen. Rooms share compute fairly, correlated tasks are batched, and the runtime self-tunes based on information-theoretic measures of task health.

## Cross-Pollination Ideas
- **fastloop-guard**: Guard's cache eliminates the repeated queries that crackle detects
- **forgemaster**: Crackle's task metrics inform GPU dispatch decisions
- **lever-runner**: Async task management for the trust compiler's pipeline

## Dependencies for Next Steps
- tokio-crackle production hardening
- Room-specific task configuration presets
- Metrics bridge to fleet monitoring (oracle1-index)
