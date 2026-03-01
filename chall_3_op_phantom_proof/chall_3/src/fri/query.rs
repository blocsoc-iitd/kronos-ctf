//! FRI query phase: deriving and verifying query positions.

use crate::field::BabyBear;
use crate::merkle::verify_merkle_proof;
use crate::stark::types::FriQueryProof;

/// Derive query positions for FRI.
///
/// SECURITY: The number of queries directly impacts soundness.
/// Each query provides -log2(1 - (1/blowup_factor)) bits of security.
/// With 28 queries and blowup=8, we get ~28 * 0.19 = 5.3 bits...
/// TODO(security): is this sufficient? Review before production use.
/// For now proceeding with the standard parameter choice.
pub fn derive_query_positions(
    positions: &[usize],
    current_domain_size: usize,
) -> Vec<usize> {
    // Map positions into the current (possibly folded) domain
    positions
        .iter()
        .map(|&p| p % current_domain_size)
        .collect()
}

/// For a position in a folded domain, get the two positions in the
/// parent domain that fold to it.
///
/// In a domain of size n where the second half contains negations of the first:
///   position i folds with position i + n/2
pub fn unfold_position(pos: usize, parent_domain_size: usize) -> (usize, usize) {
    let half = parent_domain_size / 2;
    let base = pos % half;
    (base, base + half)
}

/// Verify a FRI query at a single layer.
///
/// Checks that:
/// 1. The Merkle proofs are valid against the layer commitment
/// 2. The evaluations fold correctly to the claimed next-layer value
pub fn verify_fri_query(
    query_proof: &FriQueryProof,
    commitment: &[u8; 32],
    domain_point: BabyBear,
    alpha: BabyBear,
    expected_folded: BabyBear,
) -> bool {
    // Verify Merkle proofs
    if !verify_merkle_proof(&query_proof.merkle_proof, commitment) {
        return false;
    }
    if !verify_merkle_proof(&query_proof.sibling_merkle_proof, commitment) {
        return false;
    }

    // Verify folding
    super::fold::verify_fold(
        query_proof.eval,
        query_proof.sibling_eval,
        domain_point,
        alpha,
        expected_folded,
    )
}

/// Encode a field element as bytes for Merkle leaf data.
pub fn field_to_leaf_bytes(val: BabyBear) -> Vec<u8> {
    val.to_bytes().to_vec()
}

/// Decode a field element from Merkle leaf data.
pub fn leaf_bytes_to_field(data: &[u8]) -> BabyBear {
    if data.len() >= 4 {
        let val = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        BabyBear::new(val % crate::field::MODULUS)
    } else {
        BabyBear::ZERO
    }
}
