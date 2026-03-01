//! STARK prover: generates a proof that a Rescue-Prime hash was computed correctly.

use crate::field::BabyBear;
use crate::poly::DensePolynomial;
use crate::merkle::MerkleTree;
use crate::air::trace::generate_trace;
use crate::air::RescueAir;
use crate::hash::params::*;
use crate::fri::{self, FriDomain};
use crate::stark::transcript::Transcript;
use crate::stark::deep;
use crate::stark::types::*;

/// Generate a STARK proof for a Rescue-Prime hash computation.
pub fn prove(input: [BabyBear; 2], output: [BabyBear; 2]) -> StarkProof {
    let air = RescueAir::new(input, output);
    let trace_len = air.trace_length(); // 8
    let lde_size = LDE_DOMAIN_SIZE; // 64
    let log_trace = trace_len.trailing_zeros(); // 3
    let log_lde = lde_size.trailing_zeros(); // 6

    // --- Step 1: Generate execution trace ---
    let trace = generate_trace(input);

    // --- Step 2: Interpolate trace columns into polynomials ---
    let trace_gen = BabyBear::root_of_unity(log_trace);
    let trace_polys: Vec<DensePolynomial> = trace
        .iter()
        .map(|col| DensePolynomial::from_evaluations(col, trace_gen))
        .collect();

    // --- Step 3: Evaluate on LDE domain (coset) ---
    let lde_gen = BabyBear::root_of_unity(log_lde);
    let lde_offset = BabyBear::new(LDE_COSET_OFFSET);
    let lde_domain = FriDomain::new(lde_size, lde_gen, lde_offset);

    let trace_lde: Vec<Vec<BabyBear>> = trace_polys
        .iter()
        .map(|p| p.evaluate_on_coset(lde_size, lde_gen, lde_offset))
        .collect();

    // --- Step 4: Commit to trace ---
    let trace_leaves: Vec<Vec<u8>> = (0..lde_size)
        .map(|i| {
            let mut leaf = Vec::with_capacity(TRACE_WIDTH * 4);
            for col in 0..TRACE_WIDTH {
                leaf.extend_from_slice(&trace_lde[col][i].to_bytes());
            }
            leaf
        })
        .collect();
    let trace_tree = MerkleTree::new(&trace_leaves);
    let trace_commitment = trace_tree.root();

    // --- Step 5: Fiat-Shamir — derive composition challenge α ---
    let mut transcript = Transcript::new();
    transcript.absorb_commitment(&trace_commitment);
    let alpha = transcript.squeeze_challenge();

    // --- Step 6: Compute quotient polynomial Q(x) ---
    let quotient_poly = compute_quotient_polynomial(
        &trace_polys, &air, alpha, trace_gen, trace_len,
    );

    // Evaluate quotient on LDE domain
    let quotient_lde = quotient_poly.evaluate_on_coset(lde_size, lde_gen, lde_offset);

    // --- Step 7: Commit to quotient ---
    let quotient_leaves: Vec<Vec<u8>> = quotient_lde
        .iter()
        .map(|e| e.to_bytes().to_vec())
        .collect();
    let quotient_tree = MerkleTree::new(&quotient_leaves);
    let quotient_commitment = quotient_tree.root();

    // --- Step 8: Derive OOD point z ---
    // z is derived after the quotient polynomial computation but before the
    // quotient commitment is absorbed, ensuring the prover cannot influence
    // the evaluation point based on the quotient commitment. The quotient
    // commitment is absorbed afterward for deriving subsequent challenges.
    let z = transcript.squeeze_challenge();

    // Absorb quotient commitment for subsequent challenges
    transcript.absorb_commitment(&quotient_commitment);

    // --- Step 9: OOD evaluations ---
    let trace_ood: Vec<BabyBear> = trace_polys.iter().map(|p| p.evaluate(z)).collect();
    let omega = trace_gen;
    let z_omega = z * omega;
    let trace_ood_next: Vec<BabyBear> = trace_polys.iter().map(|p| p.evaluate(z_omega)).collect();
    let quotient_ood = quotient_poly.evaluate(z);

    // --- Step 10: Absorb OOD evaluations ---
    transcript.absorb_field_elements(&trace_ood);
    transcript.absorb_field_elements(&trace_ood_next);
    transcript.absorb_field_element(quotient_ood);

    // --- Step 11: DEEP composition ---
    let deep_alpha = transcript.squeeze_challenge();
    let deep_evals = deep::compute_deep_evaluations(
        &trace_lde,
        &quotient_lde,
        z,
        z_omega,
        &trace_ood,
        &trace_ood_next,
        quotient_ood,
        deep_alpha,
        &lde_domain.points,
    );

    // --- Step 12: FRI ---
    let fri_result = fri::fri_prove(&deep_evals, &lde_domain, &mut transcript);

    // --- Step 13: Query proofs for trace and quotient ---
    let trace_query_proofs: Vec<_> = fri_result.query_positions
        .iter()
        .map(|&pos| trace_tree.open(pos, &trace_leaves[pos]))
        .collect();
    let quotient_query_proofs: Vec<_> = fri_result.query_positions
        .iter()
        .map(|&pos| quotient_tree.open(pos, &quotient_leaves[pos]))
        .collect();

    StarkProof {
        trace_commitment,
        quotient_commitment,
        trace_ood_evals: trace_ood,
        trace_ood_next_evals: trace_ood_next,
        quotient_ood_eval: quotient_ood,
        fri_proof: fri_result.proof,
        trace_query_proofs,
        quotient_query_proofs,
    }
}

