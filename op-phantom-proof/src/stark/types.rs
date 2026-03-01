//! Types for the STARK proof system.

use crate::field::BabyBear;
use crate::merkle::MerkleProof;

/// Number of FRI queries.
pub const NUM_QUERIES: usize = 28;

/// Blowup factor for the LDE (Low Degree Extension).
pub const BLOWUP_FACTOR: usize = 8;

/// Trace length (number of rows in the execution trace).
pub const TRACE_LENGTH: usize = 8; // NUM_ROUNDS + 1 = 7 + 1 = 8

/// Number of trace columns.
pub const TRACE_WIDTH: usize = 4; // STATE_WIDTH

/// LDE domain size = TRACE_LENGTH * BLOWUP_FACTOR.
pub const LDE_DOMAIN_SIZE: usize = TRACE_LENGTH * BLOWUP_FACTOR; // 64

/// The coset offset for the LDE domain.
/// We use the multiplicative generator of F_p^* to shift the domain,
/// so the LDE domain doesn't overlap with the trace domain.
pub const LDE_COSET_OFFSET: u32 = 31; // = MULTIPLICATIVE_GENERATOR

/// A complete STARK proof.
#[derive(Clone, Debug)]
pub struct StarkProof {
    /// Merkle root of the trace LDE columns.
    pub trace_commitment: [u8; 32],
    /// Merkle root of the quotient polynomial LDE.
    pub quotient_commitment: [u8; 32],
    /// Trace column evaluations at the OOD (out-of-domain) point z.
    /// trace_ood_evals[col] = trace_poly_col(z)
    pub trace_ood_evals: Vec<BabyBear>,
    /// Trace column evaluations at z * ω (next row).
    /// trace_ood_next_evals[col] = trace_poly_col(z * ω)
    pub trace_ood_next_evals: Vec<BabyBear>,
    /// Quotient polynomial evaluation at z.
    pub quotient_ood_eval: BabyBear,
    /// FRI proof.
    pub fri_proof: FriProof,
    /// Merkle proofs for trace at query positions.
    pub trace_query_proofs: Vec<MerkleProof>,
    /// Merkle proofs for quotient at query positions.
    pub quotient_query_proofs: Vec<MerkleProof>,
}

/// FRI proof containing the layer commitments and query responses.
#[derive(Clone, Debug)]
pub struct FriProof {
    /// Merkle commitments for each FRI layer (except the last).
    pub layer_commitments: Vec<[u8; 32]>,
    /// The final constant value after all folding layers.
    pub final_value: BabyBear,
    /// Query proofs for each layer.
    /// query_proofs[layer][query_idx] = (MerkleProof for the pair)
    pub query_proofs: Vec<Vec<FriQueryProof>>,
}

/// A single FRI query response for one layer.
#[derive(Clone, Debug)]
pub struct FriQueryProof {
    /// The evaluation at the queried position.
    pub eval: BabyBear,
    /// The evaluation at the sibling position (x and -x are paired).
    pub sibling_eval: BabyBear,
    /// Merkle proof for the queried position.
    pub merkle_proof: MerkleProof,
    /// Merkle proof for the sibling position.
    pub sibling_merkle_proof: MerkleProof,
}

/// Public inputs to the STARK.
#[derive(Clone, Debug)]
pub struct PublicInputs {
    /// Input to Rescue-Prime (2 field elements).
    pub input: [BabyBear; 2],
    /// Claimed output of Rescue-Prime (2 field elements).
    pub output: [BabyBear; 2],
}

impl StarkProof {
    /// Serialize the proof to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Commitments
        bytes.extend_from_slice(&self.trace_commitment);
        bytes.extend_from_slice(&self.quotient_commitment);

        // OOD evaluations
        bytes.extend_from_slice(&(self.trace_ood_evals.len() as u32).to_le_bytes());
        for e in &self.trace_ood_evals {
            bytes.extend_from_slice(&e.to_bytes());
        }
        bytes.extend_from_slice(&(self.trace_ood_next_evals.len() as u32).to_le_bytes());
        for e in &self.trace_ood_next_evals {
            bytes.extend_from_slice(&e.to_bytes());
        }
        bytes.extend_from_slice(&self.quotient_ood_eval.to_bytes());

        // FRI proof
        bytes.extend_from_slice(&(self.fri_proof.layer_commitments.len() as u32).to_le_bytes());
        for c in &self.fri_proof.layer_commitments {
            bytes.extend_from_slice(c);
        }
        bytes.extend_from_slice(&self.fri_proof.final_value.to_bytes());

        // FRI query proofs
        bytes.extend_from_slice(&(self.fri_proof.query_proofs.len() as u32).to_le_bytes());
        for layer_proofs in &self.fri_proof.query_proofs {
            bytes.extend_from_slice(&(layer_proofs.len() as u32).to_le_bytes());
            for qp in layer_proofs {
                bytes.extend_from_slice(&qp.eval.to_bytes());
                bytes.extend_from_slice(&qp.sibling_eval.to_bytes());
                let mp_bytes = qp.merkle_proof.to_bytes();
                bytes.extend_from_slice(&(mp_bytes.len() as u32).to_le_bytes());
                bytes.extend_from_slice(&mp_bytes);
                let smp_bytes = qp.sibling_merkle_proof.to_bytes();
                bytes.extend_from_slice(&(smp_bytes.len() as u32).to_le_bytes());
                bytes.extend_from_slice(&smp_bytes);
            }
        }

