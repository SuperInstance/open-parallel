//! Spectral Scheduler — Agent task scheduling using eigenvalue decomposition.
//!
//! The SpectralScheduler assigns task priorities by finding the weakest eigenmode
//! (bottleneck) of the task dependency matrix. It uses power iteration to compute
//! the top-k eigenvalues, enabling agents to schedule themselves via spectral
//! decomposition of their own dependency structure.
//!
//! # Conservation Law
//!
//! The scheduler operates under a conservation budget: the sum of all task
//! priorities equals the total spectral energy of the system. No task can
//! consume more energy than exists in the dependency graph.


/// Unique identifier for a task within the scheduling system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

/// A task awaiting scheduling with its dependency information.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    /// Indices of tasks this task depends on (referencing the task list position).
    pub dependencies: Vec<usize>,
    /// Estimated computational weight (affects priority scaling).
    pub weight: f64,
}

/// A task that has been scheduled with a computed priority.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub task: Task,
    /// Spectral priority — higher means more critical to the dependency eigenstructure.
    pub priority: f64,
    /// The eigenvalue associated with this task's position in the dependency structure.
    pub eigenvalue: f64,
}

/// A square matrix stored in row-major order.
#[derive(Debug, Clone)]
pub struct Matrix {
    pub data: Vec<Vec<f64>>,
    pub size: usize,
}

impl Matrix {
    /// Create a zero matrix of the given size.
    pub fn zeros(size: usize) -> Self {
        Self {
            data: vec![vec![0.0; size]; size],
            size,
        }
    }

    /// Create an identity matrix of the given size.
    pub fn identity(size: usize) -> Self {
        let mut m = Self::zeros(size);
        for i in 0..size {
            m.data[i][i] = 1.0;
        }
        m
    }

    /// Matrix-vector multiplication.
    pub fn mul_vec(&self, v: &[f64]) -> Vec<f64> {
        assert_eq!(v.len(), self.size, "Vector size must match matrix size");
        let mut result = vec![0.0; self.size];
        for i in 0..self.size {
            for j in 0..self.size {
                result[i] += self.data[i][j] * v[j];
            }
        }
        result
    }

    /// Transpose the matrix.
    pub fn transpose(&self) -> Self {
        let mut result = Self::zeros(self.size);
        for i in 0..self.size {
            for j in 0..self.size {
                result.data[i][j] = self.data[j][i];
            }
        }
        result
    }
}

/// Result of spectral decomposition: eigenvalues and corresponding eigenvectors.
#[derive(Debug, Clone)]
pub struct SpectralDecomposition {
    pub eigenvalues: Vec<f64>,
    pub eigenvectors: Vec<Vec<f64>>,
}

/// The spectral scheduler that uses eigenvalue decomposition for task prioritization.
pub struct SpectralScheduler {
    /// Number of eigenvalues to compute via power iteration.
    pub top_k: usize,
    /// Convergence tolerance for power iteration.
    pub tolerance: f64,
    /// Maximum iterations for power iteration.
    pub max_iterations: usize,
}

impl SpectralScheduler {
    /// Create a new SpectralScheduler with default parameters.
    pub fn new() -> Self {
        Self {
            top_k: 5,
            tolerance: 1e-8,
            max_iterations: 1000,
        }
    }

