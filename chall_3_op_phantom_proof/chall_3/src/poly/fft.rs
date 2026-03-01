//! Radix-2 FFT (NTT) and inverse FFT over BabyBear.
//!
//! The FFT evaluates a polynomial at powers of a root of unity.
//! The inverse FFT interpolates from evaluations back to coefficients.

use crate::field::BabyBear;

/// In-place radix-2 decimation-in-time FFT.
///
/// `vals` must have length 2^k.
/// `generator` must be a primitive root of unity of order len(vals).
pub fn fft(coeffs: &[BabyBear], generator: BabyBear) -> Vec<BabyBear> {
    let n = coeffs.len();
    assert!(n.is_power_of_two(), "FFT size must be a power of 2");
    if n == 1 {
        return coeffs.to_vec();
    }

    let mut vals = coeffs.to_vec();
    bit_reverse_permutation(&mut vals);
    fft_in_place(&mut vals, generator);
    vals
}

/// Inverse FFT: given evaluations at powers of generator, recover coefficients.
pub fn ifft(evals: &[BabyBear], generator: BabyBear) -> Vec<BabyBear> {
    let n = evals.len();
    assert!(n.is_power_of_two(), "IFFT size must be a power of 2");
    if n == 1 {
        return evals.to_vec();
    }

    let gen_inv = generator.inverse().unwrap();
    let mut coeffs = evals.to_vec();
    bit_reverse_permutation(&mut coeffs);
    fft_in_place(&mut coeffs, gen_inv);

    // Scale by 1/n
    let n_inv = BabyBear::new(n as u32).inverse().unwrap();
    for c in &mut coeffs {
        *c = *c * n_inv;
    }
    coeffs
}

/// In-place Cooley-Tukey butterfly FFT.
fn fft_in_place(vals: &mut [BabyBear], generator: BabyBear) {
    let n = vals.len();
    let log_n = n.trailing_zeros();

    for s in 0..log_n {
        let m = 1 << (s + 1);
        let half_m = m >> 1;
        // Twiddle factor step: w_m = generator^(n/m)
        let w_m = generator.pow((n / m) as u64);

        for k in (0..n).step_by(m) {
            let mut w = BabyBear::ONE;
            for j in 0..half_m {
                let t = w * vals[k + j + half_m];
                let u = vals[k + j];
                vals[k + j] = u + t;
                vals[k + j + half_m] = u - t;
                w = w * w_m;
            }
        }
    }
}

/// Bit-reversal permutation on a slice of length 2^k.
fn bit_reverse_permutation(vals: &mut [BabyBear]) {
    let n = vals.len();
    let log_n = n.trailing_zeros();
    for i in 0..n {
        let j = reverse_bits(i as u32, log_n) as usize;
        if i < j {
            vals.swap(i, j);
        }
    }
}

/// Reverse the lower `bits` bits of `x`.
fn reverse_bits(x: u32, bits: u32) -> u32 {
    let mut result = 0u32;
    let mut x = x;
    for _ in 0..bits {
        result = (result << 1) | (x & 1);
        x >>= 1;
    }
    result
}
