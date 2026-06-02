//! Information-theoretic foundations for task intelligence.
//!
//! Provides principled measures of uncertainty, dependence, distribution shift,
//! and information flow, following Shannon information theory.
//!
//! # References
//!
//! - Shannon (1948), "A Mathematical Theory of Communication"
//! - Cover & Thomas (2006), "Elements of Information Theory"
//! - Kullback & Leibler (1951)
//! - Schreiber (2000), "Measuring Information Transfer"
//! - Bandt & Pompe (2002), "Permutation Entropy"

/// Discretize continuous values into `bins` equal-width bins and return histogram counts.
fn histogram(values: &[f64], bins: usize) -> Vec<usize> {
    if values.is_empty() || bins == 0 {
        return vec![];
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        let mut counts = vec![0usize; bins];
        if bins > 0 {
            counts[0] = values.len();
        }
        return counts;
    }

    let width = (max - min) / bins as f64;
    let mut counts = vec![0usize; bins];

    for &v in values {
        let idx = ((v - min) / width).floor() as usize;
        let idx = idx.min(bins - 1);
        counts[idx] += 1;
    }

    counts
}

/// Discretize two variables jointly into a 2D histogram.
fn histogram_2d(x: &[f64], y: &[f64], bins: usize) -> Vec<Vec<usize>> {
    let n = x.len().min(y.len());
    if n == 0 || bins == 0 {
        return vec![];
    }

    let x_min = x[..n].iter().cloned().fold(f64::INFINITY, f64::min);
    let x_max = x[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = y[..n].iter().cloned().fold(f64::INFINITY, f64::min);
    let y_max = y[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let x_width = if (x_max - x_min).abs() < f64::EPSILON {
        1.0
    } else {
        (x_max - x_min) / bins as f64
    };
    let y_width = if (y_max - y_min).abs() < f64::EPSILON {
        1.0
    } else {
        (y_max - y_min) / bins as f64
    };

    let mut counts = vec![vec![0usize; bins]; bins];

    for i in 0..n {
        let xi = if (x_max - x_min).abs() < f64::EPSILON {
            0
        } else {
            (((x[i] - x_min) / x_width).floor() as usize).min(bins - 1)
        };
        let yi = if (y_max - y_min).abs() < f64::EPSILON {
            0
        } else {
            (((y[i] - y_min) / y_width).floor() as usize).min(bins - 1)
        };
        counts[xi][yi] += 1;
    }

    counts
}

/// Compute Shannon entropy of a set of continuous values.
///
/// Discretizes values into `bins` equal-width bins, then computes:
/// `H(X) = -Σ p(x) log₂ p(x)`
///
/// High entropy indicates unpredictability; low entropy indicates regular patterns.
///
/// # Arguments
///
/// * `values` - The continuous values to analyze
/// * `bins` - Number of bins for discretization (typically 10–50)
///
/// # Returns
///
/// Entropy in bits. Returns 0.0 for empty input.
///
/// # Reference
///
/// Shannon (1948), "A Mathematical Theory of Communication"
pub fn entropy(values: &[f64], bins: usize) -> f64 {
    if values.is_empty() || bins == 0 {
        return 0.0;
    }

    let counts = histogram(values, bins);
    let total = values.len() as f64;

    let mut h = 0.0;
    for &c in &counts {
        if c > 0 {
            let p = c as f64 / total;
            h -= p * p.log2();
        }
    }

    h
}

/// Compute joint entropy of two variables.
///
/// `H(X,Y) = -Σ p(x,y) log₂ p(x,y)`
pub fn joint_entropy(x: &[f64], y: &[f64], bins: usize) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 || bins == 0 {
        return 0.0;
    }

    let counts = histogram_2d(x, y, bins);
    let total = n as f64;

    let mut h = 0.0;
    for row in &counts {
        for &c in row {
            if c > 0 {
                let p = c as f64 / total;
                h -= p * p.log2();
            }
        }
    }

    h
}

/// Compute mutual information between two variables.
///
/// `I(X;Y) = H(X) + H(Y) - H(X,Y)`
///
/// Captures non-linear dependencies that Pearson correlation misses.
/// Returns 0 if X and Y are independent; higher values indicate stronger dependence.
///
/// # Arguments
///
/// * `x` - First variable
/// * `y` - Second variable
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// Mutual information in bits (non-negative).
///
/// # Reference
///
/// Cover & Thomas (2006), "Elements of Information Theory"
pub fn mutual_information(x: &[f64], y: &[f64], bins: usize) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 || bins == 0 {
        return 0.0;
    }

    let hx = entropy(x, bins);
    let hy = entropy(y, bins);
    let hxy = joint_entropy(x, y, bins);

    // MI is non-negative; numerical issues can make it slightly negative
    (hx + hy - hxy).max(0.0)
}