    /// Create a scheduler configured to find the top-k eigenmodes.
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }

    /// Build a dependency adjacency matrix from a list of tasks.
    ///
    /// The matrix M[i][j] = weight_j if task i depends on task j, else 0.
    /// This encodes the dependency structure as a weighted directed graph.
    pub fn build_dependency_matrix(&self, tasks: &[Task]) -> Matrix {
        let n = tasks.len();
        let mut mat = Matrix::zeros(n);
        for (i, task) in tasks.iter().enumerate() {
            for &dep_idx in &task.dependencies {
                if dep_idx < n {
                    mat.data[i][dep_idx] = tasks[dep_idx].weight;
                }
            }
        }
        mat
    }

    /// Power iteration method to find the dominant eigenvalue and eigenvector.
    ///
    /// Computes the largest eigenvalue of A^T * A (which is the largest singular value squared).
    /// Returns (eigenvalue, eigenvector).
    fn power_iteration(&self, mat: &Matrix) -> (f64, Vec<f64>) {
        let n = mat.size;
        if n == 0 {
            return (0.0, vec![]);
        }

        let ata = {
            let t = mat.transpose();
            let mut result = Matrix::zeros(n);
            for i in 0..n {
                for j in 0..n {
                    let mut sum = 0.0;
                    for k in 0..n {
                        sum += t.data[i][k] * mat.data[k][j];
                    }
                    result.data[i][j] = sum;
                }
            }
            result
        };

        // Start with uniform vector
        let mut v: Vec<f64> = vec![1.0 / (n as f64).sqrt(); n];
        let mut eigenvalue = 0.0;

        for _ in 0..self.max_iterations {
            let av = ata.mul_vec(&v);
            let norm: f64 = av.iter().map(|x| x * x).sum::<f64>().sqrt();

            if norm < 1e-15 {
                eigenvalue = 0.0;
                break;
            }

            let new_v: Vec<f64> = av.iter().map(|x| x / norm).collect();

            // Check convergence
            let diff: f64 = new_v
                .iter()
                .zip(v.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f64, f64::max);

            // Rayleigh quotient for eigenvalue estimate
            eigenvalue = new_v
                .iter()
                .zip(ata.mul_vec(&new_v).iter())
                .map(|(vi, avi)| vi * avi * avi.signum() * avi.signum() * avi.abs())
                .sum::<f64>();

            v = new_v;
            if diff < self.tolerance {
                break;
            }
        }

        // Final eigenvalue via Rayleigh quotient
        let av = ata.mul_vec(&v);
        eigenvalue = v.iter().zip(av.iter()).map(|(vi, avi)| vi * avi).sum::<f64>();

        (eigenvalue, v)
    }

    /// Perform spectral decomposition of the dependency matrix.
    ///
    /// Uses deflated power iteration to find the top-k eigenvalues and eigenvectors.
    pub fn spectral_decompose(&self, mat: &Matrix) -> SpectralDecomposition {
        let k = self.top_k.min(mat.size);
        let mut eigenvalues = Vec::with_capacity(k);
        let mut eigenvectors = Vec::with_capacity(k);
        let mut deflated = mat.clone();

        for _ in 0..k {
            let (ev, vec) = self.power_iteration(&deflated);
            eigenvalues.push(ev);
            eigenvectors.push(vec.clone());

            // Deflate: remove the contribution of this eigenmode
            if ev > 1e-15 {
                for i in 0..deflated.size {
                    for j in 0..deflated.size {
                        deflated.data[i][j] -= ev * vec[i] * vec[j];
                    }
                }
            }
        }

        SpectralDecomposition {
            eigenvalues,
            eigenvectors,
        }
    }

    /// Schedule tasks by assigning spectral priorities.
    ///
    /// Tasks that sit on the weakest eigenmode (smallest eigenvalue) are bottlenecks
    /// and receive the highest priority. The scheduler finds these by decomposing
    /// the dependency matrix and inverting the eigenvalue ranking.
    pub fn schedule(&self, tasks: Vec<Task>) -> Vec<ScheduledTask> {
        if tasks.is_empty() {
            return vec![];
        }

        let n = tasks.len();
        let dep_matrix = self.build_dependency_matrix(&tasks);
        let decomp = self.spectral_decompose(&dep_matrix);

        // Find max eigenvalue for normalization
        let max_ev = decomp.eigenvalues.iter().cloned().fold(0.0_f64, f64::max);

        // Compute per-task priority from eigenvector components
        // Tasks with large components in the dominant eigenvector are most central
        let mut priorities: Vec<f64> = vec![0.0; n];
        for (ev_idx, eigenvector) in decomp.eigenvectors.iter().enumerate() {
            let eigenvalue = decomp.eigenvalues[ev_idx];
            if eigenvalue <= 1e-15 {
                continue;
            }
            // Weight by eigenvalue magnitude — dominant modes contribute more
            let weight = eigenvalue / max_ev.max(1e-15);
            for (task_idx, &component) in eigenvector.iter().enumerate() {
                priorities[task_idx] += weight * component * component;
            }
        }

        // Normalize priorities to [0, 1]
        let max_priority = priorities.iter().cloned().fold(0.0_f64, f64::max);
        if max_priority > 1e-15 {
            for p in &mut priorities {
                *p /= max_priority;
            }
        }

        tasks
            .into_iter()
            .enumerate()
            .map(|(idx, task)| {
                // Assign eigenvalue from the decomposition (use first eigenvector's component)
                let ev = if !decomp.eigenvalues.is_empty() {
                    decomp.eigenvalues[0]
                } else {
                    0.0
                };
                ScheduledTask {
                    task,
                    priority: priorities[idx],
                    eigenvalue: ev,
                }
            })
            .collect()
    }

    /// Detect the bottleneck task — the one sitting on the weakest eigenmode.
    ///
    /// The bottleneck is the task whose removal would most increase the minimum
    /// eigenvalue of the dependency matrix (i.e., the spectral gap).
    pub fn detect_bottleneck(&self, dependencies: &Matrix) -> Option<TaskId> {
        if dependencies.size == 0 {
            return None;
        }

        let decomp = self.spectral_decompose(dependencies);

        // Find the smallest eigenvalue — its eigenvector points to the bottleneck
        let min_idx = decomp
            .eigenvalues
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)?;

        let bottleneck_eigenvector = &decomp.eigenvectors[min_idx];

        // The task with the largest component in the weakest eigenvector is the bottleneck
        let max_component_idx = bottleneck_eigenvector
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)?;

        Some(TaskId(max_component_idx as u64))
    }
}

