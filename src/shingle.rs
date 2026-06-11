/// Generate word n-gram shingles from text.
///
/// Splits `text` on whitespace and produces sliding windows of `size` words,
/// each joined by a single space. If the text has fewer than `size` words,
/// returns the full (trimmed) text as a single shingle. Empty text yields an
/// empty vec.
pub fn shingles(text: &str, size: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return Vec::new();
    }

    if words.len() < size {
        return vec![words.join(" ")];
    }

    words.windows(size).map(|w| w.join(" ")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_shingles() {
        assert_eq!(shingles("a b c d e f", 5), vec!["a b c d e", "b c d e f"]);
    }

    #[test]
    fn text_shorter_than_size() {
        assert_eq!(shingles("hello", 5), vec!["hello"]);
    }

    #[test]
    fn empty_text() {
        assert!(shingles("", 5).is_empty());
    }

    #[test]
    fn exact_size() {
        assert_eq!(shingles("one two three", 3), vec!["one two three"]);
    }
}