/// Compute Kullback-Leibler divergence between two distributions.
///
/// `D_KL(P || Q) = Σ P(i) log(P(i)/Q(i))`
///
/// Measures how much information is lost when Q is used to approximate P.
/// This is the principled measure for distribution shift detection.
///
/// # Arguments
///
/// * `current` - The "current" distribution P
/// * `baseline` - The "baseline" distribution Q
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// KL divergence in bits. Returns `f64::INFINITY` if P has support where Q doesn't.
///
/// # Reference
///
/// Kullback & Leibler (1951)
pub fn kl_divergence(current: &[f64], baseline: &[f64], bins: usize) -> f64 {
    if current.is_empty() || baseline.is_empty() || bins == 0 {
        return 0.0;
    }

    let p_counts = histogram(current, bins);
    let q_counts = histogram(baseline, bins);

    let p_total = current.len() as f64;
    let q_total = baseline.len() as f64;

    let mut kl = 0.0;
    for i in 0..bins {
        let p = p_counts[i] as f64 / p_total;
        let q = q_counts[i] as f64 / q_total;

        if p > 0.0 && q > 0.0 {
            kl += p * (p / q).log2();
        } else if p > 0.0 && q <= 0.0 {
            return f64::INFINITY;
        }
    }

    kl
}

/// Compute Jensen-Shannon divergence between two distributions.
///
/// `JSD(P,Q) = ½ D_KL(P||M) + ½ D_KL(Q||M)` where `M = (P+Q)/2`
///
/// Symmetric, always finite, and its square root is a proper metric.
/// Use for detecting ANY distribution shift (not just mean shift).
///
/// # Arguments
///
/// * `p` - First distribution's values
/// * `q` - Second distribution's values
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// JSD in bits (symmetric, bounded by `log₂(bins)`).
pub fn jsd(p: &[f64], q: &[f64], bins: usize) -> f64 {
    if p.is_empty() || q.is_empty() || bins == 0 {
        return 0.0;
    }

    let p_counts = histogram(p, bins);
    let q_counts = histogram(q, bins);

    let p_total = p.len() as f64;
    let q_total = q.len() as f64;

    let mut jsd_val = 0.0;

    for i in 0..bins {
        let pi = p_counts[i] as f64 / p_total;
        let qi = q_counts[i] as f64 / q_total;
        let mi = (pi + qi) / 2.0;

        if pi > 0.0 && mi > 0.0 {
            jsd_val += 0.5 * pi * (pi / mi).log2();
        }
        if qi > 0.0 && mi > 0.0 {
            jsd_val += 0.5 * qi * (qi / mi).log2();
        }
    }

    jsd_val
}

/// Compute transfer entropy from X to Y.
///
/// `TE(X→Y) = I(Y_{t+1}; X_t | Y_t)`
///
/// Measures whether X's past helps predict Y beyond Y's own past.
/// This detects directional influence (information flow) between metrics.
///
/// # Arguments
///
/// * `x` - Source variable (potential cause)
/// * `y` - Target variable (potential effect)
/// * `lag` - Time lag to consider (typically 1)
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// Transfer entropy in bits. Non-negative; higher values indicate stronger
/// directional information flow from X to Y.
///
/// # Reference
///
/// Schreiber (2000), "Measuring Information Transfer"
pub fn transfer_entropy(x: &[f64], y: &[f64], lag: usize, bins: usize) -> f64 {
    let n = x.len().min(y.len());
    if n <= lag + 1 || bins == 0 {
        return 0.0;
    }

    let m = n - lag;
    let y_future: Vec<f64> = (lag..n).map(|t| y[t]).collect();
    let x_past: Vec<f64> = (0..m).map(|t| x[t]).collect();
    let y_past: Vec<f64> = (0..m).map(|t| y[t]).collect();

    let h_yxy = entropy_3d(&y_future, &x_past, &y_past, bins);
    let h_xy = joint_entropy(&x_past, &y_past, bins);
    let h_yy = joint_entropy(&y_future, &y_past, bins);
    let h_y = entropy(&y_past, bins);

    (h_yxy - h_xy - h_yy + h_y).max(0.0)
}

