//! # Phantom STARK
//!
//! A custom STARK prover/verifier for Rescue-Prime hash preimage proofs,
//! built from scratch over the BabyBear prime field.

#![allow(clippy::needless_range_loop)]

pub mod field;
pub mod poly;
pub mod hash;
pub mod merkle;
pub mod air;
pub mod fri;
pub mod stark;

pub use field::BabyBear;
