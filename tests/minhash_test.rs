use textsift::minhash::{MinHasher, Signature};
use textsift::shingle::shingles;

#[test]
fn shingles_basic() {
    assert_eq!(
        shingles("a b c d e f", 5),
        vec!["a b c d e", "b c d e f"]
    );
}

#[test]
fn shingles_shorter_than_size() {
    assert_eq!(shingles("hello", 5), vec!["hello"]);
}

#[test]
fn shingles_empty() {
    assert!(shingles("", 5).is_empty());
}

#[test]
fn shingles_exact_size() {
    assert_eq!(
        shingles("one two three four five", 5),
        vec!["one two three four five"]
    );
}

#[test]
fn identical_texts_jaccard_one() {
    let mh = MinHasher::new(128);
    let text = "the quick brown fox jumps over the lazy dog";
    let sh = shingles(text, 3);
    let sig_a = mh.signature(&sh);
    let sig_b = mh.signature(&sh);
    assert_eq!(MinHasher::jaccard(&sig_a, &sig_b), 1.0);
}

#[test]
fn completely_different_texts_low_jaccard() {
    let mh = MinHasher::new(128);
    let sh_a = shingles("alpha beta gamma delta epsilon zeta eta theta", 3);
    let sh_b = shingles("one two three four five six seven eight nine ten", 3);
    let sig_a = mh.signature(&sh_a);
    let sig_b = mh.signature(&sh_b);
    let j = MinHasher::jaccard(&sig_a, &sig_b);
    assert!(j < 0.1, "expected jaccard < 0.1 for disjoint texts, got {j}");
}

#[test]
fn overlapping_texts_approximate_jaccard() {
    // Two texts sharing ~80% of their 3-word shingles.
    // "a b c d e f g h i j" → 8 shingles of size 3
    // "a b c d e f g h x y" → 8 shingles of size 3
    // Shared: "a b c", "b c d", "c d e", "d e f", "e f g", "f g h" = 6
    // Union: 6 + 2 + 2 = 10
    // True Jaccard = 6/10 = 0.6
    let mh = MinHasher::new(256);
    let sh_a = shingles("a b c d e f g h i j", 3);
    let sh_b = shingles("a b c d e f g h x y", 3);
    let sig_a = mh.signature(&sh_a);
    let sig_b = mh.signature(&sh_b);
    let j = MinHasher::jaccard(&sig_a, &sig_b);
    let true_jaccard = 0.6;
    assert!(
        (j - true_jaccard).abs() < 0.15,
        "expected jaccard ≈ {true_jaccard} (±0.15), got {j}"
    );
}

#[test]
fn signature_deterministic() {
    let mh = MinHasher::new(128);
    let sh = shingles("deterministic test input words here now", 3);
    let sig1 = mh.signature(&sh);
    let sig2 = mh.signature(&sh);
    assert_eq!(sig1, sig2);
}
