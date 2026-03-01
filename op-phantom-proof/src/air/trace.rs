//! Trace generation and validation for Rescue-Prime AIR.

use crate::field::BabyBear;
use crate::hash::params::*;
use crate::hash::rescue::rescue_trace;

/// Generate the execution trace for a Rescue-Prime hash computation.
///
/// Returns a 2D array: trace[col][row], where:
/// - There are STATE_WIDTH columns (one per state element)
/// - There are NUM_ROUNDS + 1 rows (initial state + one per round)
///
/// Row 0: initial state = [input[0], input[1], 0, 0]
/// Row i (1 <= i <= NUM_ROUNDS): state after round i
pub fn generate_trace(input: [BabyBear; 2]) -> Vec<Vec<BabyBear>> {
    let states = rescue_trace(input);

    // Convert from row-major (states[row][col]) to column-major (trace[col][row])
    let num_rows = states.len();
    let mut trace = vec![vec![BabyBear::ZERO; num_rows]; STATE_WIDTH];

    for row in 0..num_rows {
        for col in 0..STATE_WIDTH {
            trace[col][row] = states[row][col];
        }
    }

    trace
}

/// Validate that a trace satisfies the Rescue-Prime AIR constraints.
pub fn validate_trace(
    trace: &[Vec<BabyBear>],
    input: [BabyBear; 2],
    output: [BabyBear; 2],
) -> Result<(), String> {
    let air = super::RescueAir::new(input, output);
    let num_rows = trace[0].len();

    if num_rows != air.trace_length() {
        return Err(format!(
            "Trace has {} rows, expected {}",
            num_rows,
            air.trace_length()
        ));
    }

    if trace.len() != STATE_WIDTH {
        return Err(format!(
            "Trace has {} columns, expected {}",
            trace.len(),
            STATE_WIDTH
        ));
    }

    // Check boundary constraints
    for (row, col, expected) in air.boundary_constraints() {
        let actual = trace[col][row];
        if actual != expected {
            return Err(format!(
                "Boundary constraint failed at row={}, col={}: expected {}, got {}",
                row,
                col,
                expected.to_canonical(),
                actual.to_canonical()
            ));
        }
    }

    // Check transition constraints
    for round in 0..NUM_ROUNDS {
        let mut current = [BabyBear::ZERO; STATE_WIDTH];
        let mut next = [BabyBear::ZERO; STATE_WIDTH];
        for col in 0..STATE_WIDTH {
            current[col] = trace[col][round];
            next[col] = trace[col][round + 1];
        }

        let residuals = air.evaluate_transition(&current, &next, round);
        for (col, &r) in residuals.iter().enumerate() {
            if !r.is_zero() {
                return Err(format!(
                    "Transition constraint failed at round={}, col={}: residual = {}",
                    round,
                    col,
                    r.to_canonical()
                ));
            }
        }
    }

    Ok(())
}
