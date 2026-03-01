//! FRI (Fast Reed-Solomon Interactive Oracle Proof of Proximity) protocol.
//!
//! The FRI protocol verifies that a committed vector of evaluations is
//! close to a polynomial of bounded degree. It works by iteratively
//! folding the polynomial using random challenges, halving the domain
//! each time, until reaching a constant.

pub mod fold;
pub mod query;

use crate::field::BabyBear;
use crate::merkle::MerkleTree;
use crate::stark::transcript::Transcript;
use crate::stark::types::{FriProof, FriQueryProof, NUM_QUERIES};
use self::query::*;

/// FRI domain: a coset of a multiplicative subgroup.
#[derive(Clone, Debug)]
pub struct FriDomain {
    /// Domain points.
    pub points: Vec<BabyBear>,
    /// Size of the domain.
    pub size: usize,
    /// Generator of the underlying group.
    pub generator: BabyBear,
    /// Coset offset.
    pub offset: BabyBear,
}

impl FriDomain {
    /// Create a new FRI domain as a coset: {offset * g^i : i = 0..size-1}
    pub fn new(size: usize, generator: BabyBear, offset: BabyBear) -> Self {
        assert!(size.is_power_of_two());
        let mut points = Vec::with_capacity(size);
        let mut current = offset;
        for _ in 0..size {
            points.push(current);
            current = current * generator;
        }
        Self {
            points,
            size,
            generator,
            offset,
        }
    }

    /// Get the "squared" domain for the next FRI layer.
    /// Each point x becomes x^2, and the domain halves in size.
    pub fn fold_domain(&self) -> FriDomain {
        let new_size = self.size / 2;
        let new_gen = self.generator * self.generator; // g^2
        let new_offset = self.offset * self.offset; // offset^2
        FriDomain::new(new_size, new_gen, new_offset)
    }
}

/// Result of FRI proving, including the proof and the query positions used.
pub struct FriProveResult {
    pub proof: FriProof,
    pub query_positions: Vec<usize>,
}

/// Run the FRI commit phase (prover side).
///
/// Takes evaluations on the FRI domain and produces a FRI proof.
/// Returns the proof along with the query positions (needed by STARK prover
/// to generate query openings for trace and quotient commitments).
pub fn fri_prove(
    evaluations: &[BabyBear],
    domain: &FriDomain,
    transcript: &mut Transcript,
) -> FriProveResult {
    let mut current_evals = evaluations.to_vec();
    let mut current_domain = domain.clone();
    let mut layer_commitments = Vec::new();
    let mut layer_trees: Vec<MerkleTree> = Vec::new();
    let mut layer_evals: Vec<Vec<BabyBear>> = Vec::new();
    let mut alphas = Vec::new();

    // Commit phase: fold until we reach a constant
    while current_evals.len() > 1 {
        // Commit to current layer
        let leaves: Vec<Vec<u8>> = current_evals
            .iter()
            .map(|e| field_to_leaf_bytes(*e))
            .collect();
        let tree = MerkleTree::new(&leaves);
        let commitment = tree.root();

        transcript.absorb_commitment(&commitment);
        layer_commitments.push(commitment);
        layer_trees.push(tree);
        layer_evals.push(current_evals.clone());

        // Get folding challenge
        let alpha = transcript.squeeze_challenge();
        alphas.push(alpha);

        // Fold
        current_evals = fold::fold_evaluations(
            &current_evals,
            &current_domain.points,
            alpha,
        );
        current_domain = current_domain.fold_domain();
    }

    let final_value = current_evals[0];
    transcript.absorb_field_element(final_value);

    // Query phase
    let query_positions = transcript.squeeze_query_positions(domain.size, NUM_QUERIES);
    let mut query_proofs = Vec::new();

    for layer_idx in 0..layer_evals.len() {
        let layer_domain_size = layer_evals[layer_idx].len();
        let positions = derive_query_positions(&query_positions, layer_domain_size);

        let mut layer_query_proofs = Vec::new();
        for &pos in &positions {
            let (pos_a, pos_b) = query::unfold_position(pos, layer_domain_size);

            let eval_a = layer_evals[layer_idx][pos_a];
            let eval_b = layer_evals[layer_idx][pos_b];

            let proof_a = layer_trees[layer_idx].open(
                pos_a,
                &field_to_leaf_bytes(eval_a),
            );
            let proof_b = layer_trees[layer_idx].open(
                pos_b,
                &field_to_leaf_bytes(eval_b),
            );

            layer_query_proofs.push(FriQueryProof {
                eval: eval_a,
                sibling_eval: eval_b,
                merkle_proof: proof_a,
                sibling_merkle_proof: proof_b,
            });
        }
        query_proofs.push(layer_query_proofs);
    }

    FriProveResult {
        proof: FriProof {
            layer_commitments,
            final_value,
            query_proofs,
        },
        query_positions,
    }
}

