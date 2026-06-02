use std::collections::HashSet;

use crate::phase::RuntimePhase;

/// A human-readable intelligence report about a task pool's execution patterns.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskIntelligenceReport {
    /// Name/label of the task pool.
    pub pool_name: String,
    /// Total number of unique task labels observed.
    pub total_tasks: usize,
    /// Pairs of task labels that show high mutual information (≥ threshold).
    pub correlated_pairs: Vec<(String, String, f64)>,
    /// Triples of (cause, effect, transfer_entropy) where the cause appears to
    /// be starving or otherwise influencing the effect task.
    pub starvation_pairs: Vec<(String, String, f64)>,
    /// The detected runtime phase based on distribution analysis.
    pub phase: RuntimePhase,
    /// Optional details explaining the phase classification.
    pub phase_details: String,
}

impl TaskIntelligenceReport {
    /// Render the report as a concise, human-readable summary string,
    /// matching the style: "Task pool \"X\" has N tasks. K are correlated (MI > 0.8).
    /// M is causing starvation in L others (TE > 0.6). Runtime phase: P."
    pub fn summary(&self) -> String {
        let correlated_count = self.correlated_pairs.len();
        // Count unique "cause" tasks in starvation chains
        let unique_causes: HashSet<&str> = self
            .starvation_pairs
            .iter()
            .map(|(cause, _, _)| cause.as_str())
            .collect();
        let unique_effects: HashSet<&str> = self
            .starvation_pairs
            .iter()
            .map(|(_, effect, _)| effect.as_str())
            .collect();

        format!(
            "Task pool \"{}\" has {} tasks. {} {} correlated (MI > 0.8). \
             {} {} causing starvation in {} others (TE > 0.6). \
             Runtime phase: {}.",
            self.pool_name,
            self.total_tasks,
            correlated_count,
            if correlated_count == 1 { "is" } else { "are" },
            unique_causes.len(),
            if unique_causes.len() == 1 { "is" } else { "are" },
            unique_effects.len(),
            self.phase,
        )
    }

    /// A more detailed version of the report.
    pub fn detailed(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "=== Task Intelligence Report: \"{}\" ===\n",
            self.pool_name
        ));
        out.push_str(&format!("Total task variants: {}\n", self.total_tasks));
        out.push_str(&format!("Runtime phase: {}\n", self.phase));
        out.push_str(&format!("Phase details: {}\n\n", self.phase_details));

        if self.correlated_pairs.is_empty() {
            out.push_str("No correlated task pairs detected.\n");
        } else {
            out.push_str("Correlated task pairs (MI > 0.8):\n");
            for (a, b, mi) in &self.correlated_pairs {
                out.push_str(&format!("  {} ↔ {} (MI = {:.3})\n", a, b, mi));
            }
        }

        out.push('\n');

        if self.starvation_pairs.is_empty() {
            out.push_str("No starvation cascades detected.\n");
        } else {
            out.push_str("Starvation cascades (TE > 0.6):\n");
            for (cause, effect, te) in &self.starvation_pairs {
                out.push_str(&format!(
                    "  {} → {} (TE = {:.3}) — {} may be starving {}\n",
                    cause, effect, te, cause, effect
                ));
            }
        }

        out.push_str("\n=== End Report ===\n");
        out
    }
}

impl std::fmt::Display for TaskIntelligenceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}
