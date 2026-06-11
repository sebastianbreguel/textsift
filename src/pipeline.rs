use anyhow::Result;
use rayon::prelude::*;
use serde_json::Value;

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

impl DedupConfig {
    /// Check parameter ranges. `deduplicate` panics on invalid configs
    /// (`shingle_size == 0`, `num_perm == 0`), so every user-facing surface
    /// must call this first.
    pub fn validate(&self) -> Result<(), String> {
        if self.num_perm == 0 {
            return Err("num_perm must be at least 1".to_string());
        }
        if self.shingle_size == 0 {
            return Err("shingle_size must be at least 1".to_string());
        }
        if self.threshold.is_nan() || !(0.0..=1.0).contains(&self.threshold) {
            return Err(format!(
                "threshold must be between 0.0 and 1.0, got {}",
                self.threshold
            ));
        }
        Ok(())
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
///
/// # Panics
///
/// Panics on invalid configs (`num_perm == 0`, `shingle_size == 0`).
/// Call [`DedupConfig::validate`] first, as the CLI and Python surfaces do.
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
        if dedup.process(i, text) {
            is_exact_dup[i] = true;
            exact_dup_count += 1;
        }
    }

    let exact_groups = dedup.into_groups();

    let unique_indices: Vec<usize> = (0..total).filter(|&i| !is_exact_dup[i]).collect();
    let n_unique = unique_indices.len();

    let mut uf = UnionFind::new(n_unique);
    let mut near_dup_count = 0usize;

    if !config.exact_only && n_unique > 1 {
        let hasher = MinHasher::new(config.num_perm);

        let signatures: Vec<_> = unique_indices
            .par_iter()
            .map(|&i| {
                let shingles = shingle::shingle_hashes(&texts[i], config.shingle_size);
                hasher.signature(&shingles)
            })
            .collect();

        let (bands, rows) = lsh::optimal_params(config.threshold, config.num_perm);
        let mut lsh_index = LshIndex::new(bands, rows);

        for (pos, sig) in signatures.iter().enumerate() {
            lsh_index.insert(pos, sig);
        }

        lsh_index.for_each_candidate_pair(|a, b| {
            let sim = MinHasher::jaccard(&signatures[a], &signatures[b]);
            if sim >= config.threshold {
                uf.union(a, b);
            }
        });
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
    let config = DedupConfig::from(args);
    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    let reader = crate::io::open_reader(&args.input)?;
    let mut writer = crate::io::open_writer(args.output.as_deref())?;

    // Raw input lines (emitted untouched in default mode — byte-identical
    // passthrough), each paired with whether it carries the dedup field.
    // Extracted texts (in line order) feed `deduplicate`.
    let mut lines: Vec<(String, bool)> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let mut missing_field = 0usize;

    for (line_num, line_result) in crate::io::read_lines(reader) {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("warning: line {line_num}: {e}");
                continue;
            }
        };
        match serde_json::from_str::<Value>(&line) {
            Ok(value) => {
                let has_field = match crate::io::extract_field(&value, &args.field) {
                    Some(text) => {
                        texts.push(text.to_owned());
                        true
                    }
                    None => {
                        eprintln!(
                            "warning: line {line_num} missing field '{}' — passed through",
                            args.field
                        );
                        missing_field += 1;
                        false
                    }
                };
                lines.push((line, has_field));
            }
            Err(e) => eprintln!("warning: invalid JSON on line {line_num}: {e}"),
        }
    }

    let result = deduplicate(&texts, &config);

    // Walk lines in input order; text_idx advances only over lines that
    // participated in dedup. Lines without the field pass through as unique,
    // taking fresh cluster ids after the dedup clusters.
    let mut text_idx = 0usize;
    let mut next_cluster = result.unique_clusters;

    for (line, has_field) in &lines {
        let (cid, rep) = if *has_field {
            let v = (
                result.cluster_ids[text_idx],
                result.is_representative[text_idx],
            );
            text_idx += 1;
            v
        } else {
            let cid = next_cluster;
            next_cluster += 1;
            (cid, true)
        };

        if args.clusters {
            // Re-parse only in clusters mode, where fields must be injected.
            // Deliberate CPU-for-memory trade: storing parsed Values for the
            // whole corpus costs far more RAM than re-parsing on emit.
            let mut doc: Value = serde_json::from_str(line)?;
            if let Some(obj) = doc.as_object_mut() {
                obj.insert("cluster_id".into(), Value::from(cid as u64));
                obj.insert("is_representative".into(), Value::from(rep));
            }
            crate::io::write_jsonl(&mut writer, &doc)?;
        } else if rep {
            crate::io::write_line(&mut writer, line)?;
        }
    }

    if args.stats {
        eprintln!("total docs: {}", lines.len());
        eprintln!("exact duplicates: {}", result.exact_dupes);
        eprintln!("near duplicates: {}", result.near_dupes);
        eprintln!("unique clusters: {}", result.unique_clusters);
        if missing_field > 0 {
            eprintln!("docs without field (passed through): {missing_field}");
        }
        eprintln!(
            "unique docs emitted: {}",
            result.unique_clusters + missing_field
        );
    }

    Ok(())
}
