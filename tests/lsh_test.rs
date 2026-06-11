use textsift::cluster::UnionFind;
use textsift::lsh::{optimal_params, LshIndex};
use textsift::minhash::MinHasher;
use textsift::shingle::shingle_hashes as shingles;

// ---------- optimal_params ----------

#[test]
fn optimal_params_product_equals_num_perm() {
    let (b, r) = optimal_params(0.8, 128);
    assert_eq!(b * r, 128, "bands * rows must equal num_perm");
}

#[test]
fn optimal_params_inflection_near_threshold() {
    let (b, r) = optimal_params(0.8, 128);
    let inflection = (1.0 / b as f64).powf(1.0 / r as f64);
    assert!(
        (inflection - 0.8).abs() < 0.1,
        "inflection {inflection} should be close to 0.8"
    );
}

// ---------- LSH candidate pairs ----------

#[test]
fn similar_docs_are_candidates() {
    let mh = MinHasher::new(128);
    // Two nearly identical texts — high Jaccard similarity
    let text_a = "the quick brown fox jumps over the lazy dog near the river";
    let text_b = "the quick brown fox jumps over the lazy dog near the lake";
    let sig_a = mh.signature(&shingles(text_a, 3));
    let sig_b = mh.signature(&shingles(text_b, 3));

    let jaccard = MinHasher::jaccard(&sig_a, &sig_b);
    assert!(jaccard > 0.5, "expected high Jaccard, got {jaccard}");

    let (b, r) = optimal_params(0.5, 128);
    let mut idx = LshIndex::new(b, r);
    idx.insert(0, &sig_a);
    idx.insert(1, &sig_b);

    let pairs = idx.candidate_pairs();
    assert!(
        pairs.contains(&(0, 1)),
        "similar docs should be a candidate pair"
    );
}

#[test]
fn dissimilar_docs_not_candidates() {
    let mh = MinHasher::new(128);
    let text_a = "the quick brown fox jumps over the lazy dog";
    let text_b = "quantum mechanics describes fundamental particles and wave functions in physics";
    let sig_a = mh.signature(&shingles(text_a, 3));
    let sig_b = mh.signature(&shingles(text_b, 3));

    let jaccard = MinHasher::jaccard(&sig_a, &sig_b);
    assert!(jaccard < 0.2, "expected low Jaccard, got {jaccard}");

    // Use a high threshold so the LSH is strict
    let (b, r) = optimal_params(0.5, 128);
    let mut idx = LshIndex::new(b, r);
    idx.insert(0, &sig_a);
    idx.insert(1, &sig_b);

    let pairs = idx.candidate_pairs();
    assert!(
        !pairs.contains(&(0, 1)),
        "dissimilar docs should NOT be a candidate pair"
    );
}

// ---------- UnionFind ----------

#[test]
fn union_find_basic_clusters() {
    let mut uf = UnionFind::new(5);
    uf.union(0, 1);
    uf.union(1, 2);
    uf.union(3, 4);
    let clusters = uf.clusters();
    assert_eq!(clusters.len(), 2);

    // Cluster containing 0 has {0, 1, 2}
    let root0 = uf.find(0);
    assert_eq!(clusters[&root0], vec![0, 1, 2]);
    // Cluster containing 3 has {3, 4}
    let root3 = uf.find(3);
    assert_eq!(clusters[&root3], vec![3, 4]);
}

#[test]
fn union_find_single_element() {
    let mut uf = UnionFind::new(1);
    let clusters = uf.clusters();
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[&0], vec![0]);
}

#[test]
fn union_find_all_unioned() {
    let mut uf = UnionFind::new(4);
    uf.union(0, 1);
    uf.union(2, 3);
    uf.union(1, 3);
    let clusters = uf.clusters();
    assert_eq!(clusters.len(), 1);
    let members = clusters.values().next().unwrap();
    assert_eq!(members, &vec![0, 1, 2, 3]);
}

// ---------- Integration: MinHash → LSH → UnionFind ----------

#[test]
fn end_to_end_minhash_lsh_cluster() {
    let num_perm = 128;
    let mh = MinHasher::new(num_perm);

    // Three docs: doc0 and doc1 are near-duplicates, doc2 is unrelated
    let docs = [
        "the quick brown fox jumps over the lazy dog by the river bank",
        "the quick brown fox jumps over the lazy dog by the river side",
        "quantum computing exploits superposition and entanglement for parallel computation",
    ];

    let sigs: Vec<_> = docs.iter().map(|d| mh.signature(&shingles(d, 3))).collect();

    // Verify similarity assumptions
    let j01 = MinHasher::jaccard(&sigs[0], &sigs[1]);
    let j02 = MinHasher::jaccard(&sigs[0], &sigs[2]);
    assert!(j01 > 0.5, "doc0-doc1 should be similar, got {j01}");
    assert!(j02 < 0.2, "doc0-doc2 should be dissimilar, got {j02}");

    // LSH
    let (b, r) = optimal_params(0.5, num_perm);
    let mut idx = LshIndex::new(b, r);
    for (i, sig) in sigs.iter().enumerate() {
        idx.insert(i, sig);
    }
    let pairs = idx.candidate_pairs();
    assert!(pairs.contains(&(0, 1)), "similar docs must be candidates");

    // Cluster with UnionFind
    let mut uf = UnionFind::new(docs.len());
    for (a, b) in &pairs {
        uf.union(*a, *b);
    }
    let clusters = uf.clusters();

    // doc0 and doc1 must be in the same cluster
    assert_eq!(uf.find(0), uf.find(1), "doc0 and doc1 should share a root");
    // doc2 should be alone (unless false-positive paired it with 0 or 1)
    if !pairs.contains(&(0, 2)) && !pairs.contains(&(1, 2)) {
        assert_ne!(uf.find(0), uf.find(2), "doc2 should be in its own cluster");
    }

    // Every element appears in exactly one cluster
    let total_members: usize = clusters.values().map(|v| v.len()).sum();
    assert_eq!(total_members, docs.len());
}
