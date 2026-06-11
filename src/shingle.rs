use ahash::RandomState;
use std::hash::{BuildHasher, Hasher};

/// Hash each word n-gram shingle of `text` to a single u64.
///
/// Splits `text` on whitespace and hashes sliding windows of `size` words
/// directly — no per-shingle `String` allocation. Words are separated by a
/// 0xFF byte inside the hasher (0xFF never occurs in valid UTF-8, so
/// `"ab c"` and `"a bc"` hash differently). If the text has fewer than
/// `size` words, the whole text is one shingle. Empty text yields an empty
/// vec. Seeds are fixed, so results are deterministic across runs.
///
/// # Panics
///
/// Panics if `size == 0` (validated upstream by [`crate::pipeline::DedupConfig::validate`]).
pub fn shingle_hashes(text: &str, size: usize) -> Vec<u64> {
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return Vec::new();
    }

    let state = RandomState::with_seeds(0x73686901, 0x6e676c65, 0x68617368, 0x74786673);
    let hash_window = |window: &[&str]| {
        let mut h = state.build_hasher();
        for w in window {
            h.write(w.as_bytes());
            h.write_u8(0xFF);
        }
        h.finish()
    };

    if words.len() < size {
        return vec![hash_window(&words)];
    }

    words.windows(size).map(hash_window).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_count() {
        assert_eq!(shingle_hashes("a b c d e f", 5).len(), 2);
        assert_eq!(shingle_hashes("one two three", 3).len(), 1);
    }

    #[test]
    fn text_shorter_than_size_is_one_shingle() {
        assert_eq!(shingle_hashes("hello", 5).len(), 1);
    }

    #[test]
    fn empty_text() {
        assert!(shingle_hashes("", 5).is_empty());
    }

    #[test]
    fn deterministic_and_content_sensitive() {
        let a = shingle_hashes("a b c d e f", 5);
        assert_eq!(a, shingle_hashes("a b c d e f", 5));
        assert_ne!(a, shingle_hashes("a b c d e g", 5));
    }

    #[test]
    fn word_boundaries_matter() {
        // Same bytes, different word split — separator must distinguish them.
        assert_ne!(shingle_hashes("ab c", 2), shingle_hashes("a bc", 2));
    }

    #[test]
    fn whitespace_normalized() {
        // split_whitespace collapses runs, so spacing doesn't change shingles.
        assert_eq!(shingle_hashes("a  b   c", 2), shingle_hashes("a b c", 2));
    }
}