/// Verify a FRI proof (verifier side).
pub fn fri_verify(
    proof: &FriProof,
    domain: &FriDomain,
    transcript: &mut Transcript,
    _max_degree: usize,
) -> Result<Vec<usize>, String> {
    let num_layers = proof.layer_commitments.len();

    // Replay commit phase
    let mut alphas = Vec::new();
    for i in 0..num_layers {
        transcript.absorb_commitment(&proof.layer_commitments[i]);
        let alpha = transcript.squeeze_challenge();
        alphas.push(alpha);
    }
    transcript.absorb_field_element(proof.final_value);

    // Query phase
    let query_positions = transcript.squeeze_query_positions(domain.size, NUM_QUERIES);

    let mut current_domain = domain.clone();
    for layer_idx in 0..num_layers {
        let layer_domain_size = current_domain.size;
        let positions = derive_query_positions(&query_positions, layer_domain_size);

        if layer_idx >= proof.query_proofs.len() {
            return Err("Missing query proofs for FRI layer".to_string());
        }
        if proof.query_proofs[layer_idx].len() != positions.len() {
            return Err(format!(
                "Wrong number of query proofs at layer {}: got {}, expected {}",
                layer_idx,
                proof.query_proofs[layer_idx].len(),
                positions.len()
            ));
        }

        for (q_idx, &pos) in positions.iter().enumerate() {
            let qp = &proof.query_proofs[layer_idx][q_idx];

            // Determine what the folded value should be
            let folded_pos = pos % (layer_domain_size / 2);
            let expected_folded = if layer_idx + 1 < num_layers {
                // Look up from next layer's query proofs
                let next_domain_size = layer_domain_size / 2;
                let next_positions = derive_query_positions(&query_positions, next_domain_size);

                // Find the corresponding position in the next layer
                let mut found = None;
                for (next_q, &next_pos) in next_positions.iter().enumerate() {
                    let (next_a, _) = unfold_position(next_pos, next_domain_size);
                    if next_a == folded_pos {
                        found = Some(proof.query_proofs[layer_idx + 1][next_q].eval);
                        break;
                    }
                    let (_, next_b) = unfold_position(next_pos, next_domain_size);
                    if next_b == folded_pos {
                        found = Some(proof.query_proofs[layer_idx + 1][next_q].sibling_eval);
                        break;
                    }
                }

                match found {
                    Some(v) => v,
                    None => {
                        // Compute the folded value ourselves
                        let (a, _) = unfold_position(pos, layer_domain_size);
                        let x = current_domain.points[a];
                        let f_even = (qp.eval + qp.sibling_eval)
                            * BabyBear::new(2).inverse().unwrap();
                        let f_odd = (qp.eval - qp.sibling_eval)
                            * (x.double()).inverse().unwrap();
                        f_even + f_odd
                    }
                }
            } else {
                proof.final_value
            };

            let (a, _) = unfold_position(pos, layer_domain_size);
            let x = current_domain.points[a];

            // Verify Merkle proofs
            if !crate::merkle::verify_merkle_proof(
                &qp.merkle_proof,
                &proof.layer_commitments[layer_idx],
            ) {
                return Err(format!(
                    "FRI Merkle proof failed at layer {}, query {}",
                    layer_idx, q_idx
                ));
            }
            if !crate::merkle::verify_merkle_proof(
                &qp.sibling_merkle_proof,
                &proof.layer_commitments[layer_idx],
            ) {
                return Err(format!(
                    "FRI sibling Merkle proof failed at layer {}, query {}",
                    layer_idx, q_idx
                ));
            }

            // Verify folding consistency
            if !fold::verify_fold(qp.eval, qp.sibling_eval, x, alphas[layer_idx], expected_folded)
            {
                return Err(format!(
                    "FRI fold verification failed at layer {}, query {}",
                    layer_idx, q_idx
                ));
            }
        }

        current_domain = current_domain.fold_domain();
    }

    Ok(query_positions)
}
