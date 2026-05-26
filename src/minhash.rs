use ahash::RandomState;
use std::hash::{BuildHasher, Hasher};

/// A MinHash signature: one minimum hash per permutation slot.
pub type Signature = Vec<u64>;

/// Computes MinHash signatures over a set of shingles.
///
/// Each "permutation" is simulated by hashing every shingle with a different
/// seed and keeping the minimum hash value per slot. The seeds are derived
/// deterministically from a base seed so results are reproducible.
pub struct MinHasher {
    hashers: Vec<RandomState>,
}

const BASE_SEED: u64 = 0xdeadbeefcafebabe;

impl MinHasher {
    /// Create a new `MinHasher` with `num_perm` permutation slots.
    pub fn new(num_perm: usize) -> Self {
        let hashers = (0..num_perm)
            .map(|i| {
                let seed = BASE_SEED
                    .wrapping_mul(i as u64 + 1)
                    .wrapping_add(0x517cc1b727220a95);
                // ahash RandomState with fixed seeds → deterministic
                RandomState::with_seeds(seed, seed.wrapping_add(1), seed.wrapping_add(2), seed.wrapping_add(3))
            })
            .collect();

        Self { hashers }
    }

    /// Compute the MinHash signature for a slice of shingles.
    ///
    /// For each permutation slot, hashes every shingle and keeps the minimum.
    /// Returns a `Vec<u64>` of length `num_perm`.
    pub fn signature(&self, shingles: &[String]) -> Signature {
        let num_perm = self.hashers.len();
        let mut mins = vec![u64::MAX; num_perm];

        for s in shingles {
            let bytes = s.as_bytes();
            for (slot, state) in self.hashers.iter().enumerate() {
                let mut h = state.build_hasher();
                h.write(bytes);
                let val = h.finish();
                if val < mins[slot] {
                    mins[slot] = val;
                }
            }
        }

        mins
    }

    /// Estimate Jaccard similarity between two signatures.
    ///
    /// Returns the fraction of positions where `a[i] == b[i]`.
    /// Panics if signatures have different lengths.
    pub fn jaccard(a: &Signature, b: &Signature) -> f64 {
        assert_eq!(a.len(), b.len(), "signatures must have equal length");

        if a.is_empty() {
            return 1.0;
        }

        let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
        matches as f64 / a.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_signatures() {
        let mh = MinHasher::new(128);
        let shingles = vec!["a b c".to_string(), "b c d".to_string()];
        let sig1 = mh.signature(&shingles);
        let sig2 = mh.signature(&shingles);
        assert_eq!(sig1, sig2);
        assert!((MinHasher::jaccard(&sig1, &sig2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deterministic() {
        let mh = MinHasher::new(128);
        let shingles = vec!["x y".to_string(), "y z".to_string()];
        assert_eq!(mh.signature(&shingles), mh.signature(&shingles));
    }
}
