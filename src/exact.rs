use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};

use ahash::AHasher;

type AHashSet<T> = HashSet<T, BuildHasherDefault<AHasher>>;

/// Hash-based exact duplicate detector.
///
/// Hashes each text with ahash and stores the u64 digest in a HashSet.
/// Two texts are "exact duplicates" iff they produce the same hash.
pub struct ExactDedup {
    seen: AHashSet<u64>,
}

impl ExactDedup {
    pub fn new() -> Self {
        Self {
            seen: AHashSet::default(),
        }
    }

    /// Returns `true` if `text` was already seen (duplicate).
    /// Inserts the hash on first encounter.
    pub fn is_duplicate(&mut self, text: &str) -> bool {
        let hash = Self::hash_text(text);
        // insert returns false if the value was already present
        !self.seen.insert(hash)
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
