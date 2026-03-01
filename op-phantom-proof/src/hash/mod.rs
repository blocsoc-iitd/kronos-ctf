//! Hash functions for the Phantom STARK.

pub mod params;
pub mod rescue;

use crate::field::BabyBear;

/// Rescue-Prime hash: absorbs 2 field elements, squeezes 2 field elements.
pub fn rescue_hash(input: [BabyBear; 2]) -> [BabyBear; 2] {
    rescue::rescue_hash(input)
}
