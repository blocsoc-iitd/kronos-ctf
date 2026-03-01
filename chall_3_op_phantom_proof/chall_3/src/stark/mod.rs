//! STARK proof system for Rescue-Prime.

pub mod transcript;
pub mod types;
pub mod deep;
pub mod prover;
pub mod verifier;

pub use types::{StarkProof, PublicInputs};
pub use prover::prove;
pub use verifier::verify;
