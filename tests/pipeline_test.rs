use textsift::pipeline::{deduplicate, DedupConfig, DedupResult};

fn config(threshold: f64, exact_only: bool) -> DedupConfig {
    DedupConfig {
        threshold,
        num_perm: 128,
        shingle_size: 5,
        exact_only,
    }
}

fn texts(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// A ~50-word base sentence so near-duplicate variants stay well above threshold.
fn base_doc() -> String {
    (0..50)
        .map(|i| format!("word{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Same as base_doc but with one word swapped — a near-duplicate, not exact.
fn near_dup_doc() -> String {
    let mut words: Vec<String> = (0..50).map(|i| format!("word{i}")).collect();
    words[49] = "changed".to_string();
    words.join(" ")
}

fn assert_invariants(result: &DedupResult, n: usize) {
    assert_eq!(result.cluster_ids.len(), n);
    assert_eq!(result.is_representative.len(), n);
    assert_eq!(result.total, n);
    let reps = result.is_representative.iter().filter(|&&r| r).count();
    assert_eq!(
        reps, result.unique_clusters,
        "one representative per cluster"
    );
}

#[test]
fn empty_input() {
    let result = deduplicate(&[], &config(0.8, false));
    assert_invariants(&result, 0);
    assert_eq!(result.exact_dupes, 0);
    assert_eq!(result.near_dupes, 0);
    assert_eq!(result.unique_clusters, 0);
}

#[test]
fn single_document() {
    let result = deduplicate(&texts(&["hello world"]), &config(0.8, false));
    assert_invariants(&result, 1);
    assert!(result.is_representative[0]);
    assert_eq!(result.unique_clusters, 1);
}

#[test]
fn all_unique_documents() {
    let docs = texts(&[
        "the quick brown fox jumps over the lazy dog",
        "completely different sentence about cooking pasta tonight",
        "rust is a systems programming language focused on safety",
    ]);
    let result = deduplicate(&docs, &config(0.8, false));
    assert_invariants(&result, 3);
    assert_eq!(result.exact_dupes, 0);
    assert_eq!(result.near_dupes, 0);
    assert_eq!(result.unique_clusters, 3);
    assert!(result.is_representative.iter().all(|&r| r));
}

#[test]
fn exact_duplicates_share_cluster_first_is_representative() {
    let docs = texts(&["same text here", "other doc entirely", "same text here"]);
    let result = deduplicate(&docs, &config(0.8, false));
    assert_invariants(&result, 3);
    assert_eq!(result.exact_dupes, 1);
    assert_eq!(result.cluster_ids[0], result.cluster_ids[2]);
    assert!(result.is_representative[0], "first occurrence is kept");
    assert!(!result.is_representative[2], "later copy is dropped");
    assert!(result.is_representative[1]);
    assert_eq!(result.unique_clusters, 2);
}

#[test]
fn near_duplicates_cluster_lowest_index_is_representative() {
    let docs = vec![base_doc(), near_dup_doc()];
    let result = deduplicate(&docs, &config(0.5, false));
    assert_invariants(&result, 2);
    assert_eq!(result.near_dupes, 1);
    assert_eq!(result.cluster_ids[0], result.cluster_ids[1]);
    assert!(result.is_representative[0]);
    assert!(!result.is_representative[1]);
    assert_eq!(result.unique_clusters, 1);
}

#[test]
fn exact_only_skips_near_duplicates() {
    let docs = vec![base_doc(), near_dup_doc(), base_doc()];
    let result = deduplicate(&docs, &config(0.5, true));
    assert_invariants(&result, 3);
    assert_eq!(result.exact_dupes, 1, "exact copy of doc 0 detected");
    assert_eq!(result.near_dupes, 0, "near-dup detection disabled");
    assert_ne!(result.cluster_ids[0], result.cluster_ids[1]);
    assert_eq!(result.cluster_ids[0], result.cluster_ids[2]);
    assert_eq!(result.unique_clusters, 2);
}

#[test]
fn exact_and_near_duplicates_merge_into_one_cluster() {
    // doc 0: base; doc 1: exact copy of base; doc 2: near-dup of base
    let docs = vec![base_doc(), base_doc(), near_dup_doc()];
    let result = deduplicate(&docs, &config(0.5, false));
    assert_invariants(&result, 3);
    assert_eq!(result.exact_dupes, 1);
    assert_eq!(result.near_dupes, 1);
    assert_eq!(result.cluster_ids[0], result.cluster_ids[1]);
    assert_eq!(result.cluster_ids[0], result.cluster_ids[2]);
    assert!(result.is_representative[0]);
    assert!(!result.is_representative[1]);
    assert!(!result.is_representative[2]);
    assert_eq!(result.unique_clusters, 1);
}

#[test]
fn deterministic_across_runs() {
    let docs = vec![base_doc(), near_dup_doc(), base_doc(), "unrelated short doc".to_string()];
    let cfg = config(0.5, false);
    let a = deduplicate(&docs, &cfg);
    let b = deduplicate(&docs, &cfg);
    assert_eq!(a.cluster_ids, b.cluster_ids);
    assert_eq!(a.is_representative, b.is_representative);
}

#[test]
fn unicode_text() {
    let docs = texts(&[
        "el rápido zorro marrón salta sobre el perro perezoso",
        "el rápido zorro marrón salta sobre el perro perezoso",
        "你好 世界 这是 一个 测试 文档",
    ]);
    let result = deduplicate(&docs, &config(0.8, false));
    assert_invariants(&result, 3);
    assert_eq!(result.exact_dupes, 1);
    assert_eq!(result.cluster_ids[0], result.cluster_ids[1]);
    assert!(result.is_representative[2]);
}

/// Characterization: empty and whitespace-only texts produce empty shingle
/// sets, which yield identical all-MAX MinHash signatures — so they cluster
/// together as near-duplicates even though the raw strings differ.
#[test]
fn empty_and_whitespace_docs_cluster_together() {
    let docs = texts(&["", "   ", "real content document here"]);
    let result = deduplicate(&docs, &config(0.8, false));
    assert_invariants(&result, 3);
    assert_eq!(
        result.cluster_ids[0], result.cluster_ids[1],
        "empty-ish docs share identical empty-shingle signatures"
    );
    assert_ne!(result.cluster_ids[0], result.cluster_ids[2]);
}

#[test]
fn docs_shorter_than_shingle_size() {
    // Whole text becomes a single shingle; identical short docs are exact dups,
    // different short docs stay separate.
    let docs = texts(&["hi", "hi", "bye"]);
    let result = deduplicate(&docs, &config(0.8, false));
    assert_invariants(&result, 3);
    assert_eq!(result.exact_dupes, 1);
    assert_ne!(result.cluster_ids[0], result.cluster_ids[2]);
}
