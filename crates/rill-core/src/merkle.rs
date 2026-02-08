//! BLAKE3 Merkle tree for transaction commitment.
//!
//! Uses domain-separated hashing to prevent second-preimage attacks:
//! - Leaf hash: `BLAKE3(0x00 || data)`
//! - Internal node: `BLAKE3(0x01 || left || right)`
//!
//! Odd-length layers are padded by duplicating the last element.
//! Empty trees produce [`Hash256::ZERO`].

use serde::{Deserialize, Serialize};

use crate::types::Hash256;

/// Domain separation prefix for leaf hashes.
const LEAF_PREFIX: u8 = 0x00;

/// Domain separation prefix for internal node hashes.
const NODE_PREFIX: u8 = 0x01;

/// Compute a domain-separated leaf hash: `BLAKE3(0x00 || data)`.
pub fn leaf_hash(data: &Hash256) -> Hash256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[LEAF_PREFIX]);
    hasher.update(data.as_bytes());
    Hash256(hasher.finalize().into())
}

/// Compute a domain-separated internal node hash: `BLAKE3(0x01 || left || right)`.
pub fn node_hash(left: &Hash256, right: &Hash256) -> Hash256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[NODE_PREFIX]);
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    Hash256(hasher.finalize().into())
}

/// Compute the Merkle root from a slice of leaf values (typically transaction IDs).
///
/// Returns [`Hash256::ZERO`] for an empty slice.
/// This is more efficient than building a full [`MerkleTree`] when proofs are not needed.
pub fn merkle_root(leaves: &[Hash256]) -> Hash256 {
    if leaves.is_empty() {
        return Hash256::ZERO;
    }

    let mut current: Vec<Hash256> = leaves.iter().map(leaf_hash).collect();

    while current.len() > 1 {
        current = next_layer(&current);
    }

    current[0]
}

/// Compute the next layer of the tree from the current one.
///
/// Pairs adjacent hashes with [`node_hash`]. Duplicates the last element
/// when the layer has an odd number of entries.
fn next_layer(layer: &[Hash256]) -> Vec<Hash256> {
    let mut next = Vec::with_capacity(layer.len().div_ceil(2));
    let mut i = 0;
    while i < layer.len() {
        let left = &layer[i];
        let right = if i + 1 < layer.len() {
            &layer[i + 1]
        } else {
            left
        };
        next.push(node_hash(left, right));
        i += 2;
    }
    next
}

/// Full Merkle tree supporting root computation and proof generation.
///
/// Stores all intermediate layers so that inclusion proofs can be extracted
/// for any leaf.
#[derive(Clone, Debug)]
pub struct MerkleTree {
    /// Original leaf values passed to [`from_leaves`](Self::from_leaves).
    leaves: Vec<Hash256>,
    /// `layers[0]` = leaf hashes, `layers[last]` = `[root]`.
    layers: Vec<Vec<Hash256>>,
}

impl MerkleTree {
    /// Build a Merkle tree from leaf values (typically transaction IDs).
    pub fn from_leaves(leaves: &[Hash256]) -> Self {
        if leaves.is_empty() {
            return Self {
                leaves: Vec::new(),
                layers: Vec::new(),
            };
        }

        let mut layers = Vec::new();
        let leaf_layer: Vec<Hash256> = leaves.iter().map(leaf_hash).collect();
        layers.push(leaf_layer);

        while layers.last().unwrap().len() > 1 {
            let prev = layers.last().unwrap();
            layers.push(next_layer(prev));
        }

        Self {
            leaves: leaves.to_vec(),
            layers,
        }
    }

    /// The Merkle root. Returns [`Hash256::ZERO`] for an empty tree.
    pub fn root(&self) -> Hash256 {
        self.layers
            .last()
            .and_then(|l| l.first())
            .copied()
            .unwrap_or(Hash256::ZERO)
    }

