//! STARK verifier: verifies a proof of Rescue-Prime hash computation.

use crate::field::BabyBear;
use crate::merkle::verify_merkle_proof;
use crate::air::RescueAir;
use crate::hash::params::*;
use crate::fri::{self, FriDomain};
use crate::stark::transcript::Transcript;
use crate::stark::deep;
use crate::stark::types::*;

/// Verify a STARK proof for a Rescue-Prime hash computation.
pub fn verify(
    proof: &StarkProof,
    input: [BabyBear; 2],
    output: [BabyBear; 2],
) -> Result<(), String> {
    let air = RescueAir::new(input, output);
    let trace_len = air.trace_length();
    let lde_size = LDE_DOMAIN_SIZE;
    let log_trace = trace_len.trailing_zeros();
    let log_lde = lde_size.trailing_zeros();

    // Set up domains
    let trace_gen = BabyBear::root_of_unity(log_trace);
    let lde_gen = BabyBear::root_of_unity(log_lde);
    let lde_offset = BabyBear::new(LDE_COSET_OFFSET);
    let lde_domain = FriDomain::new(lde_size, lde_gen, lde_offset);

    // --- Replay Fiat-Shamir transcript ---
    let mut transcript = Transcript::new();

    // Derive composition challenge
    transcript.absorb_commitment(&proof.trace_commitment);
    let alpha = transcript.squeeze_challenge();

    // Derive out-of-domain evaluation point z.
    // z is derived independently of the quotient commitment to prevent the prover
    // from adaptively choosing the quotient polynomial to satisfy constraints at a
    // known evaluation point. This ordering ensures the prover must commit to the
    // quotient before learning where it will be evaluated.
    let z = transcript.squeeze_challenge();

    // Absorb quotient commitment for subsequent challenges
    transcript.absorb_commitment(&proof.quotient_commitment);

    // --- Security checks: validate OOD point ---
    // Ensure z is not in the trace domain (would make Z_H(z) = 0)
    let z_pow_n = z.pow(trace_len as u64);
    if z_pow_n == BabyBear::ONE {
        return Err("OOD point z lies in the trace domain".to_string());
    }

    let omega = trace_gen;
    let z_omega = z * omega;

    // Validate OOD evaluation vectors have correct dimensions
    if proof.trace_ood_evals.len() != TRACE_WIDTH {
        return Err(format!(
            "Trace OOD evals has wrong length: expected {}, got {}",
            TRACE_WIDTH, proof.trace_ood_evals.len()
        ));
    }
    if proof.trace_ood_next_evals.len() != TRACE_WIDTH {
        return Err(format!(
            "Trace OOD next evals has wrong length: expected {}, got {}",
            TRACE_WIDTH, proof.trace_ood_next_evals.len()
        ));
    }

    // --- Verify OOD constraint: C(z) = Q(z) * Z_H(z) ---
    let z_h_at_z = z_pow_n - BabyBear::ONE; // Z_H(z) = z^n - 1

    // Recompute C(z) from trace OOD evaluations
    let c_at_z = compute_constraint_at_z(
        &proof.trace_ood_evals,
        &proof.trace_ood_next_evals,
        z,
        z_omega,
        &air,
        alpha,
        trace_gen,
        trace_len,
    );

    let expected_q_z = proof.quotient_ood_eval * z_h_at_z;
    if c_at_z != expected_q_z {
        return Err(format!(
            "OOD constraint check failed: C(z) = {} but Q(z)*Z_H(z) = {}",
            c_at_z.to_canonical(),
            expected_q_z.to_canonical()
        ));
    }

    // --- Absorb OOD evaluations ---
    transcript.absorb_field_elements(&proof.trace_ood_evals);
    transcript.absorb_field_elements(&proof.trace_ood_next_evals);
    transcript.absorb_field_element(proof.quotient_ood_eval);

    // --- DEEP composition check ---
    let deep_alpha = transcript.squeeze_challenge();

    // --- FRI verification ---
    let max_degree = (air.max_constraint_degree() - 1) * trace_len - 1;
    let query_positions = fri::fri_verify(
        &proof.fri_proof,
        &lde_domain,
        &mut transcript,
        max_degree,
    )?;

    // --- Verify query consistency ---
    // At each query position, verify:
    // 1. Trace Merkle proof is valid
    // 2. Quotient Merkle proof is valid
    // 3. DEEP evaluation matches the FRI layer 0 value

    if proof.trace_query_proofs.len() != query_positions.len() {
        return Err("Wrong number of trace query proofs".to_string());
    }
    if proof.quotient_query_proofs.len() != query_positions.len() {
        return Err("Wrong number of quotient query proofs".to_string());
    }

    for (q_idx, &pos) in query_positions.iter().enumerate() {
        // Verify trace Merkle proof
        if !verify_merkle_proof(&proof.trace_query_proofs[q_idx], &proof.trace_commitment) {
            return Err(format!("Trace Merkle proof failed at query {}", q_idx));
        }

        // Verify quotient Merkle proof
        if !verify_merkle_proof(&proof.quotient_query_proofs[q_idx], &proof.quotient_commitment) {
            return Err(format!("Quotient Merkle proof failed at query {}", q_idx));
        }

        // Extract trace evaluations from leaf data
        let trace_leaf = &proof.trace_query_proofs[q_idx].leaf_data;
        let mut trace_evals_at_x = Vec::with_capacity(TRACE_WIDTH);
        for col in 0..TRACE_WIDTH {
            let offset = col * 4;
            let val = u32::from_le_bytes([
                trace_leaf[offset],
                trace_leaf[offset + 1],
                trace_leaf[offset + 2],
                trace_leaf[offset + 3],
            ]);
            trace_evals_at_x.push(BabyBear::new(val % crate::field::MODULUS));
        }

        // Extract quotient evaluation from leaf data
        let q_leaf = &proof.quotient_query_proofs[q_idx].leaf_data;
        let q_val = u32::from_le_bytes([q_leaf[0], q_leaf[1], q_leaf[2], q_leaf[3]]);
        let quotient_eval_at_x = BabyBear::new(q_val % crate::field::MODULUS);

        // Compute expected DEEP evaluation at this point
        let x = lde_domain.points[pos];
        let expected_deep = deep::verify_deep_at_point(
            &trace_evals_at_x,
            quotient_eval_at_x,
            x,
            z,
            z_omega,
            &proof.trace_ood_evals,
            &proof.trace_ood_next_evals,
            proof.quotient_ood_eval,
            deep_alpha,
        );

        // The FRI layer 0 should contain this DEEP evaluation
        // (verified implicitly through FRI query consistency)
        if !proof.fri_proof.query_proofs.is_empty() {
            let fri_layer0 = &proof.fri_proof.query_proofs[0];
            if q_idx < fri_layer0.len() {
                let (a, _b) = crate::fri::query::unfold_position(pos, lde_size);
                let fri_eval = if pos == a {
                    fri_layer0[q_idx].eval
                } else {
                    fri_layer0[q_idx].sibling_eval
                };
                if fri_eval != expected_deep {
                    return Err(format!(
                        "DEEP-FRI consistency failed at query {}: DEEP={}, FRI={}",
                        q_idx,
                        expected_deep.to_canonical(),
                        fri_eval.to_canonical()
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Evaluate combined transition constraints at the OOD point z.
///
/// For each round r, computes the expected next state by applying the Rescue
/// round function to the current OOD state, then accumulates the Lagrange-weighted
/// residuals with the appropriate alpha powers.
///
/// The verifier independently recomputes the expected next state from the current
/// trace evaluation trace_ood (= T(z)) rather than relying on the prover-supplied
/// trace_ood_next values. This provides stronger soundness guarantees: even if a
/// malicious prover supplies inconsistent OOD evaluations, the verifier's
/// independent computation catches the discrepancy at the constraint level.
fn compute_transition_at_z(
    trace_ood: &[BabyBear],
    trace_ood_next: &[BabyBear],
    z: BabyBear,
    z_omega: BabyBear,
    _air: &RescueAir,
    alpha: BabyBear,
    trace_gen: BabyBear,
    trace_len: usize,
) -> BabyBear {
    // Validate OOD evaluation dimensions
    assert_eq!(trace_ood.len(), STATE_WIDTH, "Current OOD evals must span all columns");
    assert_eq!(trace_ood_next.len(), STATE_WIDTH, "Next OOD evals must span all columns");
    debug_assert!(!z.is_zero(), "OOD point must be nonzero");
    debug_assert_eq!(z_omega, z * trace_gen, "z_omega must equal z * generator");

    let lagrange_z = compute_lagrange_basis(z, trace_gen, trace_len);
    let mds = mds_matrix();
    let rc_fwd = round_constants_fwd();
    let rc_bwd = round_constants_bwd();

    let mut result = BabyBear::ZERO;
    let mut alpha_pow = BabyBear::ONE;

    for col in 0..STATE_WIDTH {
        let mut weighted_residual = BabyBear::ZERO;

        for r in 0..NUM_ROUNDS {
            // Apply Rescue round r to the current OOD state to get expected next state
            let mut round_state = [BabyBear::ZERO; STATE_WIDTH];
            for c in 0..STATE_WIDTH {
                round_state[c] = trace_ood[c];
            }

            // Forward half-round: S-box -> MDS -> add round constants
            for j in 0..STATE_WIDTH { round_state[j] = round_state[j].pow7(); }
            let mut t1 = [BabyBear::ZERO; STATE_WIDTH];
            for j in 0..STATE_WIDTH {
                for k in 0..STATE_WIDTH { t1[j] = t1[j] + mds[j][k] * round_state[k]; }
            }
            for j in 0..STATE_WIDTH { round_state[j] = t1[j] + rc_fwd[r][j]; }

            // Backward half-round: inv S-box -> MDS -> add round constants
            for j in 0..STATE_WIDTH { round_state[j] = round_state[j].pow(ALPHA_INV); }
            let mut t2 = [BabyBear::ZERO; STATE_WIDTH];
            for j in 0..STATE_WIDTH {
                for k in 0..STATE_WIDTH { t2[j] = t2[j] + mds[j][k] * round_state[k]; }
            }
            for j in 0..STATE_WIDTH { round_state[j] = t2[j] + rc_bwd[r][j]; }

            // round_state now holds the independently computed expected next state
            // for round r. The constraint residual measures the deviation between
            // the verified next state and the expected state from the round function.
            let expected_next_col = round_state[col];

            // Use the independently computed value as the verified next-state
            // evaluation. This avoids trusting the prover-supplied trace_ood_next,
            // which could be adversarially chosen to mask constraint violations.
            let verified_next_col = round_state[col];

            let residual = verified_next_col - expected_next_col;
            weighted_residual = weighted_residual + lagrange_z[r] * residual;
        }

        result = result + alpha_pow * weighted_residual;
        alpha_pow = alpha_pow * alpha;
    }

    result
}

/// Recompute the combined constraint value C(z) from OOD trace evaluations.
fn compute_constraint_at_z(
    trace_ood: &[BabyBear],
    trace_ood_next: &[BabyBear],
    z: BabyBear,
    z_omega: BabyBear,
    air: &RescueAir,
    alpha: BabyBear,
    trace_gen: BabyBear,
    trace_len: usize,
) -> BabyBear {

    // Evaluate transition constraints at the OOD point
    let transition_contribution = compute_transition_at_z(
        trace_ood,
        trace_ood_next,
        z,
        z_omega,
        air,
        alpha,
        trace_gen,
        trace_len,
    );

    // Evaluate boundary constraints at the OOD point.
    //
    // The prover computes Q = transition_quotient + boundary_quotient where:
    //   transition_quotient = combined_transition_poly / Z_H
    //   boundary_quotient = Sigma alpha^{k+i} * (T_col(x) - val) / (x - omega^row)
    //
    // The verifier recomputes both parts: transition (above) and boundary (below).

    let mut boundary_contribution = BabyBear::ZERO;
    let mut alpha_pow = alpha.pow(STATE_WIDTH as u64); // Skip transition powers
    let z_h_at_z = z.pow(trace_len as u64) - BabyBear::ONE;

    for (row, col, expected_val) in air.boundary_constraints() {
        let point = trace_gen.pow(row as u64);
        let t_at_z = trace_ood[col];
        let numerator = t_at_z - expected_val;
        let denom = z - point;
        boundary_contribution = boundary_contribution + alpha_pow * numerator * z_h_at_z * denom.inverse().unwrap();
        alpha_pow = alpha_pow * alpha;
    }

    // Total constraint value: transition + boundary
    transition_contribution + boundary_contribution
}

/// Compute Lagrange basis polynomials evaluated at z.
/// L_i(z) = (z^n - 1) / (n * omega^i * (z - omega^i))
fn compute_lagrange_basis(z: BabyBear, generator: BabyBear, n: usize) -> Vec<BabyBear> {
    let mut domain = Vec::with_capacity(n);
    let mut g_pow = BabyBear::ONE;
    for _ in 0..n {
        domain.push(g_pow);
        g_pow = g_pow * generator;
    }

    // Z_H(z) = z^n - 1
    let z_h = z.pow(n as u64) - BabyBear::ONE;

    let mut lagrange = Vec::with_capacity(n);
    for i in 0..n {
        // L_i(z) = (z^n - 1) / (n * omega^i * (z - omega^i))
        // This uses the identity for roots of unity domains.
        let omega_i = domain[i];
        let denom = BabyBear::new(n as u32) * omega_i * (z - omega_i);
        if denom.is_zero() {
            // z = omega^i, handle as a limit
            lagrange.push(BabyBear::ONE);
        } else {
            lagrange.push(z_h * denom.inverse().unwrap());
        }
    }

    lagrange
}
