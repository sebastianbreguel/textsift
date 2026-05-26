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

pub struct DedupConfig {
    pub threshold: f64,
    pub num_perm: usize,
    pub shingle_size: usize,
    pub exact_only: bool,
}

impl From<&Cli> for DedupConfig {
    fn from(args: &Cli) -> Self {
        Self {
            threshold: args.threshold,
            num_perm: args.num_perm,
            shingle_size: args.shingle_size,
            exact_only: args.exact_only,
        }
    }
}

pub struct DedupResult {
    pub cluster_ids: Vec<usize>,
    pub is_representative: Vec<bool>,
    pub total: usize,
    pub exact_dupes: usize,
    pub near_dupes: usize,
    pub unique_clusters: usize,
}

/// Pure computation: deduplicate a slice of texts, return cluster assignments.
/// No I/O, no CLI dependency — suitable for library use and PyO3 bindings.
pub fn deduplicate(texts: &[String], config: &DedupConfig) -> DedupResult {
    let total = texts.len();

    if total == 0 {
        return DedupResult {
            cluster_ids: vec![],
            is_representative: vec![],
            total: 0,
            exact_dupes: 0,
            near_dupes: 0,
            unique_clusters: 0,
        };
    }

    let mut dedup = ExactDedup::new();
    let mut is_exact_dup = vec![false; total];
    let mut exact_dup_count = 0usize;

    for (i, text) in texts.iter().enumerate() {
        if dedup.is_duplicate(text) {
            is_exact_dup[i] = true;
            exact_dup_count += 1;
        }
    }

    let exact_groups = build_exact_groups(texts);

    let unique_indices: Vec<usize> = (0..total).filter(|&i| !is_exact_dup[i]).collect();
    let n_unique = unique_indices.len();

    let mut uf = UnionFind::new(n_unique);
    let mut near_dup_count = 0usize;

    if !config.exact_only && n_unique > 1 {
        let hasher = MinHasher::new(config.num_perm);

        let signatures: Vec<_> = unique_indices
            .par_iter()
            .map(|&i| {
                let shingles = shingle::shingles(&texts[i], config.shingle_size);
                hasher.signature(&shingles)
            })
            .collect();

        let (bands, rows) = lsh::optimal_params(config.threshold, config.num_perm);
        let mut lsh_index = LshIndex::new(bands, rows);

        for (pos, sig) in signatures.iter().enumerate() {
            lsh_index.insert(pos, sig);
        }

        let candidate_pairs = lsh_index.candidate_pairs();

        for (a, b) in &candidate_pairs {
            let sim = MinHasher::jaccard(&signatures[*a], &signatures[*b]);
            if sim >= config.threshold {
                uf.union(*a, *b);
            }
        }
    }

    let uf_clusters = uf.clusters();

    let mut cluster_ids = vec![0usize; total];
    let mut is_representative = vec![false; total];
    let mut next_cluster_id = 0usize;

    let mut uf_roots: Vec<usize> = uf_clusters.keys().copied().collect();
    uf_roots.sort_unstable();

    for root in &uf_roots {
        let members = &uf_clusters[root];
        let cid = next_cluster_id;
        next_cluster_id += 1;

        for (j, &pos) in members.iter().enumerate() {
            let orig_idx = unique_indices[pos];
            cluster_ids[orig_idx] = cid;
            is_representative[orig_idx] = j == 0;
        }

        if members.len() > 1 {
            near_dup_count += members.len() - 1;
        }
    }

    for group in exact_groups.values() {
        if group.len() < 2 {
            continue;
        }
        let rep_idx = group[0];
        let rep_cluster = cluster_ids[rep_idx];

        for &dup_idx in &group[1..] {
            cluster_ids[dup_idx] = rep_cluster;
            is_representative[dup_idx] = false;
        }
    }

    DedupResult {
        cluster_ids,
        is_representative,
        total,
        exact_dupes: exact_dup_count,
        near_dupes: near_dup_count,
        unique_clusters: next_cluster_id,
    }
}

/// CLI entry point: read JSONL → deduplicate → write output.
pub fn run(args: &Cli) -> Result<()> {
    let reader = crate::io::open_reader(&args.input)?;
    let mut writer = crate::io::open_writer(args.output.as_deref())?;

    let mut docs: Vec<Value> = Vec::new();
    let mut texts: Vec<String> = Vec::new();

    for (line_num, result) in crate::io::read_jsonl(reader) {
        match result {
            Ok(value) => match crate::io::extract_field(&value, &args.field) {
                Some(text) => {
                    texts.push(text.to_owned());
                    docs.push(value);
                }
                None => eprintln!("warning: line {line_num} missing field '{}'", args.field),
            },
            Err(e) => eprintln!("warning: {e}"),
        }
    }

    let config = DedupConfig::from(args);
    let result = deduplicate(&texts, &config);

    if args.clusters {
        for (i, mut doc) in docs.into_iter().enumerate() {
            if let Some(obj) = doc.as_object_mut() {
                obj.insert("cluster_id".into(), Value::from(result.cluster_ids[i] as u64));
                obj.insert("is_representative".into(), Value::from(result.is_representative[i]));
            }
            crate::io::write_jsonl(&mut writer, &doc)?;
        }
    } else {
        for (i, doc) in docs.iter().enumerate() {
            if result.is_representative[i] {
                crate::io::write_jsonl(&mut writer, doc)?;
            }
        }
    }

    if args.stats {
        eprintln!("total docs: {}", result.total);
        eprintln!("exact duplicates: {}", result.exact_dupes);
        eprintln!("near duplicates: {}", result.near_dupes);
        eprintln!("unique clusters: {}", result.unique_clusters);
        eprintln!("unique docs emitted: {}", result.unique_clusters);
    }

    Ok(())
}

fn build_exact_groups(texts: &[String]) -> HashMap<u64, Vec<usize>> {
    use std::hash::{Hash, Hasher};
    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();

    for (i, text) in texts.iter().enumerate() {
        let mut hasher = ahash::AHasher::default();
        text.hash(&mut hasher);
        let h = hasher.finish();
        groups.entry(h).or_default().push(i);
    }

    groups
}