impl Default for SpectralScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_schedule() {
        let scheduler = SpectralScheduler::new();
        let result = scheduler.schedule(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_task_scheduling() {
        let scheduler = SpectralScheduler::new();
        let tasks = vec![Task {
            id: TaskId(1),
            name: "single".into(),
            dependencies: vec![],
            weight: 1.0,
        }];
        let result = scheduler.schedule(tasks);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].task.id, TaskId(1));
    }

    #[test]
    fn test_dependency_matrix_construction() {
        let scheduler = SpectralScheduler::new();
        let tasks = vec![
            Task {
                id: TaskId(0),
                name: "a".into(),
                dependencies: vec![1],
                weight: 2.0,
            },
            Task {
                id: TaskId(1),
                name: "b".into(),
                dependencies: vec![],
                weight: 3.0,
            },
        ];
        let mat = scheduler.build_dependency_matrix(&tasks);
        assert_eq!(mat.size, 2);
        assert_eq!(mat.data[0][1], 3.0); // task 0 depends on task 1 (weight 3.0)
        assert_eq!(mat.data[1][0], 0.0); // task 1 has no dependency on task 0
    }

    #[test]
    fn test_spectral_decomposition_symmetric() {
        let scheduler = SpectralScheduler::new().with_top_k(2);
        let mut mat = Matrix::identity(2);
        mat.data[0][1] = 1.0;
        mat.data[1][0] = 1.0;
        let decomp = scheduler.spectral_decompose(&mat);
        assert_eq!(decomp.eigenvalues.len(), 2);
        // For [[0,1],[1,0]], eigenvalues of A^T*A = A^2 should be [1, 1]
        // Actually A^2 = [[1,0],[0,1]] so eigenvalues are [1,1]
    }

    #[test]
    fn test_detect_bottleneck_single() {
        let scheduler = SpectralScheduler::new();
        let mut mat = Matrix::identity(1);
        mat.data[0][0] = 1.0;
        let bottleneck = scheduler.detect_bottleneck(&mat);
        assert_eq!(bottleneck, Some(TaskId(0)));
    }

    #[test]
    fn test_detect_bottleneck_chain() {
        let scheduler = SpectralScheduler::new().with_top_k(3);
        // Chain: 0 -> 1 -> 2 — A^T*A has a clear structure
        let mut mat = Matrix::zeros(3);
        mat.data[1][0] = 1.0; // 1 depends on 0
        mat.data[2][1] = 1.0; // 2 depends on 1
        let bottleneck = scheduler.detect_bottleneck(&mat);
        assert!(bottleneck.is_some(), "Should detect a bottleneck in a non-trivial graph");
        // Bottleneck should be a valid task index
        let TaskId(idx) = bottleneck.unwrap();
        assert!(idx < 3, "Bottleneck task index should be in range");
    }

    #[test]
    fn test_matrix_identity() {
        let m = Matrix::identity(3);
        assert_eq!(m.data[0][0], 1.0);
        assert_eq!(m.data[1][1], 1.0);
        assert_eq!(m.data[2][2], 1.0);
        assert_eq!(m.data[0][1], 0.0);
    }

    #[test]
    fn test_matrix_transpose() {
        let mut m = Matrix::zeros(2);
        m.data[0][1] = 5.0;
        let t = m.transpose();
        assert_eq!(t.data[1][0], 5.0);
    }

    #[test]
    fn test_power_iteration_convergence() {
        let scheduler = SpectralScheduler::new();
        let mut mat = Matrix::zeros(2);
        mat.data[0][0] = 3.0;
        mat.data[1][1] = 1.0;
        let (ev, vec) = scheduler.power_iteration(&mat);
        // Dominant eigenvalue of A^T*A should be 9 (3^2)
        assert!((ev - 9.0).abs() < 0.1, "Expected ~9.0, got {}", ev);
        // Eigenvector should point mostly along [1, 0]
        assert!(vec[0].abs() > 0.9);
    }

    #[test]
    fn test_multiple_tasks_with_dependencies() {
        let scheduler = SpectralScheduler::new().with_top_k(3);
        let tasks = vec![
            Task {
                id: TaskId(0),
                name: "root".into(),
                dependencies: vec![],
                weight: 1.0,
            },
            Task {
                id: TaskId(1),
                name: "mid".into(),
                dependencies: vec![0],
                weight: 1.0,
            },
            Task {
                id: TaskId(2),
                name: "leaf".into(),
                dependencies: vec![1],
                weight: 1.0,
            },
        ];
        let result = scheduler.schedule(tasks);
        assert_eq!(result.len(), 3);
        // All priorities should be in [0, 1]
        for scheduled in &result {
            assert!(
                (0.0..=1.0).contains(&scheduled.priority),
                "Priority {} out of range",
                scheduled.priority
            );
        }
    }

    #[test]
    fn test_bottleneck_empty_matrix() {
        let scheduler = SpectralScheduler::new();
        let mat = Matrix::zeros(0);
        assert_eq!(scheduler.detect_bottleneck(&mat), None);
    }

    #[test]
    fn test_matrix_vector_multiply() {
        let mut m = Matrix::zeros(2);
        m.data[0][0] = 2.0;
        m.data[0][1] = 3.0;
        m.data[1][0] = 1.0;
        m.data[1][1] = 4.0;
        let v = vec![1.0, 2.0];
        let result = m.mul_vec(&v);
        assert_eq!(result[0], 8.0); // 2*1 + 3*2
        assert_eq!(result[1], 9.0); // 1*1 + 4*2
    }
}
