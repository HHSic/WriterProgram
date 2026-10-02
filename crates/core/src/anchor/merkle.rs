//! The Merkle root sent for a time stamp: one 32-byte fingerprint standing
//! for every leaf (each device's last journal line, each chapter's body).
//!
//! Hashing follows RFC 6962 (Certificate Transparency) so the tree is a
//! well-known one: a leaf is `SHA-256(0x00 ‖ data)`, a node is
//! `SHA-256(0x01 ‖ left ‖ right)`, and a list of `n` leaves splits at the
//! largest power of two below `n`. The prefixes keep a leaf from ever being
//! read as a node. A leaf's data is `<label>\n<value>`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One thing the root stands for.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Leaf {
    /// `journal:<device id>` or `doc:<document id>`.
    pub label: String,
    /// SHA-256 (hex) of that journal's last line, or of the chapter's body.
    pub value: String,
}

impl Leaf {
    pub fn journal(device: &str, line_hash: &str) -> Self {
        Leaf {
            label: format!("journal:{device}"),
            value: line_hash.into(),
        }
    }

    pub fn doc(id: &str, body: &str) -> Self {
        Leaf {
            label: format!("doc:{id}"),
            value: body.into(),
        }
    }

    /// The device, for a journal leaf.
    pub fn device(&self) -> Option<&str> {
        self.label.strip_prefix("journal:")
    }

    /// The document, for a chapter leaf.
    pub fn doc_id(&self) -> Option<&str> {
        self.label.strip_prefix("doc:")
    }

    fn hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update([0u8]);
        h.update(self.label.as_bytes());
        h.update(b"\n");
        h.update(self.value.as_bytes());
        h.finalize().into()
    }
}

fn node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([1u8]);
    h.update(left);
    h.update(right);
    h.finalize().into()
}

/// The root over `leaves` in the order given (records keep them sorted by
/// label). No leaves gives the SHA-256 of nothing.
pub fn root(leaves: &[Leaf]) -> [u8; 32] {
    let hashes: Vec<[u8; 32]> = leaves.iter().map(Leaf::hash).collect();
    if hashes.is_empty() {
        return Sha256::digest([]).into();
    }
    subtree(&hashes)
}

fn subtree(hashes: &[[u8; 32]]) -> [u8; 32] {
    if hashes.len() == 1 {
        return hashes[0];
    }
    let mut split = 1;
    while split * 2 < hashes.len() {
        split *= 2;
    }
    node(&subtree(&hashes[..split]), &subtree(&hashes[split..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::hex;

    fn leaf(i: usize) -> Leaf {
        Leaf::doc(&format!("d{i}"), &format!("{i:064x}"))
    }

    #[test]
    fn roots_follow_rfc_6962() {
        let a = leaf(1);
        let b = leaf(2);
        let c = leaf(3);
        assert_eq!(root(std::slice::from_ref(&a)), a.hash());
        assert_eq!(root(&[a.clone(), b.clone()]), node(&a.hash(), &b.hash()));
        // Three leaves: the first two pair up, the third joins at the top.
        assert_eq!(
            root(&[a.clone(), b.clone(), c.clone()]),
            node(&node(&a.hash(), &b.hash()), &c.hash())
        );
        assert_eq!(
            hex(&root(&[])),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn any_change_moves_the_root() {
        let leaves: Vec<Leaf> = (0..7).map(leaf).collect();
        let base = root(&leaves);
        for i in 0..leaves.len() {
            let mut changed = leaves.clone();
            changed[i].value = format!("{:064x}", 99);
            assert_ne!(root(&changed), base, "leaf {i}");
        }
        let mut swapped = leaves.clone();
        swapped.swap(0, 1);
        assert_ne!(root(&swapped), base);
        assert_ne!(root(&leaves[..6]), base);
        // A leaf's label counts as much as its value.
        let mut relabelled = leaves.clone();
        relabelled[3].label = "doc:other".into();
        assert_ne!(root(&relabelled), base);
    }

    #[test]
    fn a_known_root() {
        // Fixed so a change to the hashing shows up (the verifier must keep
        // recomputing roots of old records the same way).
        let leaves = vec![
            Leaf::journal("dev1aaaaaaaa", &"a".repeat(64)),
            Leaf::doc("k7q2m9x4t1ab", &"b".repeat(64)),
        ];
        let mut first = Sha256::new();
        first.update([0u8]);
        first.update(format!("journal:dev1aaaaaaaa\n{}", "a".repeat(64)));
        let mut second = Sha256::new();
        second.update([0u8]);
        second.update(format!("doc:k7q2m9x4t1ab\n{}", "b".repeat(64)));
        let expected = node(&first.finalize().into(), &second.finalize().into());
        assert_eq!(root(&leaves), expected);
    }
}
