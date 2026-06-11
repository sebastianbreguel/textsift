/// A MinHash signature: one minimum hash per permutation slot.
pub type Signature = Vec<u64>;

/// Computes MinHash signatures over pre-hashed shingles.
///
/// Each shingle is hashed to a `u64` once (see [`crate::shingle::shingle_hashes`]);
/// every "permutation" is then a cheap universal-hash transform
/// `a.wrapping_mul(h).wrapping_add(b)` with an odd `a` (a bijection on u64,
/// so minima are well-distributed). This is O(shingles) byte-hashing plus
/// O(shingles × num_perm) multiply-adds — versus re-hashing the full shingle
/// bytes per permutation. Parameters derive deterministically from a fixed
/// seed, so results are reproducible.
pub struct MinHasher {
    /// (odd multiplier, offset) per permutation slot.
    params: Vec<(u64, u64)>,
}

const BASE_SEED: u64 = 0xdeadbeefcafebabe;

/// splitmix64 — standard seed-stream generator.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

impl MinHasher {
    /// Create a new `MinHasher` with `num_perm` permutation slots.
    pub fn new(num_perm: usize) -> Self {
        let mut s = BASE_SEED;
        let params = (0..num_perm)
            .map(|_| {
                let a = splitmix64(&mut s) | 1; // odd → bijective mod 2^64
                let b = splitmix64(&mut s);
                (a, b)
            })
            .collect();

        Self { params }
    }

    /// Compute the MinHash signature for a slice of shingle hashes.
    ///
    /// For each permutation slot, transforms every shingle hash and keeps
    /// the minimum. Returns a `Vec<u64>` of length `num_perm`.
    pub fn signature(&self, shingle_hashes: &[u64]) -> Signature {
        let mut mins = vec![u64::MAX; self.params.len()];

        for &h in shingle_hashes {
            for (slot, &(a, b)) in self.params.iter().enumerate() {
                let val = a.wrapping_mul(h).wrapping_add(b);
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
        let hashes = vec![17u64, 42, 1000];
        let sig1 = mh.signature(&hashes);
        let sig2 = mh.signature(&hashes);
        assert_eq!(sig1, sig2);
        assert!((MinHasher::jaccard(&sig1, &sig2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deterministic() {
        let mh = MinHasher::new(128);
        let hashes = vec![3u64, 7, 11];
        assert_eq!(mh.signature(&hashes), mh.signature(&hashes));
    }
}