/// Compute the quotient polynomial Q(x) = C(x) / Z_H(x).
///
/// C(x) combines transition and boundary constraints.
fn compute_quotient_polynomial(
    trace_polys: &[DensePolynomial],
    air: &RescueAir,
    alpha: BabyBear,
    trace_gen: BabyBear,
    trace_len: usize,
) -> DensePolynomial {
    let mds = mds_matrix();
    let rc_fwd = round_constants_fwd();
    let rc_bwd = round_constants_bwd();

    // --- Transition constraints ---
    // For each round r, compute constraint residuals on the trace domain.
    // constraint[col][row] = next_state_col(ω^row) - expected_col(current_state(ω^row), round)
    let mut constraint_evals = vec![vec![BabyBear::ZERO; trace_len]; STATE_WIDTH];

    for row in 0..NUM_ROUNDS {
        let omega_r = trace_gen.pow(row as u64);

        let mut current = [BabyBear::ZERO; STATE_WIDTH];
        let mut next = [BabyBear::ZERO; STATE_WIDTH];
        for col in 0..STATE_WIDTH {
            current[col] = trace_polys[col].evaluate(omega_r);
            next[col] = trace_polys[col].evaluate(omega_r * trace_gen);
        }

        // Compute expected next state
        let mut expected = current;
        // Forward half: S-box, MDS, RC
        for i in 0..STATE_WIDTH {
            expected[i] = expected[i].pow7();
        }
        let mut temp = [BabyBear::ZERO; STATE_WIDTH];
        for i in 0..STATE_WIDTH {
            for k in 0..STATE_WIDTH {
                temp[i] = temp[i] + mds[i][k] * expected[k];
            }
        }
        for i in 0..STATE_WIDTH {
            expected[i] = temp[i] + rc_fwd[row][i];
        }
        // Backward half: inv S-box, MDS, RC
        for i in 0..STATE_WIDTH {
            expected[i] = expected[i].pow(crate::hash::params::ALPHA_INV);
        }
        let mut temp2 = [BabyBear::ZERO; STATE_WIDTH];
        for i in 0..STATE_WIDTH {
            for k in 0..STATE_WIDTH {
                temp2[i] = temp2[i] + mds[i][k] * expected[k];
            }
        }
        for i in 0..STATE_WIDTH {
            expected[i] = temp2[i] + rc_bwd[row][i];
        }

        for col in 0..STATE_WIDTH {
            constraint_evals[col][row] = next[col] - expected[col];
        }
    }
    // Row NUM_ROUNDS (= 7, the last row): no transition constraint, leave as zero

    // Combine transition constraints with powers of alpha
    let mut combined_evals = vec![BabyBear::ZERO; trace_len];
    let mut alpha_pow = BabyBear::ONE;
    for col in 0..STATE_WIDTH {
        for row in 0..trace_len {
            combined_evals[row] = combined_evals[row] + alpha_pow * constraint_evals[col][row];
        }
        alpha_pow = alpha_pow * alpha;
    }

    // Interpolate combined constraint polynomial
    let combined_poly = DensePolynomial::from_evaluations(&combined_evals, trace_gen);

    // Divide by vanishing polynomial Z_H(x) = x^n - 1
    let z_h = crate::poly::vanishing_poly(trace_len);
    let transition_quotient = if combined_poly.is_zero() {
        DensePolynomial::zero()
    } else {
        let (q, r) = combined_poly.div_rem(&z_h);
        assert!(r.is_zero(), "Transition constraints not divisible by Z_H — invalid trace");
        q
    };

    // --- Boundary constraints ---
    let mut boundary_quotient = DensePolynomial::zero();
    for (row, col, expected_val) in air.boundary_constraints() {
        let point = trace_gen.pow(row as u64);
        let numerator = trace_polys[col].sub(&DensePolynomial::constant(expected_val));
        let denom = DensePolynomial::new(vec![-point, BabyBear::ONE]);
        let (q, r) = numerator.div_rem(&denom);
        assert!(
            r.is_zero(),
            "Boundary constraint not divisible at row={}, col={}",
            row, col
        );
        boundary_quotient = boundary_quotient.add(&q.scale(alpha_pow));
        alpha_pow = alpha_pow * alpha;
    }

    transition_quotient.add(&boundary_quotient)
}
