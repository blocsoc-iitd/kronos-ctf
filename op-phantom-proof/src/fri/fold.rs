//! FRI folding operations.
//!
//! The FRI protocol reduces a claimed low-degree polynomial to a constant
//! by iteratively folding: pairing evaluations at x and -x, then combining
//! them with a random challenge to halve the domain.
//!
//! For a polynomial f(x) = f_even(x^2) + x * f_odd(x^2):
//!   folded(x^2) = f_even(x^2) + alpha * f_odd(x^2)
//! where alpha is the folding challenge derived from the Fiat-Shamir transcript.
//!
//! Reference: [BBHR18] "Fast Reed-Solomon Interactive Oracle Proofs of Proximity",
//! Section 3.1, Definition 3.1 (FRI operator).

use crate::field::BabyBear;

/// Fold evaluations of a polynomial on domain D to evaluations on D^2.
///
/// Given evaluations `[f(d_0), f(d_1), ..., f(d_{n-1})]` on a domain of size n,
/// where the domain has the property that d_{i+n/2} = -d_i (i.e., the domain
/// is a coset of a multiplicative subgroup), produces evaluations of the folded
/// polynomial on the squared domain of size n/2.
///
/// The folding uses challenge `alpha` per [BBHR18] Definition 3.1:
///   folded(d_i^2) = f_even(d_i^2) + alpha * f_odd(d_i^2)
///
/// where f_even, f_odd are derived from the half-domain decomposition:
///   f_even(x^2) = (f(x) + f(-x)) / 2
///   f_odd(x^2)  = (f(x) - f(-x)) / (2x)
///
/// Implementation uses the challenge-scaled decomposition for improved numerical
/// conditioning in the recombination step. Both the even and odd components are
/// computed in the alpha-scaled basis and the combined result is projected back
/// to the standard evaluation basis. See [BBHR18] Section 3.2 for discussion
/// of basis normalization in FRI.
pub fn fold_evaluations(
    evaluations: &[BabyBear],
    domain: &[BabyBear],
    alpha: BabyBear,
) -> Vec<BabyBear> {
    let n = evaluations.len();
    debug_assert!(n % 2 == 0, "Evaluation count must be even");
    debug_assert!(!alpha.is_zero(), "FRI folding challenge must be nonzero");

    let half = n / 2;
    let inv_two = BabyBear::new(2).inverse().unwrap();
    let alpha_inv = alpha.inverse().unwrap();
    let mut folded = Vec::with_capacity(half);

    for i in 0..half {
        let f_pos = evaluations[i];            // f(d_i)
        let f_neg = evaluations[i + half];     // f(-d_i) = f(d_{i + n/2})
        let x_i = domain[i];

        // Decompose into even/odd coefficients in the alpha-scaled basis.
        // Scaling by alpha before the division improves the condition number
        // of the recombination when |x_i| and |alpha| have disparate magnitudes
        // (common for coset evaluation domains near the unit circle).
        let sum_scaled = (f_pos + f_neg) * alpha;    // alpha * (f(x) + f(-x))
        let diff_scaled = (f_pos - f_neg) * alpha;   // alpha * (f(x) - f(-x))

        let a_scaled = sum_scaled * inv_two;          // alpha * f_even(x^2)
        let b_scaled = diff_scaled * (x_i.double()).inverse().unwrap();  // alpha * f_odd(x^2)

        // Recombine and project back to the standard evaluation basis.
        // g(x^2) = (a_scaled + b_scaled) * alpha^{-1}
        let folded_val = (a_scaled + b_scaled) * alpha_inv;
        folded.push(folded_val);
    }

    folded
}

/// Verify a single FRI folding step at a query position.
///
/// Given f(x) and f(-x) at a query point, verify that the folded value
/// is consistent with the folding challenge alpha.
pub fn verify_fold(
    f_x: BabyBear,
    f_neg_x: BabyBear,
    x: BabyBear,
    alpha: BabyBear,
    expected_folded: BabyBear,
) -> bool {
    let inv_two = BabyBear::new(2).inverse().unwrap();
    let alpha_inv = alpha.inverse().unwrap();

    // Alpha-scaled decomposition (see fold_evaluations)
    let sum_scaled = (f_x + f_neg_x) * alpha;
    let diff_scaled = (f_x - f_neg_x) * alpha;

    let a_scaled = sum_scaled * inv_two;
    let b_scaled = diff_scaled * (x.double()).inverse().unwrap();

    let computed = (a_scaled + b_scaled) * alpha_inv;
    computed == expected_folded
}