    /// Number of leaves in the tree.
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// Generate an inclusion proof for the leaf at `index`.
    ///
    /// Returns `None` if the index is out of bounds or the tree is empty.
    pub fn proof(&self, index: usize) -> Option<MerkleProof> {
        if self.leaves.is_empty() || index >= self.leaves.len() {
            return None;
        }

        let mut path = Vec::new();
        let mut pos = index;

        // Walk from leaf layer to just below the root
        for layer in &self.layers[..self.layers.len() - 1] {
            let sibling_pos = pos ^ 1;
            let sibling = if sibling_pos < layer.len() {
                layer[sibling_pos]
            } else {
                // Odd layer: last element's sibling is itself (duplication)
                layer[pos]
            };

            let side = if pos % 2 == 0 {
                Side::Right
            } else {
                Side::Left
            };

            path.push(ProofStep {
                hash: sibling,
                side,
            });
            pos /= 2;
        }

        Some(MerkleProof {
            leaf_index: index,
            leaf: self.leaves[index],
            path,
        })
    }
}

/// Which side a sibling hash is on relative to the current node.
#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub enum Side {
    /// Sibling is on the left (we are on the right).
    Left,
    /// Sibling is on the right (we are on the left).
    Right,
}

/// A single step in a Merkle inclusion proof.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct ProofStep {
    /// The sibling hash at this level of the tree.
    pub hash: Hash256,
    /// Which side the sibling is on.
    pub side: Side,
}

/// Merkle inclusion proof for a single leaf.
///
/// Proves that `leaf` is included in a tree with a given root by providing
/// the sibling hashes along the path from the leaf to the root.
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
    bincode::Encode, bincode::Decode,
)]
pub struct MerkleProof {
    /// Index of the leaf in the original tree.
    pub leaf_index: usize,
    /// The original leaf value (e.g. a transaction ID).
    pub leaf: Hash256,
    /// Sibling hashes from leaf level up to root.
    pub path: Vec<ProofStep>,
}

