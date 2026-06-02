/// The detected phase of a task pool based on its execution patterns.
///
/// Phases are detected by analyzing the information-theoretic properties of
/// task throughput distributions over moving windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimePhase {
    /// Everything is running normally. Task throughput distributions are stable
    /// and no anomalous patterns detected.
    Nominal,
    /// Some tasks have begun to slow down. The distribution is starting to shift,
    /// but no cascading behavior is visible yet. This is the window for
    /// preemptive intervention.
    PreTransition,
    /// A cascade is underway. Correlations are strengthening, transfer entropy
    /// is spiking, and multiple task groups are exhibiting starvation behavior.
    /// The runtime is transitioning to a new regime.
    Transitioning,
    /// The system has stabilized after a transition. Tasks have recovered to a
    /// new baseline. This phase is followed by a return to Nominal once enough
    /// steady-state data accumulates.
    Recovered,
}

impl RuntimePhase {
    /// Human-readable description of the phase.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Nominal => "Runtime is operating normally. No anomalous task patterns detected.",
            Self::PreTransition => {
                "Pre-transition: Some tasks are slowing down. Distribution shift detected. \
                 Consider preemptive intervention."
            }
            Self::Transitioning => {
                "Transitioning: Cascade in progress. Strong correlations and information \
                 flow detected. Intervention recommended."
            }
            Self::Recovered => {
                "Recovered: System has stabilized after a transition. Returning to normal \
                 monitoring."
            }
        }
    }
}

impl std::fmt::Display for RuntimePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nominal => write!(f, "Nominal"),
            Self::PreTransition => write!(f, "PreTransition"),
            Self::Transitioning => write!(f, "Transitioning"),
            Self::Recovered => write!(f, "Recovered"),
        }
    }
}