/// Compute 3D joint entropy for three variables.
fn entropy_3d(a: &[f64], b: &[f64], c: &[f64], bins: usize) -> f64 {
    let n = a.len().min(b.len()).min(c.len());
    if n == 0 || bins == 0 {
        return 0.0;
    }

    // Discretize each variable
    let min_a = a[..n].iter().cloned().fold(f64::INFINITY, f64::min);
    let max_a = a[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_b = b[..n].iter().cloned().fold(f64::INFINITY, f64::min);
    let max_b = b[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_c = c[..n].iter().cloned().fold(f64::INFINITY, f64::min);
    let max_c = c[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let width_a = if (max_a - min_a).abs() < f64::EPSILON {
        1.0
    } else {
        (max_a - min_a) / bins as f64
    };
    let width_b = if (max_b - min_b).abs() < f64::EPSILON {
        1.0
    } else {
        (max_b - min_b) / bins as f64
    };
    let width_c = if (max_c - min_c).abs() < f64::EPSILON {
        1.0
    } else {
        (max_c - min_c) / bins as f64
    };

    // Fast path: for small `bins`, use a flat vec as a 3D grid
    let total_cells = bins * bins * bins;
    let mut counts = vec![0usize; total_cells];

    for i in 0..n {
        let ai = if (max_a - min_a).abs() < f64::EPSILON {
            0
        } else {
            (((a[i] - min_a) / width_a).floor() as usize).min(bins - 1)
        };
        let bi = if (max_b - min_b).abs() < f64::EPSILON {
            0
        } else {
            (((b[i] - min_b) / width_b).floor() as usize).min(bins - 1)
        };
        let ci = if (max_c - min_c).abs() < f64::EPSILON {
            0
        } else {
            (((c[i] - min_c) / width_c).floor() as usize).min(bins - 1)
        };
        let idx = ai * bins * bins + bi * bins + ci;
        counts[idx] += 1;
    }

    let total = n as f64;
    let mut h = 0.0;
    for &count in &counts {
        if count > 0 {
            let p = count as f64 / total;
            h -= p * p.log2();
        }
    }

    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_uniform() {
        // Uniform distribution should have high entropy
        let v: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let h = entropy(&v, 10);
        assert!(h > 3.0, "Expected high entropy for uniform, got {}", h);
        assert!(h <= 3.35, "Entropy shouldn't exceed log2(10)");
    }

    #[test]
    fn test_entropy_constant() {
        // All identical values => zero entropy
        let v = vec![42.0; 50];
        let h = entropy(&v, 10);
        assert!(h.abs() < 1e-10, "Expected zero entropy for constant, got {}", h);
    }

    #[test]
    fn test_mutual_information_identical() {
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..100).map(|i| i as f64 + 3.0).collect();
        let mi = mutual_information(&x, &y, 10);
        assert!(mi > 3.0, "Expected high MI for identical patterns, got {}", mi);
    }

    #[test]
    fn test_mutual_information_independent() {
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..100).map(|_| fastrand::f64() * 100.0).collect();
        let mi = mutual_information(&x, &y, 10);
        assert!(mi < 1.5, "Expected low MI for independent, got {}", mi);
    }

    #[test]
    fn test_jsd_same_distribution() {
        let p: Vec<f64> = (0..100).map(|i| (i % 10) as f64).collect();
        let q: Vec<f64> = (0..100).map(|i| (i % 10) as f64).collect();
        let d = jsd(&p, &q, 10);
        assert!(d < 1e-10, "Expected JSD ~0 for same distribution, got {}", d);
    }

    #[test]
    fn test_jsd_different_distributions() {
        let p: Vec<f64> = (0..100).map(|_| fastrand::f64() * 100.0).collect();
        let q: Vec<f64> = (0..100).map(|_| 500.0 + fastrand::f64() * 100.0).collect();
        let d = jsd(&p, &q, 10);
        assert!(d > 0.0, "Expected JSD > 0 for different distributions, got {}", d);
    }

    #[test]
    fn test_transfer_entropy_bounds() {
        // Test that TE is always non-negative (fundamental property)
        let x: Vec<f64> = (0..100).map(|i| (i % 3) as f64 * 50.0).collect();
        let y: Vec<f64> = (0..100).map(|i| if i > 0 { x[i-1] } else { 0.0 }).collect();
        let te = transfer_entropy(&x, &y, 1, 4);
        assert!(te >= 0.0, "TE should be non-negative, got {}", te);
    }

    #[test]
    fn test_transfer_entropy_independent() {
        // Independent processes → TE ≈ 0
        let x: Vec<f64> = (0..200).map(|_| fastrand::f64() * 100.0).collect();
        let y: Vec<f64> = (0..200).map(|_| fastrand::f64() * 100.0).collect();
        let te = transfer_entropy(&x, &y, 1, 8);
        assert!(te < 2.0, "Expected low TE for independent, got {}", te);
    }

    #[test]
    fn test_transfer_entropy_no_causality() {
        let x: Vec<f64> = (0..200).map(|_| fastrand::f64()).collect();
        let y: Vec<f64> = (0..200).map(|_| fastrand::f64()).collect();
        let te = transfer_entropy(&x, &y, 1, 8);
        assert!(
            te < 1.0,
            "Expected low TE for independent processes, got {}",
            te
        );
    }

    #[test]
    fn test_kl_divergence_identical() {
        let p: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let q: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let kl = kl_divergence(&p, &q, 10);
        assert!(kl < 1e-10, "Expected KL ~0 for same distribution, got {}", kl);
    }

    #[test]
    fn test_permutation_entropy() {
        let h = permutation_entropy(&[0.0, 1.0, 2.0], 3);
        assert!((h - 1.0).abs() > 1e-10, "Expected ~1.0 for sorted, got {}", h);
    }
}

/// Compute permutation entropy for a time series.
///
/// Permutation entropy measures the complexity of a time series by looking at
/// the ordinal patterns of consecutive values. It is robust to noise and
/// invariant to monotonic transformations.
///
/// # Arguments
///
/// * `values` - Time series values.
/// * `order` - Embedding dimension (typical values: 3-7).
///
/// # Returns
///
/// Permutation entropy normalized to [0, 1]. 0 = perfectly regular, 1 = fully random.
///
/// # Reference
///
/// Bandt & Pompe (2002), "Permutation Entropy: A Natural Complexity Measure for
/// Time Series"
pub fn permutation_entropy(values: &[f64], order: usize) -> f64 {
    let n = values.len();
    if n < order || order < 2 {
        return 0.0;
    }

    let n_patterns = n - order + 1;
    let fact = (1..=order).product::<usize>();

    let mut pattern_counts: std::collections::HashMap<Vec<usize>, usize> =
        std::collections::HashMap::new();

    for i in 0..n_patterns {
        let window = &values[i..i + order];
        // Get the ordinal pattern (indices sorted by value)
        let mut indices: Vec<usize> = (0..order).collect();
        indices.sort_by(|&a, &b| window[a].partial_cmp(&window[b]).unwrap_or(std::cmp::Ordering::Equal));
        *pattern_counts.entry(indices).or_insert(0) += 1;
    }

    let total = n_patterns as f64;
    let mut h = 0.0;
    for &count in pattern_counts.values() {
        if count > 0 {
            let p = count as f64 / total;
            h -= p * p.log2();
        }
    }

    // Normalize by log2(factorial(order)) to get [0, 1]
    let max_h = (fact as f64).log2();
    if max_h > 0.0 {
        h / max_h
    } else {
        0.0
    }
}