impl MerkleProof {
    /// Verify this proof against an expected Merkle root.
    ///
    /// Recomputes the root from the leaf and sibling path, then compares.
    pub fn verify(&self, expected_root: &Hash256) -> bool {
        let mut current = leaf_hash(&self.leaf);

        for step in &self.path {
            current = match step.side {
                Side::Left => node_hash(&step.hash, &current),
                Side::Right => node_hash(&current, &step.hash),
            };
        }

        current == *expected_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> Hash256 {
        Hash256([byte; 32])
    }

    // --- Domain separation ---

    #[test]
    fn leaf_hash_differs_from_node_hash() {
        // Even with identical 32-byte input, leaf and node hashes must differ
        // due to the prefix byte.
        let a = h(0xAA);
        let lh = leaf_hash(&a);
        // node_hash with same data as if it were two 32-byte halves:
        // This can't be the same as a leaf hash because the prefix differs.
        let nh = node_hash(&a, &a);
        assert_ne!(lh, nh);
    }

    #[test]
    fn leaf_hash_deterministic() {
        let a = h(0x01);
        assert_eq!(leaf_hash(&a), leaf_hash(&a));
    }

    #[test]
    fn node_hash_deterministic() {
        let a = h(0x01);
        let b = h(0x02);
        assert_eq!(node_hash(&a, &b), node_hash(&a, &b));
    }

    #[test]
    fn node_hash_order_matters() {
        let a = h(0x01);
        let b = h(0x02);
        assert_ne!(node_hash(&a, &b), node_hash(&b, &a));
    }

    #[test]
    fn leaf_hash_changes_with_input() {
        assert_ne!(leaf_hash(&h(0x01)), leaf_hash(&h(0x02)));
    }

    // --- merkle_root ---

    #[test]
    fn merkle_root_empty() {
        assert_eq!(merkle_root(&[]), Hash256::ZERO);
    }

    #[test]
    fn merkle_root_single() {
        let a = h(0xAA);
        assert_eq!(merkle_root(&[a]), leaf_hash(&a));
    }

    #[test]
    fn merkle_root_two() {
        let a = h(0x01);
        let b = h(0x02);
        let expected = node_hash(&leaf_hash(&a), &leaf_hash(&b));
        assert_eq!(merkle_root(&[a, b]), expected);
    }

    #[test]
    fn merkle_root_three_odd() {
        let a = h(0x01);
        let b = h(0x02);
        let c = h(0x03);
        // Layer 0: [lh(a), lh(b), lh(c)]
        // Layer 1: [node(lh(a), lh(b)), node(lh(c), lh(c))]  -- c duplicated
        // Layer 2: [node(layer1[0], layer1[1])]
        let la = leaf_hash(&a);
        let lb = leaf_hash(&b);
        let lc = leaf_hash(&c);
        let n01 = node_hash(&la, &lb);
        let n22 = node_hash(&lc, &lc);
        let expected = node_hash(&n01, &n22);
        assert_eq!(merkle_root(&[a, b, c]), expected);
    }

    #[test]
    fn merkle_root_four_balanced() {
        let leaves: Vec<Hash256> = (1..=4).map(|i| h(i)).collect();
        let la = leaf_hash(&leaves[0]);
        let lb = leaf_hash(&leaves[1]);
        let lc = leaf_hash(&leaves[2]);
        let ld = leaf_hash(&leaves[3]);
        let n01 = node_hash(&la, &lb);
        let n23 = node_hash(&lc, &ld);
        let expected = node_hash(&n01, &n23);
        assert_eq!(merkle_root(&leaves), expected);
    }

    #[test]
    fn merkle_root_deterministic() {
        let leaves: Vec<Hash256> = (0..7).map(|i| h(i)).collect();
        assert_eq!(merkle_root(&leaves), merkle_root(&leaves));
    }

    #[test]
    fn merkle_root_changes_with_leaf() {
        let a = vec![h(1), h(2), h(3)];
        let b = vec![h(1), h(2), h(4)];
        assert_ne!(merkle_root(&a), merkle_root(&b));
    }

    #[test]
    fn merkle_root_order_matters() {
        let a = vec![h(1), h(2)];
        let b = vec![h(2), h(1)];
        assert_ne!(merkle_root(&a), merkle_root(&b));
    }

    // --- MerkleTree ---

    #[test]
    fn tree_empty() {
        let tree = MerkleTree::from_leaves(&[]);
        assert_eq!(tree.root(), Hash256::ZERO);
        assert_eq!(tree.leaf_count(), 0);
        assert!(tree.proof(0).is_none());
    }

    #[test]
    fn tree_root_matches_standalone() {
        for count in 1..=10 {
            let leaves: Vec<Hash256> = (0..count).map(|i| h(i as u8)).collect();
            let tree = MerkleTree::from_leaves(&leaves);
            assert_eq!(
                tree.root(),
                merkle_root(&leaves),
                "mismatch at count={count}"
            );
        }
    }

    #[test]
    fn tree_leaf_count() {
        let leaves: Vec<Hash256> = (0..5).map(|i| h(i)).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        assert_eq!(tree.leaf_count(), 5);
    }

    #[test]
    fn tree_proof_out_of_bounds() {
        let tree = MerkleTree::from_leaves(&[h(1), h(2)]);
        assert!(tree.proof(2).is_none());
        assert!(tree.proof(100).is_none());
    }

    // --- Single-leaf proof ---

    #[test]
    fn proof_single_leaf() {
        let a = h(0xAA);
        let tree = MerkleTree::from_leaves(&[a]);
        let root = tree.root();

        let proof = tree.proof(0).unwrap();
        assert_eq!(proof.leaf, a);
        assert_eq!(proof.leaf_index, 0);
        assert!(proof.path.is_empty());
        assert!(proof.verify(&root));
    }

    // --- Two-leaf proofs ---

    #[test]
    fn proof_two_leaves() {
        let leaves = vec![h(0x01), h(0x02)];
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();

        let p0 = tree.proof(0).unwrap();
        assert_eq!(p0.leaf, h(0x01));
        assert_eq!(p0.path.len(), 1);
        assert!(p0.verify(&root));

        let p1 = tree.proof(1).unwrap();
        assert_eq!(p1.leaf, h(0x02));
        assert_eq!(p1.path.len(), 1);
        assert!(p1.verify(&root));
    }

    // --- Multi-leaf proofs (odd and even counts) ---

    #[test]
    fn proof_all_leaves_three() {
        let leaves = vec![h(1), h(2), h(3)];
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();

        for i in 0..3 {
            let proof = tree.proof(i).unwrap();
            assert_eq!(proof.leaf, leaves[i]);
            assert_eq!(proof.leaf_index, i);
            assert!(proof.verify(&root), "proof failed for leaf {i}");
        }
    }

    #[test]
    fn proof_all_leaves_four() {
        let leaves: Vec<Hash256> = (1..=4).map(|i| h(i)).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();

        for i in 0..4 {
            let proof = tree.proof(i).unwrap();
            assert!(proof.verify(&root), "proof failed for leaf {i}");
        }
    }

    #[test]
    fn proof_all_leaves_five() {
        let leaves: Vec<Hash256> = (1..=5).map(|i| h(i)).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();

        for i in 0..5 {
            let proof = tree.proof(i).unwrap();
            assert!(proof.verify(&root), "proof failed for leaf {i}");
        }
    }

    #[test]
    fn proof_all_leaves_large() {
        let leaves: Vec<Hash256> = (0..33).map(|i| h(i as u8)).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();

        for i in 0..33 {
            let proof = tree.proof(i).unwrap();
            assert!(proof.verify(&root), "proof failed for leaf {i}");
        }
    }

    // --- Proof depth ---

    #[test]
    fn proof_depth_power_of_two() {
        // 8 leaves → balanced tree of depth 3
        let leaves: Vec<Hash256> = (0..8).map(|i| h(i)).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let proof = tree.proof(0).unwrap();
        assert_eq!(proof.path.len(), 3);
    }

    #[test]
    fn proof_depth_non_power_of_two() {
        // 5 leaves → 3 layers above leaf: ceil(log2(5)) = 3
        let leaves: Vec<Hash256> = (0..5).map(|i| h(i)).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let proof = tree.proof(0).unwrap();
        assert_eq!(proof.path.len(), 3);
    }

    // --- Proof verification failures ---

    #[test]
    fn proof_verify_wrong_root() {
        let leaves = vec![h(1), h(2), h(3), h(4)];
        let tree = MerkleTree::from_leaves(&leaves);
        let proof = tree.proof(0).unwrap();

        let wrong_root = Hash256([0xFF; 32]);
        assert!(!proof.verify(&wrong_root));
    }

    #[test]
    fn proof_verify_tampered_leaf() {
        let leaves = vec![h(1), h(2), h(3), h(4)];
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();
        let mut proof = tree.proof(0).unwrap();

        proof.leaf = h(0xFF); // tamper
        assert!(!proof.verify(&root));
    }

    #[test]
    fn proof_verify_tampered_sibling() {
        let leaves = vec![h(1), h(2), h(3), h(4)];
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();
        let mut proof = tree.proof(0).unwrap();

        proof.path[0].hash = Hash256([0xFF; 32]); // tamper
        assert!(!proof.verify(&root));
    }

    #[test]
    fn proof_from_different_tree_fails() {
        let tree_a = MerkleTree::from_leaves(&[h(1), h(2)]);
        let tree_b = MerkleTree::from_leaves(&[h(3), h(4)]);

        let proof_a = tree_a.proof(0).unwrap();
        assert!(!proof_a.verify(&tree_b.root()));
    }

    // --- Proof serialization ---

    #[test]
    fn proof_bincode_roundtrip() {
        let leaves: Vec<Hash256> = (1..=5).map(|i| h(i)).collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let proof = tree.proof(2).unwrap();

        let encoded = bincode::encode_to_vec(&proof, bincode::config::standard()).unwrap();
        let (decoded, _): (MerkleProof, usize) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();

        assert_eq!(proof, decoded);
        assert!(decoded.verify(&tree.root()));
    }

    // --- Duplicate last element behavior ---

    #[test]
    fn odd_tree_last_leaf_proof_uses_duplication() {
        // With 3 leaves, leaf[2]'s sibling at the leaf layer is itself (duplicated).
        let leaves = vec![h(1), h(2), h(3)];
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();

        let proof = tree.proof(2).unwrap();
        assert!(proof.verify(&root));

        // First sibling should be the duplicate (leaf_hash of the same leaf)
        assert_eq!(proof.path[0].hash, leaf_hash(&h(3)));
        assert_eq!(proof.path[0].side, Side::Right);
    }

    #[test]
    fn single_leaf_differs_from_two_identical() {
        // A tree with [A] must have a different root than [A, A]
        // because [A] = leaf_hash(A), while [A, A] = node_hash(leaf_hash(A), leaf_hash(A)).
        let a = h(0xAA);
        let root_one = merkle_root(&[a]);
        let root_two = merkle_root(&[a, a]);
        assert_ne!(root_one, root_two);
    }
}
