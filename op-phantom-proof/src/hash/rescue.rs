//! Rescue-Prime permutation and hash function over BabyBear.
//!
//! Rescue-Prime applies alternating S-box layers (x^α and x^{α^{-1}})
//! interleaved with MDS matrix multiplication and round constant addition.
//!
//! Each round consists of:
//!   1. Apply S-box: state[i] = state[i]^α
//!   2. MDS multiply: state = MDS × state
//!   3. Add round constants (forward)
//!   4. Apply inverse S-box: state[i] = state[i]^{α^{-1}}
//!   5. MDS multiply: state = MDS × state
//!   6. Add round constants (backward)

use crate::field::BabyBear;
use super::params::*;

/// Apply the Rescue-Prime permutation to a state of 4 field elements.
pub fn rescue_permutation(state: &mut [BabyBear; STATE_WIDTH]) {
    let mds = mds_matrix();
    let rc_fwd = round_constants_fwd();
    let rc_bwd = round_constants_bwd();

    for round in 0..NUM_ROUNDS {
        // Forward half-round: S-box (x^α), MDS, add constants
        for i in 0..STATE_WIDTH {
            state[i] = state[i].pow7(); // α = 7
        }
        mds_multiply(state, &mds);
        for i in 0..STATE_WIDTH {
            state[i] = state[i] + rc_fwd[round][i];
        }

        // Backward half-round: inverse S-box (x^{α^{-1}}), MDS, add constants
        for i in 0..STATE_WIDTH {
            state[i] = state[i].pow(ALPHA_INV);
        }
        mds_multiply(state, &mds);
        for i in 0..STATE_WIDTH {
            state[i] = state[i] + rc_bwd[round][i];
        }
    }
}

/// Multiply state by the MDS matrix.
fn mds_multiply(state: &mut [BabyBear; STATE_WIDTH], mds: &[[BabyBear; STATE_WIDTH]; STATE_WIDTH]) {
    let mut new_state = [BabyBear::ZERO; STATE_WIDTH];
    for i in 0..STATE_WIDTH {
        for j in 0..STATE_WIDTH {
            new_state[i] = new_state[i] + mds[i][j] * state[j];
        }
    }
    *state = new_state;
}

/// Rescue-Prime hash: absorbs 2 field elements, produces 2 field elements.
///
/// Uses sponge construction:
/// - State = [input[0], input[1], 0, 0] (rate=2, capacity=2)
/// - Apply permutation
/// - Output = [state[0], state[1]]
pub fn rescue_hash(input: [BabyBear; 2]) -> [BabyBear; 2] {
    let mut state = [BabyBear::ZERO; STATE_WIDTH];
    state[0] = input[0];
    state[1] = input[1];
    // Capacity elements remain zero (domain separation is implicit in
    // the fixed-length input format)

    rescue_permutation(&mut state);

    [state[0], state[1]]
}

/// Compute the Rescue-Prime trace: returns the state after each half-round.
/// This is used by the AIR to verify the computation step-by-step.
///
/// Returns a vector of states, where:
/// - trace[0] = initial state (after input absorption)
/// - trace[2*r + 1] = state after forward half-round of round r
/// - trace[2*r + 2] = state after backward half-round of round r
/// Total: 1 + 2*NUM_ROUNDS = 15 states, but we only use the
/// state transitions between consecutive round pairs.
pub fn rescue_trace(input: [BabyBear; 2]) -> Vec<[BabyBear; STATE_WIDTH]> {
    let mds = mds_matrix();
    let rc_fwd = round_constants_fwd();
    let rc_bwd = round_constants_bwd();

    let mut state = [BabyBear::ZERO; STATE_WIDTH];
    state[0] = input[0];
    state[1] = input[1];

    let mut trace = Vec::with_capacity(NUM_ROUNDS + 1);
    trace.push(state);

    for round in 0..NUM_ROUNDS {
        // Forward half-round
        for i in 0..STATE_WIDTH {
            state[i] = state[i].pow7();
        }
        mds_multiply(&mut state, &mds);
        for i in 0..STATE_WIDTH {
            state[i] = state[i] + rc_fwd[round][i];
        }

        // Backward half-round
        for i in 0..STATE_WIDTH {
            state[i] = state[i].pow(ALPHA_INV);
        }
        mds_multiply(&mut state, &mds);
        for i in 0..STATE_WIDTH {
            state[i] = state[i] + rc_bwd[round][i];
        }

        trace.push(state);
    }

    trace
}
