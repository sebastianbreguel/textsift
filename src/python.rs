use pyo3::prelude::*;

use crate::pipeline::{DedupConfig, DedupResult as RustDedupResult, deduplicate};

#[pyclass]
#[derive(Clone)]
pub struct DedupResult {
    #[pyo3(get)]
    pub cluster_ids: Vec<usize>,
    #[pyo3(get)]
    pub is_representative: Vec<bool>,
    #[pyo3(get)]
    pub total: usize,
    #[pyo3(get)]
    pub exact_dupes: usize,
    #[pyo3(get)]
    pub near_dupes: usize,
    #[pyo3(get)]
    pub unique_clusters: usize,
}

#[pymethods]
impl DedupResult {
    pub fn stats(&self) -> String {
        format!(
            "total: {}, exact_dupes: {}, near_dupes: {}, unique: {}",
            self.total, self.exact_dupes, self.near_dupes, self.unique_clusters
        )
    }

    pub fn unique_indices(&self) -> Vec<usize> {
        self.is_representative
            .iter()
            .enumerate()
            .filter(|(_, &r)| r)
            .map(|(i, _)| i)
            .collect()
    }
}

impl From<RustDedupResult> for DedupResult {
    fn from(r: RustDedupResult) -> Self {
        Self {
            cluster_ids: r.cluster_ids,
            is_representative: r.is_representative,
            total: r.total,
            exact_dupes: r.exact_dupes,
            near_dupes: r.near_dupes,
            unique_clusters: r.unique_clusters,
        }
    }
}

#[pyfunction]
#[pyo3(signature = (texts, threshold=0.8, num_perm=128, shingle_size=5, exact_only=false))]
fn dedup(
    texts: Vec<String>,
    threshold: f64,
    num_perm: usize,
    shingle_size: usize,
    exact_only: bool,
) -> DedupResult {
    let config = DedupConfig {
        threshold,
        num_perm,
        shingle_size,
        exact_only,
    };
    deduplicate(&texts, &config).into()
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(dedup, m)?)?;
    m.add_class::<DedupResult>()?;
    Ok(())
}
