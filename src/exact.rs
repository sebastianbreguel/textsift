use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use ahash::AHasher;

type AHashMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

pub struct ExactDedup {
    groups: AHashMap<u64, Vec<usize>>,
}

impl ExactDedup {
    pub fn new() -> Self {
        Self {
            groups: AHashMap::default(),
        }
    }

    /// Process doc at index `i`. Returns `true` if it's a duplicate of a prior doc.
    pub fn process(&mut self, i: usize, text: &str) -> bool {
        let hash = Self::hash_text(text);
        let entry = self.groups.entry(hash).or_default();
        entry.push(i);
        entry.len() > 1
    }

    /// Returns `true` if `text` was already seen (duplicate).
    pub fn is_duplicate(&mut self, text: &str) -> bool {
        let hash = Self::hash_text(text);
        let entry = self.groups.entry(hash).or_default();
        let is_dup = !entry.is_empty();
        if !is_dup {
            entry.push(0);
        }
        is_dup
    }

    /// Consume and return the exact-match groups (hash → list of doc indices).
    pub fn into_groups(self) -> AHashMap<u64, Vec<usize>> {
        self.groups
    }

    fn hash_text(text: &str) -> u64 {
        let mut hasher = AHasher::default();
        hasher.write(text.as_bytes());
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_occurrence_is_not_duplicate() {
        let mut dedup = ExactDedup::new();
        assert!(!dedup.is_duplicate("hello world"));
    }

    #[test]
    fn second_occurrence_is_duplicate() {
        let mut dedup = ExactDedup::new();
        assert!(!dedup.is_duplicate("hello"));
        assert!(dedup.is_duplicate("hello"));
    }

    #[test]
    fn different_texts_not_duplicate() {
        let mut dedup = ExactDedup::new();
        assert!(!dedup.is_duplicate("aaa"));
        assert!(!dedup.is_duplicate("bbb"));
    }
}
