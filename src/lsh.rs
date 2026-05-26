use ahash::{HashMap, HashSet, RandomState};
use std::hash::{BuildHasher, Hasher};

/// LSH banding index for fast candidate-pair detection from MinHash signatures.
///
/// Splits each signature into `bands` chunks of `rows` hash values.
/// Two documents that share a chunk in any band become a candidate pair.
pub struct LshIndex {
    bands: usize,
    rows: usize,
    /// One HashMap per band: band_hash → list of doc IDs that fell into that bucket.
    tables: Vec<HashMap<u64, Vec<usize>>>,
    hash_state: RandomState,
}

/// Find the (bands, rows) pair whose S-curve inflection `(1/b)^(1/r)` is
/// closest to `threshold`, subject to `bands * rows == num_perm`.
pub fn optimal_params(threshold: f64, num_perm: usize) -> (usize, usize) {
    let mut best: Option<(usize, usize)> = None;
    let mut best_dist = f64::MAX;

    for b in 1..=num_perm {
        if num_perm % b != 0 {
            continue;
        }
        let r = num_perm / b;
        // S-curve inflection point: the similarity at which ~50% of pairs are candidates
        let inflection = (1.0 / b as f64).powf(1.0 / r as f64);
        let dist = (inflection - threshold).abs();
        if dist < best_dist {
            best_dist = dist;
            best = Some((b, r));
        }
    }

    best.expect("num_perm must be >= 1")
}

impl LshIndex {
    /// Create a new index with the given band structure.
    pub fn new(bands: usize, rows: usize) -> Self {
        let tables = (0..bands).map(|_| HashMap::default()).collect();
        Self {
            bands,
            rows,
            tables,
            hash_state: RandomState::with_seeds(42, 43, 44, 45),
        }
    }

    /// Insert a document's MinHash signature into the index.
    ///
    /// Splits the signature into `bands` chunks of `rows` elements, hashes each
    /// chunk to a single u64 bucket key, and records `doc_id` in that bucket.
    pub fn insert(&mut self, doc_id: usize, signature: &[u64]) {
        assert_eq!(
            signature.len(),
            self.bands * self.rows,
            "signature length must equal bands * rows"
        );

        for (band_idx, chunk) in signature.chunks(self.rows).enumerate() {
            let bucket = self.hash_chunk(chunk);
            self.tables[band_idx]
                .entry(bucket)
                .or_default()
                .push(doc_id);
        }
    }

    /// Collect all candidate pairs across every band.
    ///
    /// For each bucket with >1 document, emits all unique (i, j) pairs where i < j.
    pub fn candidate_pairs(&self) -> Vec<(usize, usize)> {
        let mut seen = HashSet::default();

        for table in &self.tables {
            for docs in table.values() {
                if docs.len() < 2 {
                    continue;
                }
                for i in 0..docs.len() {
                    for j in (i + 1)..docs.len() {
                        let pair = if docs[i] < docs[j] {
                            (docs[i], docs[j])
                        } else {
                            (docs[j], docs[i])
                        };
                        seen.insert(pair);
                    }
                }
            }
        }

        seen.into_iter().collect()
    }

    /// Hash a chunk of u64 values into a single bucket key.
    fn hash_chunk(&self, chunk: &[u64]) -> u64 {
        let mut h = self.hash_state.build_hasher();
        for &val in chunk {
            h.write_u64(val);
        }
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimal_params_product() {
        let (b, r) = optimal_params(0.5, 128);
        assert_eq!(b * r, 128);
    }

    #[test]
    fn optimal_params_threshold_08() {
        let (b, r) = optimal_params(0.8, 128);
        assert_eq!(b * r, 128);
        let inflection = (1.0 / b as f64).powf(1.0 / r as f64);
        assert!(
            (inflection - 0.8).abs() < 0.1,
            "inflection {inflection} too far from 0.8"
        );
    }

    #[test]
    fn identical_sigs_are_candidates() {
        let (b, r) = optimal_params(0.5, 16);
        let mut idx = LshIndex::new(b, r);
        let sig = vec![42u64; 16];
        idx.insert(0, &sig);
        idx.insert(1, &sig);
        let pairs = idx.candidate_pairs();
        assert!(pairs.contains(&(0, 1)));
    }

    #[test]
    fn pairs_are_deduplicated() {
        let (b, r) = optimal_params(0.5, 16);
        let mut idx = LshIndex::new(b, r);
        let sig = vec![99u64; 16];
        idx.insert(0, &sig);
        idx.insert(1, &sig);
        let pairs = idx.candidate_pairs();
        // Should appear exactly once despite matching in every band
        assert_eq!(pairs.iter().filter(|&&p| p == (0, 1)).count(), 1);
    }
}
