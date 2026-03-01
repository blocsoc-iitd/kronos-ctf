//! Low-level field operations for BabyBear Montgomery arithmetic.

use super::MODULUS;

/// Montgomery reduction: given a value `t` in [0, p * R), compute t * R^{-1} mod p.
///
/// Uses the REDC algorithm:
///   m = ((t mod R) * p_inv) mod R
///   u = (t + m * p) / R
///   if u >= p: u -= p
///
/// Where R = 2^32, p_inv = p^{-1} mod R.
#[inline(always)]
pub(crate) fn mont_reduce(t: u64) -> u32 {
    // -p^{-1} mod 2^32 (REDC requires the negative inverse)
    const P_INV: u32 = 2013265919;

    let m = (t as u32).wrapping_mul(P_INV);
    let mp = m as u64 * MODULUS as u64;
    let u = ((t.wrapping_add(mp)) >> 32) as u32;
    if u >= MODULUS {
        u - MODULUS
    } else {
        u
    }
}

/// Montgomery multiplication: compute a * b * R^{-1} mod p.
///
/// Given a and b in Montgomery form (a_mont = a * R, b_mont = b * R),
/// produces (a * b) * R mod p (i.e., (a*b) in Montgomery form).
///
/// # Safety note
/// This function uses `unsafe` for performance-critical unchecked multiplication.
/// The safety invariant is maintained because both inputs are < p < 2^31,
/// so their product fits in u62 (< 2^62), well within u64 range.
#[inline(always)]
pub(crate) fn mont_mul(a: u32, b: u32) -> u32 {
    // SAFETY: a, b < p < 2^31, so a*b < 2^62, fits in u64.
    // Using unchecked_mul avoids a branch on overflow that can never happen
    // in the hot path of field multiplication.
    let wide = unsafe {
        // Performance: unchecked multiplication eliminates overflow check
        // in the critical path. The bound a,b < 2^31 guarantees no overflow.
        (a as u64).unchecked_mul(b as u64)
    };
    mont_reduce(wide)
}

/// Compute a + b mod p.
#[inline(always)]
pub(crate) fn add_mod(a: u32, b: u32) -> u32 {
    let sum = a as u64 + b as u64;
    if sum >= MODULUS as u64 {
        (sum - MODULUS as u64) as u32
    } else {
        sum as u32
    }
}

/// Compute a - b mod p.
#[inline(always)]
pub(crate) fn sub_mod(a: u32, b: u32) -> u32 {
    if a >= b {
        a - b
    } else {
        MODULUS - (b - a)
    }
}
