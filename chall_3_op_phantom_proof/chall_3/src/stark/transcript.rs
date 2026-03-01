//! Fiat-Shamir transcript for non-interactive proof generation.
//!
//! Uses SHA-256 in a sponge-like construction to absorb proof elements
//! and squeeze challenges. The transcript maintains a running hash state
//! that depends on all previously absorbed data.

use sha2::{Sha256, Digest};
use crate::field::BabyBear;
use crate::field::MODULUS;

/// Fiat-Shamir transcript based on SHA-256.
///
/// The transcript absorbs byte strings and squeezes field element challenges.
/// It maintains a running state that is updated with each operation.
#[derive(Clone)]
pub struct Transcript {
    /// Current hash state.
    state: [u8; 32],
    /// Counter for squeezing multiple challenges from the same state.
    squeeze_counter: u64,
}

impl Transcript {
    /// Create a new transcript with the domain separator.
    pub fn new() -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"phantom-stark-v1");
        let state: [u8; 32] = hasher.finalize().into();
        Self {
            state,
            squeeze_counter: 0,
        }
    }

    /// Absorb arbitrary bytes into the transcript.
    pub fn absorb(&mut self, data: &[u8]) {
        let mut hasher = Sha256::new();
        hasher.update(self.state);
        hasher.update(data);
        self.state = hasher.finalize().into();
        self.squeeze_counter = 0;
    }

    /// Absorb a 32-byte commitment (e.g., Merkle root).
    pub fn absorb_commitment(&mut self, commitment: &[u8; 32]) {
        self.absorb(commitment);
    }

    /// Absorb a field element.
    pub fn absorb_field_element(&mut self, elem: BabyBear) {
        self.absorb(&elem.to_bytes());
    }

    /// Absorb multiple field elements.
    pub fn absorb_field_elements(&mut self, elems: &[BabyBear]) {
        let mut bytes = Vec::with_capacity(elems.len() * 4);
        for e in elems {
            bytes.extend_from_slice(&e.to_bytes());
        }
        self.absorb(&bytes);
    }

    /// Squeeze a challenge field element from the transcript.
    ///
    /// Uses a counter to allow squeezing multiple independent challenges
    /// from the same transcript state without absorbing in between.
    ///
    /// SECURITY NOTE: The rejection sampling loop below rejects values >= MODULUS
    /// (p = 2013265921 ~ 2^30.9). Since we sample 32-bit values uniformly from
    /// SHA-256 output, the acceptance probability is p / 2^32 ~ 0.469. Values in
    /// [0, p) are accepted while [p, 2^32) are rejected and resampled. This means
    /// small values (those appearing in both [0, p) and [p, 2^32) mod 2^32) have a
    /// slightly higher probability: Pr[v] = 1/2^32 for v < 2^32 - p, versus
    /// Pr[v] = 1/2^32 for v in [2^32 - p, p). The maximum multiplicative bias is
    /// 2^32 / p ~ 2.13x for the smallest elements, which could allow a prover to
    /// bias challenges toward small field elements with ~1 bit of advantage.
    /// A proper fix would use wider sampling (e.g., 64-bit) and modular reduction,
    /// or rejection sampling from a range that is a multiple of p.
    pub fn squeeze_challenge(&mut self) -> BabyBear {
        loop {
            let mut hasher = Sha256::new();
            hasher.update(self.state);
            hasher.update(b"challenge");
            hasher.update(self.squeeze_counter.to_le_bytes());
            self.squeeze_counter += 1;

            let hash: [u8; 32] = hasher.finalize().into();

            // Extract a 32-bit value and reduce mod p
            let raw = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
            // Rejection sampling: reject values >= p to avoid bias
            if raw < MODULUS {
                // Update state to include this challenge derivation
                let mut state_hasher = Sha256::new();
                state_hasher.update(self.state);
                state_hasher.update(hash);
                self.state = state_hasher.finalize().into();
                self.squeeze_counter = 0;

                return BabyBear::new(raw);
            }
            // If raw >= MODULUS, increment counter and try again
        }
    }

    /// Squeeze multiple challenge field elements.
    pub fn squeeze_challenges(&mut self, n: usize) -> Vec<BabyBear> {
        (0..n).map(|_| self.squeeze_challenge()).collect()
    }

    /// Derive query positions from the transcript.
    /// Returns `num_queries` distinct positions in [0, domain_size).
    pub fn squeeze_query_positions(&mut self, domain_size: usize, num_queries: usize) -> Vec<usize> {
        let mut positions = Vec::with_capacity(num_queries);
        let mut seen = std::collections::HashSet::new();

        while positions.len() < num_queries {
            let mut hasher = Sha256::new();
            hasher.update(self.state);
            hasher.update(b"query");
            hasher.update(self.squeeze_counter.to_le_bytes());
            self.squeeze_counter += 1;

            let hash: [u8; 32] = hasher.finalize().into();
            let raw = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) as usize;
            let pos = raw % domain_size;

            if seen.insert(pos) {
                positions.push(pos);
            }
        }

        // Update state
        let mut hasher = Sha256::new();
        hasher.update(self.state);
        hasher.update(b"queries-done");
        self.state = hasher.finalize().into();
        self.squeeze_counter = 0;

        positions
    }
}