        // Trace query proofs
        bytes.extend_from_slice(&(self.trace_query_proofs.len() as u32).to_le_bytes());
        for tp in &self.trace_query_proofs {
            let tp_bytes = tp.to_bytes();
            bytes.extend_from_slice(&(tp_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&tp_bytes);
        }

        // Quotient query proofs
        bytes.extend_from_slice(&(self.quotient_query_proofs.len() as u32).to_le_bytes());
        for qp in &self.quotient_query_proofs {
            let qp_bytes = qp.to_bytes();
            bytes.extend_from_slice(&(qp_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&qp_bytes);
        }

        bytes
    }

    /// Deserialize a proof from bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut offset = 0;

        fn read_u32(data: &[u8], offset: &mut usize) -> Option<u32> {
            if *offset + 4 > data.len() {
                return None;
            }
            let val = u32::from_le_bytes(data[*offset..*offset + 4].try_into().ok()?);
            *offset += 4;
            Some(val)
        }

        fn read_bytes32(data: &[u8], offset: &mut usize) -> Option<[u8; 32]> {
            if *offset + 32 > data.len() {
                return None;
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&data[*offset..*offset + 32]);
            *offset += 32;
            Some(arr)
        }

        fn read_field(data: &[u8], offset: &mut usize) -> Option<BabyBear> {
            let val = read_u32(data, offset)?;
            Some(BabyBear::new(val % crate::field::MODULUS))
        }

        let trace_commitment = read_bytes32(data, &mut offset)?;
        let quotient_commitment = read_bytes32(data, &mut offset)?;

        let num_ood = read_u32(data, &mut offset)? as usize;
        let mut trace_ood_evals = Vec::with_capacity(num_ood);
        for _ in 0..num_ood {
            trace_ood_evals.push(read_field(data, &mut offset)?);
        }

        let num_ood_next = read_u32(data, &mut offset)? as usize;
        let mut trace_ood_next_evals = Vec::with_capacity(num_ood_next);
        for _ in 0..num_ood_next {
            trace_ood_next_evals.push(read_field(data, &mut offset)?);
        }

        let quotient_ood_eval = read_field(data, &mut offset)?;

        let num_layers = read_u32(data, &mut offset)? as usize;
        let mut layer_commitments = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            layer_commitments.push(read_bytes32(data, &mut offset)?);
        }
        let final_value = read_field(data, &mut offset)?;

        let num_query_layers = read_u32(data, &mut offset)? as usize;
        let mut query_proofs = Vec::with_capacity(num_query_layers);
        for _ in 0..num_query_layers {
            let num_queries = read_u32(data, &mut offset)? as usize;
            let mut layer_proofs = Vec::with_capacity(num_queries);
            for _ in 0..num_queries {
                let eval = read_field(data, &mut offset)?;
                let sibling_eval = read_field(data, &mut offset)?;
                let mp_len = read_u32(data, &mut offset)? as usize;
                if offset + mp_len > data.len() {
                    return None;
                }
                let (merkle_proof, _) = MerkleProof::from_bytes(&data[offset..offset + mp_len])?;
                offset += mp_len;
                let smp_len = read_u32(data, &mut offset)? as usize;
                if offset + smp_len > data.len() {
                    return None;
                }
                let (sibling_merkle_proof, _) =
                    MerkleProof::from_bytes(&data[offset..offset + smp_len])?;
                offset += smp_len;
                layer_proofs.push(FriQueryProof {
                    eval,
                    sibling_eval,
                    merkle_proof,
                    sibling_merkle_proof,
                });
            }
            query_proofs.push(layer_proofs);
        }

        let fri_proof = FriProof {
            layer_commitments,
            final_value,
            query_proofs,
        };

        let num_trace_proofs = read_u32(data, &mut offset)? as usize;
        let mut trace_query_proofs = Vec::with_capacity(num_trace_proofs);
        for _ in 0..num_trace_proofs {
            let tp_len = read_u32(data, &mut offset)? as usize;
            if offset + tp_len > data.len() {
                return None;
            }
            let (tp, _) = MerkleProof::from_bytes(&data[offset..offset + tp_len])?;
            offset += tp_len;
            trace_query_proofs.push(tp);
        }

        let num_q_proofs = read_u32(data, &mut offset)? as usize;
        let mut quotient_query_proofs = Vec::with_capacity(num_q_proofs);
        for _ in 0..num_q_proofs {
            let qp_len = read_u32(data, &mut offset)? as usize;
            if offset + qp_len > data.len() {
                return None;
            }
            let (qp, _) = MerkleProof::from_bytes(&data[offset..offset + qp_len])?;
            offset += qp_len;
            quotient_query_proofs.push(qp);
        }

        Some(StarkProof {
            trace_commitment,
            quotient_commitment,
            trace_ood_evals,
            trace_ood_next_evals,
            quotient_ood_eval,
            fri_proof,
            trace_query_proofs,
            quotient_query_proofs,
        })
    }
}
