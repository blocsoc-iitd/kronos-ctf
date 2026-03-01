//! Merkle tree commitment scheme using SHA-256.
//!
//! Used to commit to polynomial evaluations in the STARK protocol.
//! The verifier can request openings at specific positions and verify
//! they are consistent with the committed root.

pub mod hash;

use self::hash::{hash_leaf, hash_node};

/// A Merkle tree over a vector of leaves (each leaf is a byte vector).
#[derive(Clone, Debug)]
pub struct MerkleTree {
    /// All nodes of the tree, stored in a flat array.
    /// nodes[1] = root, nodes[2], nodes[3] = children of root, etc.
    /// nodes[n..2n] = leaf hashes (for a tree with n leaves).
    nodes: Vec<[u8; 32]>,
    /// Number of leaves.
    num_leaves: usize,
}

/// A Merkle authentication path (proof of inclusion).
#[derive(Clone, Debug)]
pub struct MerkleProof {
    /// Sibling hashes from leaf to root.
    pub siblings: Vec<[u8; 32]>,
    /// The leaf data (raw bytes).
    pub leaf_data: Vec<u8>,
    /// Index of the leaf.
    pub index: usize,
}

impl MerkleTree {
    /// Build a Merkle tree from leaf data.
    ///
    /// `leaves` is a vector of byte slices, one per leaf.
    /// The number of leaves must be a power of 2.
    pub fn new(leaves: &[Vec<u8>]) -> Self {
        let n = leaves.len();
        assert!(n.is_power_of_two(), "Number of leaves must be a power of 2");
        assert!(n >= 2, "Need at least 2 leaves");

        // Allocate flat array: indices 1..2n
        // Index 0 is unused; index 1 is root.
        let mut nodes = vec![[0u8; 32]; 2 * n];

        // Hash leaves into positions n..2n-1
        // KNOWN ISSUE: leaf hash does not include the index. This means a
        // malicious prover could potentially swap leaves between positions
        // without invalidating the Merkle proof. The FRI query structure
        // provides some position binding, but this is not a complete defense.
        // See hash.rs for details and the tracking TODO.
        for i in 0..n {
            nodes[n + i] = hash_leaf(&leaves[i]);
        }

        // Build internal nodes bottom-up
        for i in (1..n).rev() {
            nodes[i] = hash_node(&nodes[2 * i], &nodes[2 * i + 1]);
        }

        Self {
            nodes,
            num_leaves: n,
        }
    }

    /// Get the Merkle root.
    pub fn root(&self) -> [u8; 32] {
        self.nodes[1]
    }

    /// Open the leaf at position `index`, producing an authentication path.
    pub fn open(&self, index: usize, leaf_data: &[u8]) -> MerkleProof {
        assert!(index < self.num_leaves, "Index out of range");

        let mut siblings = Vec::new();
        let mut pos = self.num_leaves + index;

        while pos > 1 {
            // Sibling is the other child of our parent
            let sibling = pos ^ 1;
            siblings.push(self.nodes[sibling]);
            pos >>= 1;
        }

        MerkleProof {
            siblings,
            leaf_data: leaf_data.to_vec(),
            index,
        }
    }

    /// Number of leaves.
    pub fn num_leaves(&self) -> usize {
        self.num_leaves
    }
}

/// Verify a Merkle proof against an expected root.
pub fn verify_merkle_proof(proof: &MerkleProof, root: &[u8; 32]) -> bool {
    let mut current = hash_leaf(&proof.leaf_data);
    let mut index = proof.index;

    for sibling in &proof.siblings {
        if index & 1 == 0 {
            current = hash_node(&current, sibling);
        } else {
            current = hash_node(sibling, &current);
        }
        index >>= 1;
    }

    current == *root
}

impl MerkleProof {
    /// Serialize the proof to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Index (4 bytes, little-endian)
        bytes.extend_from_slice(&(self.index as u32).to_le_bytes());
        // Number of siblings (4 bytes)
        bytes.extend_from_slice(&(self.siblings.len() as u32).to_le_bytes());
        // Siblings
        for sib in &self.siblings {
            bytes.extend_from_slice(sib);
        }
        // Leaf data length (4 bytes)
        bytes.extend_from_slice(&(self.leaf_data.len() as u32).to_le_bytes());
        // Leaf data
        bytes.extend_from_slice(&self.leaf_data);
        bytes
    }

    /// Deserialize a proof from bytes.
    pub fn from_bytes(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < 8 {
            return None;
        }
        let mut offset = 0;

        let index = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;

        let num_siblings = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;

        let mut siblings = Vec::with_capacity(num_siblings);
        for _ in 0..num_siblings {
            if offset + 32 > data.len() {
                return None;
            }
            let mut sib = [0u8; 32];
            sib.copy_from_slice(&data[offset..offset + 32]);
            siblings.push(sib);
            offset += 32;
        }

        if offset + 4 > data.len() {
            return None;
        }
        let leaf_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;

        if offset + leaf_len > data.len() {
            return None;
        }
        let leaf_data = data[offset..offset + leaf_len].to_vec();
        offset += leaf_len;

        Some((
            MerkleProof {
                siblings,
                leaf_data,
                index,
            },
            offset,
        ))
    }
}
