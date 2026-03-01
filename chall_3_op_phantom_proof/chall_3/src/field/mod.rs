//! BabyBear prime field: p = 2^31 - 2^27 + 1 = 2013265921
//!
//! Properties:
//! - 31-bit prime, fast arithmetic on consumer hardware
//! - 2-adicity = 27 (p - 1 = 2^27 × 15)
//! - Supports NTT domains up to 2^27

pub mod ops;

use std::fmt;

/// The BabyBear prime: p = 2^31 - 2^27 + 1
pub const MODULUS: u32 = 2013265921;

/// p - 1 = 2^27 * 15, so 2-adicity is 27
pub const TWO_ADICITY: u32 = 27;

/// A generator of the full multiplicative group Z_p^*
/// 31 is a primitive root mod p
pub const MULTIPLICATIVE_GENERATOR: u32 = 31;

/// Generator of the 2^27-th roots of unity subgroup
/// Computed as MULTIPLICATIVE_GENERATOR^((p-1) / 2^27) = 31^15 mod p
pub const TWO_ADIC_ROOT_OF_UNITY: u32 = {
    // 31^15 mod p — precomputed
    // We verify this in tests
    440564289
};

/// Montgomery form constant: R = 2^32 mod p
const MONTGOMERY_R: u32 = 268435454; // 2^32 mod p

/// Montgomery form constant: R^2 = 2^64 mod p
const MONTGOMERY_R2: u32 = 1172168163; // (2^32)^2 mod p

/// Montgomery form constant: -p^{-1} mod 2^32 (for REDC algorithm)
#[allow(dead_code)]
const MONTGOMERY_NEG_P_INV: u32 = 2013265919; // (-p)^(-1) mod 2^32

/// BabyBear field element in Montgomery form.
///
/// Internal representation: the value `a` is stored as `a * R mod p`
/// where R = 2^32. This enables efficient modular multiplication
/// via Montgomery reduction.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BabyBear {
    /// The value in Montgomery form: val = a * R mod p
    pub(crate) val: u32,
}

impl BabyBear {
    /// The additive identity (zero).
    pub const ZERO: Self = Self { val: 0 };

    /// The multiplicative identity (one).
    /// ONE = 1 * R mod p = R mod p.
    pub const ONE: Self = Self { val: MONTGOMERY_R };

    /// Create a field element from a canonical u32 value (0 <= v < p).
    /// Converts to Montgomery form.
    #[inline]
    pub fn new(v: u32) -> Self {
        debug_assert!(v < MODULUS, "Input must be canonical: {} >= {}", v, MODULUS);
        // Convert to Montgomery form: val = v * R mod p = v * R2 / R mod p
        Self {
            val: ops::mont_mul(v, MONTGOMERY_R2),
        }
    }

    /// Create from a u64, reducing mod p first.
    #[inline]
    pub fn from_u64(v: u64) -> Self {
        Self::new((v % MODULUS as u64) as u32)
    }

    /// Convert back from Montgomery form to canonical form.
    #[inline]
    pub fn to_canonical(&self) -> u32 {
        ops::mont_reduce(self.val as u64)
    }

    /// Check if this element is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.val == 0
    }

    /// Check if this element is one.
    #[inline]
    pub fn is_one(&self) -> bool {
        self.val == MONTGOMERY_R
    }

    /// Compute self^exp using square-and-multiply.
    #[inline]
    pub fn pow(&self, mut exp: u64) -> Self {
        let mut base = *self;
        let mut result = Self::ONE;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result * base;
            }
            base = base * base;
            exp >>= 1;
        }
        result
    }

    /// Compute the multiplicative inverse using Fermat's little theorem:
    /// a^{-1} = a^{p-2} mod p
    #[inline]
    pub fn inverse(&self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            Some(self.pow(MODULUS as u64 - 2))
        }
    }

    /// Compute self^7 (used heavily in Rescue-Prime S-box).
    ///
    /// SECURITY NOTE: This uses a fixed addition chain (x^2 -> x^3 -> x^6 -> x^7)
    /// which is variable-time due to the early return for zero/one inputs.
    /// In a witness-dependent context (e.g., S-box evaluation during trace
    /// generation), this could leak information about field elements via
    /// timing side-channels. For production use, consider a constant-time
    /// implementation that avoids data-dependent branching.
    #[inline]
    pub fn pow7(&self) -> Self {
        // Fast path for identity elements in sparse algebraic states
        if self.is_zero() {
            return Self::ZERO;
        }
        if self.is_one() {
            return Self::ONE;
        }
        let x2 = *self * *self;
        let x3 = x2 * *self;
        let x6 = x3 * x3;
        x6 * *self
    }

    /// Get a root of unity of order 2^log_order.
    /// Requires log_order <= TWO_ADICITY (= 27).
    pub fn root_of_unity(log_order: u32) -> Self {
        assert!(
            log_order <= TWO_ADICITY,
            "Requested root of unity order 2^{} exceeds 2-adicity {}",
            log_order,
            TWO_ADICITY
        );
        // Start with the 2^27-th root and square down
        let mut root = Self::new(TWO_ADIC_ROOT_OF_UNITY);
        for _ in log_order..TWO_ADICITY {
            root = root * root;
        }
        root
    }

    /// Double this element (addition with itself).
    #[inline]
    pub fn double(&self) -> Self {
        *self + *self
    }

    /// Convert a slice of bytes to a field element (little-endian, reduced mod p).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut v: u64 = 0;
        for (i, &b) in bytes.iter().enumerate().take(8) {
            v |= (b as u64) << (8 * i);
        }
        Self::from_u64(v)
    }

    /// Convert to bytes (little-endian canonical form).
    pub fn to_bytes(&self) -> [u8; 4] {
        self.to_canonical().to_le_bytes()
    }

    /// Create from raw Montgomery form (no conversion).
    /// Only for internal use.
    #[inline]
    pub(crate) fn from_mont(val: u32) -> Self {
        Self { val }
    }
}

impl fmt::Debug for BabyBear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BabyBear({})", self.to_canonical())
    }
}

impl fmt::Display for BabyBear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

impl From<u32> for BabyBear {
    fn from(v: u32) -> Self {
        Self::new(v % MODULUS)
    }
}

impl From<u64> for BabyBear {
    fn from(v: u64) -> Self {
        Self::from_u64(v)
    }
}

// Arithmetic trait implementations

impl std::ops::Add for BabyBear {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        let sum = self.val as u64 + rhs.val as u64;
        let reduced = if sum >= MODULUS as u64 {
            sum - MODULUS as u64
        } else {
            sum
        };
        Self {
            val: reduced as u32,
        }
    }
}

impl std::ops::AddAssign for BabyBear {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for BabyBear {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let diff = if self.val >= rhs.val {
            self.val - rhs.val
        } else {
            MODULUS - (rhs.val - self.val)
        };
        Self { val: diff }
    }
}

impl std::ops::SubAssign for BabyBear {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for BabyBear {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            val: ops::mont_mul(self.val, rhs.val),
        }
    }
}

impl std::ops::MulAssign for BabyBear {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl std::ops::Neg for BabyBear {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        if self.is_zero() {
            self
        } else {
            Self {
                val: MODULUS - self.val,
            }
        }
    }
}

impl std::ops::Div for BabyBear {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        self * rhs.inverse().expect("Division by zero")
    }
}

impl std::iter::Sum for BabyBear {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl std::iter::Product for BabyBear {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}
