use std::collections::HashMap;

/// Disjoint-set (Union-Find) with path compression and union by rank.
///
/// Used to cluster document indices after LSH candidate-pair detection.
pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    /// Create `n` singleton sets numbered 0..n.
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    /// Find the root representative of element `x` with path compression.
    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    /// Merge the sets containing `x` and `y` using union by rank.
    pub fn union(&mut self, x: usize, y: usize) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        match self.rank[rx].cmp(&self.rank[ry]) {
            std::cmp::Ordering::Less => self.parent[rx] = ry,
            std::cmp::Ordering::Greater => self.parent[ry] = rx,
            std::cmp::Ordering::Equal => {
                self.parent[ry] = rx;
                self.rank[rx] += 1;
            }
        }
    }

    /// Group all elements by their root representative.
    ///
    /// Each cluster's members are sorted so the smallest index comes first
    /// (that index serves as the canonical representative).
    pub fn clusters(&mut self) -> HashMap<usize, Vec<usize>> {
        let n = self.parent.len();
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..n {
            let root = self.find(i);
            groups.entry(root).or_default().push(i);
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singletons() {
        let mut uf = UnionFind::new(3);
        let c = uf.clusters();
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn basic_union() {
        let mut uf = UnionFind::new(5);
        uf.union(0, 1);
        uf.union(1, 2);
        uf.union(3, 4);
        let c = uf.clusters();
        assert_eq!(c.len(), 2);
        // Find the cluster containing 0
        let root0 = uf.find(0);
        assert_eq!(c[&root0], vec![0, 1, 2]);
        let root3 = uf.find(3);
        assert_eq!(c[&root3], vec![3, 4]);
    }

    #[test]
    fn all_unioned() {
        let mut uf = UnionFind::new(4);
        uf.union(0, 1);
        uf.union(2, 3);
        uf.union(0, 3);
        let c = uf.clusters();
        assert_eq!(c.len(), 1);
        let members = c.values().next().unwrap();
        assert_eq!(members, &vec![0, 1, 2, 3]);
    }

    #[test]
    fn single_element() {
        let mut uf = UnionFind::new(1);
        let c = uf.clusters();
        assert_eq!(c.len(), 1);
        assert_eq!(c[&0], vec![0]);
    }
}
