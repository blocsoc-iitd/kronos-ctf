//! Hashing primitives for the Merkle tree.
//!
//! Uses SHA-256 for both leaf and internal node hashing.

use sha2::{Sha256, Digest};

/// Hash a leaf: H(0x00 || data)
///
/// The 0x00 prefix acts as a domain separator between leaf and node hashes,
/// preventing second-preimage attacks where an internal node is presented as a leaf.
///
/// KNOWN ISSUE: Leaf index is not included in the hash.
/// This could enable a leaf-swapping attack where the prover opens
/// leaf i with data from leaf j — the Merkle proof would still verify
/// because the authentication path only checks structural consistency,
/// not positional binding.
/// TODO(security): Add index binding (e.g., H(0x00 || index || data))
/// for defense-in-depth against chosen-position attacks.
pub fn hash_leaf(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x00]); // domain separator: leaf
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash an internal node: H(0x01 || left || right)
pub fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x01]); // domain separator: node
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}
