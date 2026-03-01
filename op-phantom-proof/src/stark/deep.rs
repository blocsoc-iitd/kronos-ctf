//! DEEP (Domain Extension for Eliminating Pretenders) composition polynomial.
//!
//! The DEEP technique evaluates the trace polynomials and quotient polynomial
//! at an out-of-domain (OOD) point z, then constructs a composition polynomial
//! that combines these evaluations with the LDE. This allows FRI to verify
//! all constraints with a single low-degree test.
//!
//! The DEEP composition polynomial is:
//!   D(x) = Σ_i α^i * (T_i(x) - T_i(z)) / (x - z)
//!        + Σ_i α^{k+i} * (T_i(x) - T_i(z·ω)) / (x - z·ω)
//!        + α^{2k} * (Q(x) - Q(z)) / (x - z)
//!
//! where T_i are trace polynomials, Q is the quotient polynomial, and
//! the α^i are powers of the composition challenge.

use crate::field::BabyBear;

/// Compute DEEP composition evaluations on the LDE domain.
///
/// Given:
/// - `trace_lde[col][point]`: trace polynomial evaluations on LDE domain
/// - `quotient_lde[point]`: quotient polynomial evaluations on LDE domain
/// - `z`: OOD evaluation point
/// - `z_omega`: z * ω (for next-row evaluations)
/// - `trace_ood[col]`: T_col(z) values
/// - `trace_ood_next[col]`: T_col(z * ω) values
/// - `quotient_ood`: Q(z)
/// - `alpha`: composition challenge
/// - `lde_domain[point]`: the actual domain points
///
/// Returns the DEEP composition evaluations on the LDE domain.
pub fn compute_deep_evaluations(
    trace_lde: &[Vec<BabyBear>],
    quotient_lde: &[BabyBear],
    z: BabyBear,
    z_omega: BabyBear,
    trace_ood: &[BabyBear],
    trace_ood_next: &[BabyBear],
    quotient_ood: BabyBear,
    alpha: BabyBear,
    lde_domain: &[BabyBear],
) -> Vec<BabyBear> {
    let domain_size = lde_domain.len();
    let num_cols = trace_lde.len();

    let mut deep_evals = vec![BabyBear::ZERO; domain_size];
    let mut alpha_pow = BabyBear::ONE;

    // For each trace column: α^i * (T_i(x) - T_i(z)) / (x - z)
    for col in 0..num_cols {
        for j in 0..domain_size {
            let x = lde_domain[j];
            let numerator = trace_lde[col][j] - trace_ood[col];
            let denominator = x - z;
            // denominator is nonzero because z is not in the LDE domain
            // (z is an OOD point)
            deep_evals[j] = deep_evals[j] + alpha_pow * numerator * denominator.inverse().unwrap();
        }
        alpha_pow = alpha_pow * alpha;
    }

    // For each trace column: α^{k+i} * (T_i(x) - T_i(z·ω)) / (x - z·ω)
    for col in 0..num_cols {
        for j in 0..domain_size {
            let x = lde_domain[j];
            let numerator = trace_lde[col][j] - trace_ood_next[col];
            let denominator = x - z_omega;
            deep_evals[j] = deep_evals[j] + alpha_pow * numerator * denominator.inverse().unwrap();
        }
        alpha_pow = alpha_pow * alpha;
    }

    // Quotient term: α^{2k} * (Q(x) - Q(z)) / (x - z)
    for j in 0..domain_size {
        let x = lde_domain[j];
        let numerator = quotient_lde[j] - quotient_ood;
        let denominator = x - z;
        deep_evals[j] = deep_evals[j] + alpha_pow * numerator * denominator.inverse().unwrap();
    }

    deep_evals
}

/// Verify DEEP at a single point.
///
/// Given a query position, check that the DEEP evaluation is consistent
/// with the provided trace and quotient evaluations.
pub fn verify_deep_at_point(
    trace_evals_at_x: &[BabyBear],
    quotient_eval_at_x: BabyBear,
    x: BabyBear,
    z: BabyBear,
    z_omega: BabyBear,
    trace_ood: &[BabyBear],
    trace_ood_next: &[BabyBear],
    quotient_ood: BabyBear,
    alpha: BabyBear,
) -> BabyBear {
    let num_cols = trace_evals_at_x.len();
    let mut result = BabyBear::ZERO;
    let mut alpha_pow = BabyBear::ONE;

    let x_minus_z_inv = (x - z).inverse().unwrap();
    let x_minus_z_omega_inv = (x - z_omega).inverse().unwrap();

    // Trace terms at z
    for col in 0..num_cols {
        let numerator = trace_evals_at_x[col] - trace_ood[col];
        result = result + alpha_pow * numerator * x_minus_z_inv;
        alpha_pow = alpha_pow * alpha;
    }

    // Trace terms at z*ω
    for col in 0..num_cols {
        let numerator = trace_evals_at_x[col] - trace_ood_next[col];
        result = result + alpha_pow * numerator * x_minus_z_omega_inv;
        alpha_pow = alpha_pow * alpha;
    }

    // Quotient term
    let numerator = quotient_eval_at_x - quotient_ood;
    result = result + alpha_pow * numerator * x_minus_z_inv;

    result
}
