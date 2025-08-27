use sha2::{Digest, Sha256};
use std::collections::HashMap;

// === Safe leaf encoding: length-prefix (u32 big-endian) ===
// Why? Concatenating "key:value" is ambiguous (e.g., "a::b").
// Length-prefixing eliminates ambiguity and is robust to any bytes (including NUL).
fn encode_leaf(key: &str, value: &str) -> Vec<u8> {
    let kb = key.as_bytes();
    let vb = value.as_bytes();
    let mut out = Vec::with_capacity(8 + kb.len() + vb.len());
    out.extend_from_slice(&(kb.len() as u32).to_be_bytes());
    out.extend_from_slice(kb);
    out.extend_from_slice(&(vb.len() as u32).to_be_bytes());
    out.extend_from_slice(vb);
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct MerkleNode {
    pub hash: Vec<u8>,
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
    /// Present only for leaf nodes (None for internal nodes)
    pub key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MerkleTree {
    pub root: Option<MerkleNode>,
    // Stores leaf hashes keyed by user-provided key (we don't store raw values here).
    leaf_map: HashMap<String, Vec<u8>>,
}

impl MerkleTree {
    /// Create an empty Merkle tree.
    pub fn new() -> Self {
        Self {
            root: None,
            leaf_map: HashMap::new(),
        }
    }

    /// Shared helper: compute a leaf hash from (key, value).
    /// Using a shared function guarantees tests and implementation stay in sync.
    fn compute_leaf_hash(key: &str, value: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(encode_leaf(key, value));
        hasher.finalize().to_vec()
    }

    /// Insert or update a (key, value). The leaf hash is recomputed and the tree rebuilt.
    pub fn insert(&mut self, key: &str, value: &str) {
        let hash = Self::compute_leaf_hash(key, value);
        self.leaf_map.insert(key.to_string(), hash);
        self.rebuild();
    }

    /// Remove a key (if it exists) and rebuild the tree.
    pub fn remove(&mut self, key: &str) {
        self.leaf_map.remove(key);
        self.rebuild();
    }

    /// Get a reference to the current root hash (if the tree is non-empty).
    pub fn get_root_hash(&self) -> Option<&Vec<u8>> {
        self.root.as_ref().map(|node| &node.hash)
    }

    /// Rebuild the entire Merkle tree from the current `leaf_map`.
    /// Notes:
    /// - Sort leaves by key (lexicographical) for deterministic root.
    /// - Pair nodes left-to-right; if odd, "promote" the last node.
    fn rebuild(&mut self) {
        if self.leaf_map.is_empty() {
            self.root = None;
            return;
        }

        // ✅ Determinism: sort leaves by key so the same set yields the same root.
        let mut leaves: Vec<_> = self.leaf_map.iter().collect();
        leaves.sort_by(|a, b| a.0.cmp(b.0));

        let mut nodes: Vec<MerkleNode> = leaves
            .into_iter()
            .map(|(k, h)| MerkleNode {
                hash: h.clone(),
                left: None,
                right: None,
                key: Some(k.clone()), // store key at leaves
            })
            .collect();

        // Bottom-up reduction: combine pairs into parents until a single root remains.
        while nodes.len() > 1 {
            let mut new_level = Vec::new();

            for chunk in nodes.chunks(2) {
                if chunk.len() == 2 {
                    // Parent hash = H(left.hash || right.hash)
                    let mut hasher = Sha256::new();
                    hasher.update(&chunk[0].hash);
                    hasher.update(&chunk[1].hash);
                    let hash = hasher.finalize().to_vec();

                    new_level.push(MerkleNode {
                        hash,
                        left: Some(Box::new(chunk[0].clone())),
                        right: Some(Box::new(chunk[1].clone())),
                        key: None, // internal node
                    });
                } else {
                    // Convention used here: with an odd count, promote the last node.
                    new_level.push(chunk[0].clone());
                }
            }

            nodes = new_level;
        }

        self.root = nodes.into_iter().next();
    }

    // ===================== Traversal & Views =====================

    /// Return the sorted keys (lexicographic) currently present in the tree.
    pub fn inorder_keys(&self) -> Vec<String> {
        let mut ks: Vec<String> = self.leaf_map.keys().cloned().collect();
        ks.sort();
        ks
    }

    /// Return all leaf (key, hash) pairs in lexicographic key order.
    pub fn leaves(&self) -> Vec<(String, Vec<u8>)> {
        let mut v: Vec<(String, Vec<u8>)> =
            self.leaf_map.iter().map(|(k, h)| (k.clone(), h.clone())).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    /// Preorder traversal returning node hashes from the current materialized tree.
    /// (Root → Left-subtree → Right-subtree)
    pub fn preorder_hashes(&self) -> Vec<Vec<u8>> {
        fn go(n: &MerkleNode, acc: &mut Vec<Vec<u8>>) {
            acc.push(n.hash.clone());
            if let Some(l) = &n.left { go(l, acc); }
            if let Some(r) = &n.right { go(r, acc); }
        }
        let mut out = Vec::new();
        if let Some(r) = &self.root {
            go(r, &mut out);
        }
        out
    }

    /// Count nodes (internal + leaves) in the current tree.
    pub fn node_count(&self) -> usize {
        fn cnt(n: &MerkleNode) -> usize {
            1
                + n.left.as_deref().map(cnt).unwrap_or(0)
                + n.right.as_deref().map(cnt).unwrap_or(0)
        }
        self.root.as_ref().map(cnt).unwrap_or(0)
    }

    // ===================== DIFF SUPPORT (find the “wrong” keys) =====================

    /// Return the exact set of differing keys between `self` and `other`.
    /// A key is included iff:
    /// - it exists in only one tree, OR
    /// - it exists in both trees but the leaf hashes differ.
    pub fn diff_keys(&self, other: &MerkleTree) -> Vec<String> {
        use std::collections::BTreeSet;

        // union of keys to compare
        let mut all_keys: BTreeSet<&String> = BTreeSet::new();
        for k in self.leaf_map.keys() { all_keys.insert(k); }
        for k in other.leaf_map.keys() { all_keys.insert(k); }

        let mut diffs: Vec<String> = Vec::new();

        for k in all_keys {
            match (self.leaf_map.get(k), other.leaf_map.get(k)) {
                (Some(h1), Some(h2)) => {
                    if h1 != h2 {
                        diffs.push(k.clone());
                    }
                }
                (Some(_), None) | (None, Some(_)) => {
                    diffs.push(k.clone());
                }
                (None, None) => unreachable!(),
            }
        }

        diffs
    }

    /// Convenience: return the first differing key in lexicographic order (if any).
    pub fn diff_first_key(&self, other: &MerkleTree) -> Option<String> {
        let mut diffs = self.diff_keys(other);
        diffs.sort();
        diffs.dedup();
        diffs.into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use super::encode_leaf;
    use std::collections::HashSet;
    use rand::rngs::StdRng;
    use rand::{SeedableRng, Rng};
    use rand::seq::SliceRandom;

    fn set<T: Eq + std::hash::Hash + Clone>(xs: &[T]) -> HashSet<T> {
        xs.iter().cloned().collect()
    }
  
    // Helper used by tests; it mirrors the production hashing logic.
    fn leaf_hash(key: &str, value: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(encode_leaf(key, value));
        hasher.finalize().to_vec()
    }

    // ───────────────────────── Basic tests ─────────────────────────

    #[test]
   fn test_single_leaf_root_equals_leaf_hash() {
    // 0) 🧪 Empty tree → no root
    let t0 = MerkleTree::new();
    assert!(
        t0.get_root_hash().is_none(),
        "An empty Merkle tree must not have a root"
    );

    // 1) 🌱 One leaf → root == that leaf's hash
    let mut t1 = MerkleTree::new();
    t1.insert("k", "v");

    // leaf_hash must hash the same encoding as the implementation (length-prefix)

    let expected = leaf_hash("k", "v");
    let got = t1.get_root_hash().expect("single-leaf tree must have a root");
    assert_eq!(
        got, &expected,
        "🌱 For a one-leaf tree, the root must equal that leaf's hash"
    );

    // 2) 🌿 Two leaves → changing a value must change the root
    let mut t2 = MerkleTree::new();
    t2.insert("key1", "value1");
    t2.insert("key2", "value2");
    let root_before = t2.get_root_hash().unwrap().clone();

    t2.insert("key2", "new_value"); // update existing key
    let root_after = t2.get_root_hash().unwrap().clone();
    assert_ne!(
        root_before, root_after,
        "Updating a value under the same key must change the root"
    );

    // 3) 🍂 Removing keys → tree is empty only after removing ALL keys
    t2.remove("key1"); // still has key2
    assert!(
        t2.get_root_hash().is_some(),
        "After removing only one of two keys, the tree must still have a root"
    );

    t2.remove("key2"); // now empty
    assert!(
        t2.get_root_hash().is_none(),
        "After removing all keys, the tree must be empty and have no root"
    );
}
    #[test]
    fn test_root_hash_is_32_bytes() {
        // 🔒 SHA-256 always yields exactly 32 bytes.
        let mut tree = MerkleTree::new();
        tree.insert("a", "1");
        let root = tree.get_root_hash().unwrap();
        assert_eq!(root.len(), 32, "🔒 Root hash must be 32 bytes (SHA-256)");
    }

    #[test]
    fn test_insert_same_value_idempotent() {
        // ➕➕ Reinserting the same (key, value) must NOT change the root.
        let mut tree = MerkleTree::new();
        tree.insert("k1", "v1");
        let r1 = tree.get_root_hash().unwrap().clone();

        tree.insert("k1", "v1");
        let r2 = tree.get_root_hash().unwrap().clone();

        assert_eq!(r1, r2, "➕➕ Reinserting the same data must not change the root");
    }

    #[test]
    fn test_update_value_changes_root() {
        // 🔄 Updating the value for an existing key MUST change the root.
        let mut tree = MerkleTree::new();
        tree.insert("k1", "v1");
        tree.insert("k2", "v2");
        let r_before = tree.get_root_hash().unwrap().clone();

        tree.insert("k2", "v2'");
        let r_after = tree.get_root_hash().unwrap().clone();

        assert_ne!(
            r_before, r_after,
            "🔄 Updating an existing key with a different value must change the root"
        );
    }

    #[test]
    fn test_remove_nonexistent_keeps_root() {
        // 🗑️ Removing a non-existent key should be a no-op for the root.
        let mut tree = MerkleTree::new();
        tree.insert("a", "1");
        tree.insert("b", "2");
        let r_before = tree.get_root_hash().unwrap().clone();

        tree.remove("c");
        let r_after = tree.get_root_hash().unwrap().clone();

        assert_eq!(
            r_before, r_after,
            "🗑️ Removing a missing key must not change the root"
        );
    }

    #[test]
    fn test_odd_number_of_leaves_promotes_one_leaf() {
        // 🌳 With 3 leaves, exactly one child of the root should be a leaf (promoted),
        // while the other is an internal node (a parent).
        let mut tree = MerkleTree::new();
        tree.insert("k1", "v1");
        tree.insert("k2", "v2");
        tree.insert("k3", "v3");

        let root = tree.root.as_ref().expect("🌳 Root must exist");
        let left_is_leaf = root
            .left
            .as_ref()
            .map(|n| n.left.is_none() && n.right.is_none())
            .unwrap_or(false);
        let right_is_leaf = root
            .right
            .as_ref()
            .map(|n| n.left.is_none() && n.right.is_none())
            .unwrap_or(false);

        assert!(
            left_is_leaf ^ right_is_leaf,
            "🌳 With 3 leaves, exactly one child of the root should be a leaf"
        );
    }

    #[test]
    fn test_many_items_and_unicode_stability() {
        // 🌐 Reinserting the same Unicode-heavy dataset should keep the root unchanged.
        let mut tree = MerkleTree::new();

        let data = vec![
            ("α", "1"), ("β", "2"), ("γ", "3"), ("中文", "值"),
            ("emoji🙂", "ok"), ("key6", "v6"), ("key7", "v7"),
            ("key8", "v8"), ("key9", "v9"), ("key10", "v10"),
        ];

        for (k, v) in &data {
            tree.insert(k, v);
        }
        let r1 = tree.get_root_hash().unwrap().clone();

        // Reinsert the exact same data (no changes)
        for (k, v) in &data {
            tree.insert(k, v);
        }
        let r2 = tree.get_root_hash().unwrap().clone();

        assert_eq!(r1, r2, "🌐 Rebuilding with identical data must keep the root");
    }

    // ───────────────────────── Hard/edge tests ─────────────────────────

    /// 🧭 Determinism (even count): same set, different insertion orders → same root.
    #[test]
    fn hard_determinism_even_count_different_insert_orders() {
        let pairs = vec![("k1","v1"), ("k2","v2"), ("k3","v3"), ("k4","v4")];

        // Order A
        let mut t1 = MerkleTree::new();
        for (k,v) in pairs.iter() { t1.insert(k, v); }
        let r1 = t1.get_root_hash().unwrap().clone();

        // Order B (reversed)
        let mut t2 = MerkleTree::new();
        for (k,v) in pairs.iter().rev() { t2.insert(k, v); }
        let r2 = t2.get_root_hash().unwrap().clone();

        assert_eq!(r1, r2, "🧭 Root must be deterministic for the same set (even count)");
    }

    /// 🧭 Determinism (odd count): same set, different insertion orders → same root.
    #[test]
    fn hard_determinism_odd_count_different_insert_orders() {
        let pairs = vec![("a","1"), ("b","2"), ("c","3")];

        let mut t1 = MerkleTree::new();
        for (k,v) in pairs.iter() { t1.insert(k, v); }
        let r1 = t1.get_root_hash().unwrap().clone();

        let mut t2 = MerkleTree::new();
        t2.insert("b","2");
        t2.insert("c","3");
        t2.insert("a","1");
        let r2 = t2.get_root_hash().unwrap().clone();

        assert_eq!(r1, r2, "🧭 Root must be deterministic for the same set (odd count)");
    }

    /// ⚠️ Former ambiguity test (“key:value”) now passes because we use length-prefix.
    #[test]
    fn hard_serialization_ambiguity_colon_separator() {
        // Set A
        let mut t1 = MerkleTree::new();
        t1.insert("x", "y");
        t1.insert("a:", "b");

        // Set B (logically different)
        let mut t2 = MerkleTree::new();
        t2.insert("x", "y");
        t2.insert("a", ":b");

        let r1 = t1.get_root_hash().unwrap().clone();
        let r2 = t2.get_root_hash().unwrap().clone();

        assert_ne!(
            r1, r2,
            "⚠️ Different datasets must not produce the same root (length-prefix prevents collisions)"
        );
    }

    /// 🧪 Two independent trees over the same dataset (different orders) → same root.
    #[test]
    fn hard_two_independent_trees_same_set_same_root() {
        let set1 = vec![("u","1"), ("v","2"), ("w","3"), ("z","4"), ("q","5")];

        // Tree 1: order 1
        let mut t1 = MerkleTree::new();
        for (k,v) in [&set1[2], &set1[0], &set1[4], &set1[1], &set1[3]] {
            t1.insert(k, v);
        }
        let r1 = t1.get_root_hash().unwrap().clone();

        // Tree 2: same set, different order
        let mut t2 = MerkleTree::new();
        for (k,v) in [&set1[4], &set1[3], &set1[2], &set1[1], &set1[0]] {
            t2.insert(k, v);
        }
        let r2 = t2.get_root_hash().unwrap().clone();

        assert_eq!(r1, r2, "🧪 Identical datasets must yield identical roots");
    }

    /// 🧪 Manual check (2 leaves): root = H( H(k1,v1) || H(k2,v2) ) assuming k1<k2 after sort.
    #[test]
    fn hard_manual_root_two_leaves() {
        let (k1, v1) = ("a", "A");
        let (k2, v2) = ("b", "B");
        assert!(k1 < k2, "Assumption for the test: k1 < k2 after sorting by key");

        let h1 = leaf_hash(k1, v1);
        let h2 = leaf_hash(k2, v2);

        let mut tree = MerkleTree::new();
        tree.insert(k1, v1);
        tree.insert(k2, v2);

        let got = tree.get_root_hash().unwrap();

        let mut hasher = Sha256::new();
        hasher.update(&h1);
        hasher.update(&h2);
        let expect = hasher.finalize().to_vec();

        assert_eq!(got, &expect, "🧪 Root with two leaves must equal H(H1 || H2)");
    }

    /// 🌿 Edge cases: empty strings and NUL bytes should work (length-prefix handles them).
    #[test]
    fn hard_empty_and_nul_bytes() {
        let cases = vec![
            ("", ""),
            ("", "nonempty"),
            ("nonempty", ""),
            ("has\0nul", "v"),
            ("k", "va\0lue"),
            ("a\0b", "\0\0\0"),
        ];

        let mut t = MerkleTree::new();
        for (k, v) in &cases { t.insert(k, v); }
        let r1 = t.get_root_hash().unwrap().clone();

        // Reinserting identical data should NOT change the root.
        for (k, v) in &cases { t.insert(k, v); }
        let r2 = t.get_root_hash().unwrap().clone();

        assert_eq!(r1, r2, "🌿 Reinserting identical data must keep the root (even with NUL bytes)");
    }

    /// 🔁 Remove a leaf and then reinsert the exact same leaf → original root restored.
    #[test]
    fn hard_remove_then_reinsert_restores_root() {
        let items = vec![("k1","v1"), ("k2","v2"), ("k3","v3")];

        let mut t = MerkleTree::new();
        for (k, v) in &items { t.insert(k, v); }
        let r0 = t.get_root_hash().unwrap().clone();

        // Remove one leaf
        t.remove("k2");
        let r_removed = t.get_root_hash().unwrap().clone();
        assert_ne!(r0, r_removed, "🔁 Removing a leaf must change the root");

        // Reinsert the same leaf
        t.insert("k2", "v2");
        let r_restored = t.get_root_hash().unwrap().clone();

        assert_eq!(r0, r_restored, "🔁 Reinserting the same leaf must restore the original root");
    }

    /// 🧩 Distinguish between "update existing key" vs "insert a new key".
    #[test]
    fn hard_update_vs_new_key_diff() {
        // Baseline
        let mut base = MerkleTree::new();
        base.insert("k1", "v1");
        base.insert("k2", "v2");
        let r_base = base.get_root_hash().unwrap().clone();

        // Case A: update k2
        let mut a = base.clone();
        a.insert("k2", "v2_updated");
        let r_a = a.get_root_hash().unwrap().clone();

        // Case B: keep k2, insert k3
        let mut b = base.clone();
        b.insert("k3", "v3");
        let r_b = b.get_root_hash().unwrap().clone();

        assert_ne!(r_base, r_a, "🧩 Updating an existing key must change the root");
        assert_ne!(r_base, r_b, "🧩 Inserting a new key must change the root");
        assert_ne!(r_a, r_b, "🧩 Updating vs adding a key should typically produce different roots");
    }

    /// 🔀 Multiple idempotent updates (same value) must not change the root; a real change must.
    #[test]
    fn hard_multiple_idempotent_updates() {
        let mut t = MerkleTree::new();
        t.insert("k", "v");
        let r1 = t.get_root_hash().unwrap().clone();

        // Idempotent updates
        for _ in 0..10 {
            t.insert("k", "v");
            let r = t.get_root_hash().unwrap().clone();
            assert_eq!(r1, r, "🔀 Re-applying the same value must keep the root");
        }

        // Real change
        t.insert("k", "v2");
        let r2 = t.get_root_hash().unwrap().clone();
        assert_ne!(r1, r2, "🔀 Changing to a different value must change the root");
    }

    /// 🌲 With exactly three leaves: one child of the root is a leaf, the other is an internal node.
    #[test]
    fn hard_shape_three_leaves() {
        let mut t = MerkleTree::new();
        t.insert("a", "1");
        t.insert("b", "2");
        t.insert("c", "3");

        let root = t.root.as_ref().expect("Root must exist");
        let left_is_leaf = root.left.as_ref().map(|n| n.left.is_none() && n.right.is_none()).unwrap_or(false);
        let right_is_leaf = root.right.as_ref().map(|n| n.left.is_none() && n.right.is_none()).unwrap_or(false);

        assert!(
            left_is_leaf ^ right_is_leaf,
            "🌲 With three leaves, exactly one child of the root must be a leaf (promoted node)"
        );
    }

    /// 🧵 Cloning preserves the root; mutating the clone diverges the root.
    #[test]
    fn hard_clone_then_mutate_diverges() {
        let mut t1 = MerkleTree::new();
        for (k, v) in &[("k1","v1"), ("k2","v2"), ("k3","v3")] { t1.insert(k, v); }

        let t2 = t1.clone();
        let r1 = t1.get_root_hash().unwrap().clone();
        let r2 = t2.get_root_hash().unwrap().clone();
        assert_eq!(r1, r2, "🧵 Cloning a tree must preserve the root");

        let mut t2m = t2.clone();
        t2m.insert("k2", "v2_new");
        let r2m = t2m.get_root_hash().unwrap().clone();
        assert_ne!(r1, r2m, "🧵 Mutating a clone must change its root relative to the original");
    }

    /// 🧱 Mini stress test: insert 200 items, delete half, reinsert them, and ensure root restoration.
    #[test]
    fn hard_stress_delete_half_then_restore() {
        let n = 200usize;
        let all: Vec<(String, String)> = (0..n).map(|i| (format!("k{i}"), format!("v{i}"))).collect();

        let mut t = MerkleTree::new();
        for (k, v) in &all { t.insert(k, v); }
        let r0 = t.get_root_hash().unwrap().clone();

        // Remove first half
        for i in 0..(n/2) { t.remove(&format!("k{i}")); }
        let r_del = t.get_root_hash().unwrap().clone();
        assert_ne!(r0, r_del, "🧱 Removing half of the items must change the root");

        // Reinsert them
        for i in 0..(n/2) { t.insert(&format!("k{i}"), &format!("v{i}")); }
        let r1 = t.get_root_hash().unwrap().clone();

        assert_eq!(r0, r1, "🧱 Reinserting the exact items must restore the original root");
    }

    /// 🧬 Manual check (4 leaves):
    /// After sorting keys as [k1,k2,k3,k4], the root must be:
    /// H( H(H1||H2) || H(H3||H4) ).
    #[test]
    fn hard_manual_root_four_leaves() {
        let items = vec![("k1","v1"), ("k2","v2"), ("k3","v3"), ("k4","v4")];

        let mut t = MerkleTree::new();
        for (k,v) in &items { t.insert(k,v); }
        let got = t.get_root_hash().unwrap().clone();

        // Sort by key and compute manually.
        let mut sorted = items.clone();
        sorted.sort_by(|a,b| a.0.cmp(&b.0));

        let h: Vec<Vec<u8>> = sorted.iter().map(|(k,v)| leaf_hash(k,v)).collect();

        let mut h12 = Sha256::new();
        h12.update(&h[0]); h12.update(&h[1]);
        let h12 = h12.finalize();

        let mut h34 = Sha256::new();
        h34.update(&h[2]); h34.update(&h[3]);
        let h34 = h34.finalize();

        let mut hroot = Sha256::new();
        hroot.update(h12);
        hroot.update(h34);
        let expect = hroot.finalize().to_vec();

        assert_eq!(got, expect, "🧬 Root with four leaves must equal H( H(H1||H2) || H(H3||H4) )");
    }
     #[test]
    fn diff_no_difference_returns_empty() {
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();
        for (k,v) in &[("k1","v1"), ("k2","v2"), ("k3","v3")] {
            a.insert(k, v);
            b.insert(k, v);
        }
        assert_eq!(a.get_root_hash(), b.get_root_hash(), "sanity: same data → same root");

        let diffs = a.diff_keys(&b);
        assert!(diffs.is_empty(), "No differences expected");
        assert_eq!(a.diff_first_key(&b), None, "diff_first_key must be None when identical");
    }

    #[test]
    fn diff_single_value_change_returns_that_key() {
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();
        a.insert("k1","v1");
        a.insert("k2","v2");
        b.insert("k1","v1");
        b.insert("k2","DIFF"); // changed

        let diffs = set(&a.diff_keys(&b));
        assert_eq!(diffs, set(&[String::from("k2")]));
        assert_eq!(a.diff_first_key(&b), Some("k2".into()));
    }

    #[test]
    fn diff_missing_key_is_detected() {
        // b is missing k3
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();
        for (k,v) in &[("k1","v1"), ("k2","v2"), ("k3","v3")] { a.insert(k,v) }
        for (k,v) in &[("k1","v1"), ("k2","v2")] { b.insert(k,v) }

        let diffs = set(&a.diff_keys(&b));
        assert_eq!(diffs, set(&[String::from("k3")]));
        assert_eq!(a.diff_first_key(&b), Some("k3".into()));
    }

    #[test]
    fn diff_extra_key_is_detected() {
        // b has an extra kX
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();
        for (k,v) in &[("k1","v1"), ("k2","v2")] { a.insert(k,v) }
        for (k,v) in &[("k1","v1"), ("k2","v2"), ("kX","vX")] { b.insert(k,v) }

        let diffs = set(&a.diff_keys(&b));
        assert_eq!(diffs, set(&[String::from("kX")]));
        assert_eq!(a.diff_first_key(&b), Some("kX".into()));
    }

    #[test]
    fn diff_multiple_keys_detected_unordered() {
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();
        for (k,v) in &[("a","1"), ("b","2"), ("c","3"), ("d","4")] {
            a.insert(k,v);
            b.insert(k,v);
        }
        // Change two values in b
        b.insert("b","2'"); 
        b.insert("d","4'");

        let diffs = set(&a.diff_keys(&b));
        let expect = set(&[String::from("b"), String::from("d")]);
        assert_eq!(diffs, expect, "Must detect all differing keys");
        assert!(expect.contains(&a.diff_first_key(&b).unwrap()));
    }

    #[test]
    fn diff_empty_vs_nonempty_returns_all_keys() {
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();

        for (k,v) in &[("x","1"), ("y","2"), ("z","3")] { a.insert(k,v); }
        // b stays empty

        let diffs = set(&a.diff_keys(&b));
        assert_eq!(diffs, set(&["x".to_string(),"y".to_string(),"z".to_string()]));
        assert!(a.diff_first_key(&b).is_some());
    }

    #[test]
    fn diff_unicode_and_nul_bytes() {
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();

        let cases = vec![
            ("α", "1"),
            ("中文", "值"),
            ("emoji🙂", "ok"),
            ("nu\0l", "v"),
            ("k", "va\0lue"),
        ];
        for (k,v) in &cases { a.insert(k,v); b.insert(k,v); }
        // change one
        b.insert("中文", "变"); // different value

        let diffs = set(&a.diff_keys(&b));
        assert_eq!(diffs, set(&[String::from("中文")]));
        assert_eq!(a.diff_first_key(&b), Some("中文".into()));
    }

    #[test]
    fn diff_structure_mismatch_due_to_odd_promotion() {
        // a has 3 keys, b has 4 keys → internal shape differs (promotion on a).
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();
        for (k,v) in &[("k1","v1"), ("k2","v2"), ("k3","v3")] { a.insert(k,v); }
        for (k,v) in &[("k1","v1"), ("k2","v2"), ("k3","v3"), ("k4","v4")] { b.insert(k,v); }

        // Expect the extra key to be reported
        let diffs = set(&a.diff_keys(&b));
        assert_eq!(diffs, set(&[String::from("k4")]));
        assert_eq!(a.diff_first_key(&b), Some("k4".into()));
    }

    #[test]
    fn diff_collects_all_keys_when_both_sides_have_unique_extras() {
        // a has kA extra, b has kB extra
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();

        for (k,v) in &[("k1","v1"), ("k2","v2")] { a.insert(k,v); b.insert(k,v); }
        a.insert("kA","vA");
        b.insert("kB","vB");

        let diffs = set(&a.diff_keys(&b));
        let expect = set(&[String::from("kA"), String::from("kB")]);
        assert_eq!(diffs, expect);
        assert!(expect.contains(&a.diff_first_key(&b).unwrap()));
    }

    #[test]
    fn diff_when_both_changed_same_key() {
        // both change k2 to different values
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();
        a.insert("k1","v1");
        a.insert("k2","A");
        b.insert("k1","v1");
        b.insert("k2","B");

        let diffs = a.diff_keys(&b);
        // do not require uniqueness in return (function may push twice),
        // but the set must contain k2.
        let s = set(&diffs);
        assert!(s.contains("k2"), "k2 must be reported");
        assert_eq!(a.diff_first_key(&b), Some("k2".into()));
    }

    #[test]
    fn diff_remove_then_reinsert_restores_no_diff() {
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();

        for (k,v) in &[("k1","v1"), ("k2","v2"), ("k3","v3")] {
            a.insert(k,v);
            b.insert(k,v);
        }
        // remove in b, then reinsert with same value
        b.remove("k2");
        let diffs1 = a.diff_keys(&b);
        assert!(set(&diffs1).contains("k2"));

        b.insert("k2","v2");
        let diffs2 = a.diff_keys(&b);
        assert!(diffs2.is_empty(), "After reinserting the same value, trees should match");
    }

    #[test]
    fn diff_random_value_changes_detected_correctly() {
        let n = 120usize;
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();

        // base data
        for i in 0..n {
            let (k, v) = (format!("k{i}"), format!("v{i}"));
            a.insert(&k, &v);
            b.insert(&k, &v);
        }

        // randomly change 15 keys on b
        let mut rng = StdRng::seed_from_u64(2024);
        let mut changed_keys: Vec<String> = Vec::new();
        for _ in 0..15 {
            let idx = rng.gen_range(0..n);
            let k = format!("k{idx}");
            b.insert(&k, &format!("DIFF{idx}"));
            changed_keys.push(k);
        }
        changed_keys.sort();
        changed_keys.dedup(); // in case of collisions

        let diffs = a.diff_keys(&b);
        let s = set(&diffs);
        let expect = set(&changed_keys);
        assert_eq!(s, expect, "Must report exactly the keys whose values changed");
    }

    #[test]
    fn diff_random_removals_detected_correctly() {
        let n = 150usize;
        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();

        for i in 0..n {
            let (k, v) = (format!("k{i}"), format!("v{i}"));
            a.insert(&k, &v);
            b.insert(&k, &v);
        }

        let mut rng = StdRng::seed_from_u64(99);
        let mut removed: Vec<String> = Vec::new();
        for _ in 0..25 {
            let idx = rng.gen_range(0..n);
            let k = format!("k{idx}");
            if !removed.contains(&k) {
                b.remove(&k);
                removed.push(k);
            }
        }
        removed.sort();

        let diffs = a.diff_keys(&b);
        let s = set(&diffs);
        let expect = set(&removed);
        assert_eq!(s, expect, "Must report exactly the keys that were removed");
    }

    #[test]
    fn diff_structure_mismatch_large_random_subset() {
        // a has N keys; b has N + M (extra subset)
        let n = 300usize;
        let m = 40usize;

        let mut a = MerkleTree::new();
        let mut b = MerkleTree::new();

        for i in 0..n {
            let (k, v) = (format!("k{i}"), format!("v{i}"));
            a.insert(&k, &v);
            b.insert(&k, &v);
        }
        // Add extra keys to b
        for j in 0..m {
            b.insert(&format!("extra{j}"), &format!("val{j}"));
        }

        let diffs = a.diff_keys(&b);
        let s = set(&diffs);
        let expect: HashSet<String> = (0..m).map(|j| format!("extra{j}")).collect();

        assert_eq!(s, expect, "Must report all extra keys on the other side");
    }
    // 1) Empty tree: root is None
    #[test]
    fn t01_empty_tree_root_none() {
        let t = MerkleTree::new();
        assert!(t.get_root_hash().is_none());
        assert_eq!(t.node_count(), 0);
    }

    // 2) Single leaf: root == leaf hash
    #[test]
    fn t02_single_leaf_root_equals_leaf() {
        let mut t = MerkleTree::new();
        t.insert("a", "A");
        let expected = leaf_hash("a","A");
        let got = t.get_root_hash().unwrap().clone();
        assert_eq!(got, expected);
        assert_eq!(t.node_count(), 1);
    }

    // 3) Root hash length must be 32 bytes (SHA-256)
    #[test]
    fn t03_root_len_32() {
        let mut t = MerkleTree::new();
        t.insert("x","1");
        assert_eq!(t.get_root_hash().unwrap().len(), 32);
    }

    // 4) Lexicographical sorting of keys
    #[test]
    fn t04_inorder_keys_sorted() {
        let mut t = MerkleTree::new();
        t.insert("k2","v2");
        t.insert("k1","v1");
        t.insert("k10","v10");
        let ks = t.inorder_keys();
        assert_eq!(ks, vec!["k1".to_string(), "k10".to_string(), "k2".to_string()]);
    }

    // 5) Deterministic root independent of insertion order
    #[test]
    fn t05_deterministic_root_order_independent() {
        let items = vec![("a","1"),("b","2"),("c","3"),("d","4"),("e","5")];

        let mut t1 = MerkleTree::new();
        for (k,v) in &items { t1.insert(k,v); }
        let r1 = t1.get_root_hash().unwrap().clone();

        let mut t2 = MerkleTree::new();
        for (k,v) in items.iter().rev() { t2.insert(k,v); }
        let r2 = t2.get_root_hash().unwrap().clone();

        assert_eq!(r1, r2);
    }

    // 6) Internal node hash = H(left || right) – manual check with 2 leaves
    #[test]
    fn t06_manual_internal_hash_two_leaves() {
        let (k1,v1) = ("a","A");
        let (k2,v2) = ("b","B");
        assert!(k1 < k2);

        let h1 = leaf_hash(k1,v1);
        let h2 = leaf_hash(k2,v2);

        let mut t = MerkleTree::new();
        t.insert(k1,v1);
        t.insert(k2,v2);
        let root = t.get_root_hash().unwrap().clone();

        let mut hasher = Sha256::new();
        hasher.update(&h1);
        hasher.update(&h2);
        let expect = hasher.finalize().to_vec();
        assert_eq!(root, expect);
    }

    // 7) Manual root with 4 leaves = H( H(H1||H2) || H(H3||H4) )
    #[test]
    fn t07_manual_root_four_leaves() {
        let items = vec![("k1","v1"),("k2","v2"),("k3","v3"),("k4","v4")];

        let mut t = MerkleTree::new();
        for (k,v) in &items { t.insert(k,v); }
        let got = t.get_root_hash().unwrap().clone();

        let mut sorted = items.clone();
        sorted.sort_by(|a,b| a.0.cmp(&b.0));
        let hs: Vec<Vec<u8>> = sorted.iter().map(|(k,v)| leaf_hash(k,v)).collect();

        let mut h12 = Sha256::new(); h12.update(&hs[0]); h12.update(&hs[1]); let h12 = h12.finalize();
        let mut h34 = Sha256::new(); h34.update(&hs[2]); h34.update(&hs[3]); let h34 = h34.finalize();

        let mut hroot = Sha256::new(); hroot.update(h12); hroot.update(h34); let expect = hroot.finalize().to_vec();
        assert_eq!(got, expect);
    }

    // 8) Odd number of leaves promotes one node
    #[test]
    fn t08_odd_count_promotes_one() {
        let mut t = MerkleTree::new();
        t.insert("a","1"); t.insert("b","2"); t.insert("c","3");
        let root = t.root.as_ref().unwrap();
        let left_is_leaf = root.left.as_ref().map(|n| n.left.is_none() && n.right.is_none()).unwrap_or(false);
        let right_is_leaf = root.right.as_ref().map(|n| n.left.is_none() && n.right.is_none()).unwrap_or(false);
        assert!(left_is_leaf ^ right_is_leaf);
    }

    // 9) Idempotent insert (same key/value) keeps root
    #[test]
    fn t09_idempotent_insert() {
        let mut t = MerkleTree::new();
        t.insert("k","v");
        let r1 = t.get_root_hash().unwrap().clone();
        t.insert("k","v");
        let r2 = t.get_root_hash().unwrap().clone();
        assert_eq!(r1, r2);
    }

    // 10) Updating a value must change the root
    #[test]
    fn t10_update_changes_root() {
        let mut t = MerkleTree::new();
        t.insert("k1","v1");
        t.insert("k2","v2");
        let r_before = t.get_root_hash().unwrap().clone();
        t.insert("k2","v2_new");
        let r_after = t.get_root_hash().unwrap().clone();
        assert_ne!(r_before, r_after);
    }

    // 11) Removing non-existent key should not change root
    #[test]
    fn t11_remove_nonexistent_keeps_root() {
        let mut t = MerkleTree::new();
        t.insert("a","1"); t.insert("b","2");
        let r1 = t.get_root_hash().unwrap().clone();
        t.remove("zzz");
        let r2 = t.get_root_hash().unwrap().clone();
        assert_eq!(r1, r2);
    }

    // 12) leaves() returns lexicographically ordered pairs and correct hashes
    #[test]
    fn t12_leaves_view_sorted_and_hashed() {
        let mut t = MerkleTree::new();
        t.insert("b","2"); t.insert("a","1"); t.insert("c","3");
        let leaves = t.leaves();
        assert_eq!(leaves.iter().map(|(k,_)| k.clone()).collect::<Vec<_>>(), vec!["a","b","c"]);
        assert_eq!(leaves[0].1, leaf_hash("a","1"));
        assert_eq!(leaves[1].1, leaf_hash("b","2"));
        assert_eq!(leaves[2].1, leaf_hash("c","3"));
    }

    // 13) preorder_hashes() returns at least root hash and is non-empty for non-empty tree
    #[test]
    fn t13_preorder_non_empty() {
        let mut t = MerkleTree::new();
        t.insert("a","1"); t.insert("b","2");
        let pre = t.preorder_hashes();
        assert!(!pre.is_empty());
        assert_eq!(pre[0], *t.get_root_hash().unwrap());
    }

    // 14) node_count vs number of leaves (for power-of-two leaves it's 2n-1)
    #[test]
    fn t14_node_count_two_pow() {
        let mut t = MerkleTree::new();
        for i in 0..4 { t.insert(&format!("k{i}"), &format!("v{i}")); }
        // For 4 leaves in perfect binary tree: 2*4 - 1 = 7
        assert_eq!(t.node_count(), 7);
    }

    // 15) Diff: no difference
    #[test]
    fn t15_diff_no_change_empty_vec() {
        let mut a = MerkleTree::new(); let mut b = MerkleTree::new();
        for (k,v) in &[("k1","v1"),("k2","v2"),("k3","v3")] { a.insert(k,v); b.insert(k,v); }
        assert!(a.diff_keys(&b).is_empty());
        assert!(b.diff_keys(&a).is_empty());
        assert_eq!(a.diff_first_key(&b), None);
    }

    // 16) Diff: single value change
    #[test]
    fn t16_diff_single_value_change() {
        let mut a = MerkleTree::new(); let mut b = MerkleTree::new();
        a.insert("k1","v1"); a.insert("k2","v2");
        b.insert("k1","v1"); b.insert("k2","DIFF");
        let diffs = a.diff_keys(&b);
        assert_eq!(diffs, vec!["k2".to_string()]);
        assert_eq!(a.diff_first_key(&b), Some("k2".into()));
    }

    // 17) Diff: missing key
    #[test]
    fn t17_diff_missing_key() {
        let mut a = MerkleTree::new(); let mut b = MerkleTree::new();
        a.insert("k1","v1"); a.insert("k2","v2"); a.insert("k3","v3");
        b.insert("k1","v1"); b.insert("k2","v2");
        assert_eq!(a.diff_keys(&b), vec!["k3".to_string()]);
    }

    // 18) Diff: extra key
    #[test]
    fn t18_diff_extra_key() {
        let mut a = MerkleTree::new(); let mut b = MerkleTree::new();
        a.insert("k1","v1"); a.insert("k2","v2");
        b.insert("k1","v1"); b.insert("k2","v2"); b.insert("kX","vX");
        assert_eq!(a.diff_keys(&b), vec!["kX".to_string()]);
    }

    // 19) Unicode & NUL bytes robustness (length-prefix)
    #[test]
    fn t19_unicode_and_nul() {
        let mut t = MerkleTree::new();
        t.insert("中文","值"); t.insert("nu\0l","v"); t.insert("k","va\0lue");
        let r1 = t.get_root_hash().unwrap().clone();
        // re-insert same → root stable
        t.insert("中文","值"); t.insert("nu\0l","v"); t.insert("k","va\0lue");
        let r2 = t.get_root_hash().unwrap().clone();
        assert_eq!(r1, r2);
    }

    // 20) Remove then re-insert restores original root
    #[test]
    fn t20_remove_then_reinsert_restores() {
        let mut t = MerkleTree::new();
        t.insert("k1","v1"); t.insert("k2","v2"); t.insert("k3","v3");
        let r0 = t.get_root_hash().unwrap().clone();
        t.remove("k2");
        assert_ne!(t.get_root_hash().unwrap(), &r0);
        t.insert("k2","v2");
        assert_eq!(t.get_root_hash().unwrap(), &r0);
    }

    // 21) Many items stability (rebuild with same data keeps root)
    #[test]
    fn t21_many_items_stability() {
        let mut t = MerkleTree::new();
        for i in 0..50 { t.insert(&format!("k{i}"), &format!("v{i}")); }
        let r1 = t.get_root_hash().unwrap().clone();
        for i in 0..50 { t.insert(&format!("k{i}"), &format!("v{i}")); }
        let r2 = t.get_root_hash().unwrap().clone();
        assert_eq!(r1, r2);
    }

    // 22) Traversal: preorder length equals node_count (non-empty)
    #[test]
    fn t22_preorder_len_equals_node_count() {
        let mut t = MerkleTree::new();
        for i in 0..5 { t.insert(&format!("k{i}"), &format!("v{i}")); }
        let pre = t.preorder_hashes();
        assert_eq!(pre.len(), t.node_count());
    }
    
}
