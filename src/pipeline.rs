use anyhow::Result;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

use crate::cli::Cli;
use crate::cluster::UnionFind;
use crate::exact::ExactDedup;
use crate::lsh::{self, LshIndex};
use crate::minhash::MinHasher;
use crate::shingle;

/// Run the full deduplication pipeline.
///
/// 1. Read all JSONL docs, extract text field
/// 2. Exact dedup
/// 3. MinHash LSH (unless --exact-only)
/// 4. Assign cluster IDs
/// 5. Write output (clean or clusters mode)
/// 6. Print stats if --stats
pub fn run(args: &Cli) -> Result<()> {
    let reader = crate::io::open_reader(&args.input)?;
    let mut writer = crate::io::open_writer(args.output.as_deref())?;

    // --- Step 1: Read all docs into memory ---
    let mut docs: Vec<Value> = Vec::new();
    let mut texts: Vec<String> = Vec::new();

    for (line_num, result) in crate::io::read_jsonl(reader) {
        match result {
            Ok(value) => match crate::io::extract_field(&value, &args.field) {
                Some(text) => {
                    texts.push(text.to_owned());
                    docs.push(value);
                }
                None => {
                    eprintln!("warning: line {line_num} missing field '{}'", args.field);
                }
            },
            Err(e) => {
                eprintln!("warning: {e}");
            }
        }
    }

    let total = docs.len();
    if total == 0 {
        if args.stats {
            print_stats(0, 0, 0, 0, 0);
        }
        return Ok(());
    }

    // --- Step 2: Exact dedup ---
    // Group docs by their exact text hash. The first occurrence is the representative.
    let mut dedup = ExactDedup::new();
    // is_exact_dup[i] = true means doc i is an exact duplicate of an earlier doc
    let mut is_exact_dup = vec![false; total];
    let mut exact_dup_count: usize = 0;

    for (i, text) in texts.iter().enumerate() {
        if dedup.is_duplicate(text) {
            is_exact_dup[i] = true;
            exact_dup_count += 1;
        }
    }

    // Build exact duplicate groups: map text hash -> list of doc indices
    // We need to group exact duplicates together for clustering
    let exact_groups = build_exact_groups(&texts);

    // --- Step 3: MinHash LSH (skip if --exact-only) ---
    let mut near_dup_count: usize = 0;

    // We'll build a unified clustering over non-exact-dup docs
    // Non-exact-dup doc indices (indices into docs/texts arrays)
    let unique_indices: Vec<usize> = (0..total).filter(|&i| !is_exact_dup[i]).collect();
    let n_unique = unique_indices.len();

    // Map from original doc index -> position in unique_indices
    let mut orig_to_unique: HashMap<usize, usize> = HashMap::new();
    for (pos, &orig) in unique_indices.iter().enumerate() {
        orig_to_unique.insert(orig, pos);
    }

    // UnionFind over unique docs only
    let mut uf = UnionFind::new(n_unique);

    if !args.exact_only && n_unique > 1 {
        let hasher = MinHasher::new(args.num_perm);

        // Compute signatures in parallel (embarrassingly parallel — no shared state)
        let signatures: Vec<_> = unique_indices
            .par_iter()
            .map(|&i| {
                let shingles = shingle::shingles(&texts[i], args.shingle_size);
                hasher.signature(&shingles)
            })
            .collect();

        // LSH indexing
        let (bands, rows) = lsh::optimal_params(args.threshold, args.num_perm);
        let mut lsh_index = LshIndex::new(bands, rows);

        for (pos, sig) in signatures.iter().enumerate() {
            lsh_index.insert(pos, sig);
        }

        let candidate_pairs = lsh_index.candidate_pairs();

        // Verify candidates against threshold and union
        for (a, b) in &candidate_pairs {
            let sim = MinHasher::jaccard(&signatures[*a], &signatures[*b]);
            if sim >= args.threshold {
                uf.union(*a, *b);
            }
        }
    }

    // Get clusters from UnionFind (positions in unique_indices)
    let uf_clusters = uf.clusters();

    // --- Step 4: Assign cluster IDs ---
    // Each doc gets a cluster_id. Build: doc_index -> (cluster_id, is_representative)
    let mut doc_cluster: Vec<(usize, bool)> = vec![(0, false); total];
    let mut next_cluster_id: usize = 0;

    // First, handle the UF clusters (these are the unique/non-exact-dup docs)
    // Sort cluster roots for deterministic output
    let mut uf_roots: Vec<usize> = uf_clusters.keys().copied().collect();
    uf_roots.sort_unstable();

    // Map from UF position -> cluster_id assigned
    let mut uf_pos_to_cluster: HashMap<usize, usize> = HashMap::new();

    for root in &uf_roots {
        let members = &uf_clusters[root];
        let cluster_id = next_cluster_id;
        next_cluster_id += 1;

        for &pos in members {
            let orig_idx = unique_indices[pos];
            let is_rep = pos == members[0]; // first member (smallest orig index) is representative
            doc_cluster[orig_idx] = (cluster_id, is_rep);
            uf_pos_to_cluster.insert(pos, cluster_id);
        }
    }

    // Count near duplicates: docs in multi-member UF clusters minus the representative
    for members in uf_clusters.values() {
        if members.len() > 1 {
            near_dup_count += members.len() - 1;
        }
    }

    // Now assign exact duplicates to the same cluster as their representative
    for group in exact_groups.values() {
        if group.len() < 2 {
            continue;
        }
        // The first element in the group is the representative (already has a cluster from UF)
        let rep_idx = group[0];
        let rep_cluster = doc_cluster[rep_idx].0;

        for &dup_idx in &group[1..] {
            doc_cluster[dup_idx] = (rep_cluster, false);
        }
    }

    // --- Step 5: Write output ---
    let unique_clusters = next_cluster_id;

    if args.clusters {
        // Clusters mode: write ALL docs with cluster_id and is_representative
        for (i, mut doc) in docs.into_iter().enumerate() {
            let (cluster_id, is_rep) = doc_cluster[i];
            if let Some(obj) = doc.as_object_mut() {
                obj.insert("cluster_id".to_string(), Value::from(cluster_id as u64));
                obj.insert("is_representative".to_string(), Value::Bool(is_rep));
            }
            crate::io::write_jsonl(&mut writer, &doc)?;
        }
    } else {
        // Default mode: write only representative docs (clean, no extra fields)
        for (i, doc) in docs.iter().enumerate() {
            let (_cluster_id, is_rep) = doc_cluster[i];
            if is_rep {
                crate::io::write_jsonl(&mut writer, doc)?;
            }
        }
    }

    // --- Step 6: Stats ---
    if args.stats {
        let unique_emitted = unique_clusters;
        print_stats(total, exact_dup_count, near_dup_count, unique_clusters, unique_emitted);
    }

    Ok(())
}

/// Build groups of doc indices that share the exact same text.
fn build_exact_groups(texts: &[String]) -> HashMap<u64, Vec<usize>> {
    use std::hash::{Hasher, Hash};
    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();

    for (i, text) in texts.iter().enumerate() {
        let mut hasher = ahash::AHasher::default();
        text.hash(&mut hasher);
        let h = hasher.finish();
        groups.entry(h).or_default().push(i);
    }

    groups
}

fn print_stats(total: usize, exact_dupes: usize, near_dupes: usize, clusters: usize, unique: usize) {
    eprintln!("total docs: {total}");
    eprintln!("exact duplicates: {exact_dupes}");
    eprintln!("near duplicates: {near_dupes}");
    eprintln!("unique clusters: {clusters}");
    eprintln!("unique docs emitted: {unique}");
}
