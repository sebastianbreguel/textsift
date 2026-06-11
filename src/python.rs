use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::pipeline::{deduplicate, DedupConfig, DedupResult as RustDedupResult};

// skip_from_py_object: results flow Rust→Python only, never back in.
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct DedupResult {
    #[pyo3(get)]
    pub cluster_ids: Vec<usize>,
    #[pyo3(get)]
    pub is_representative: Vec<bool>,
    /// Estimated Jaccard similarity to the cluster representative
    /// (1.0 for representatives, singletons, and exact duplicates).
    #[pyo3(get)]
    pub similarity: Vec<f64>,
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
            similarity: r.similarity,
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
    py: Python<'_>,
    texts: Vec<String>,
    threshold: f64,
    num_perm: usize,
    shingle_size: usize,
    exact_only: bool,
) -> PyResult<DedupResult> {
    let config = DedupConfig {
        threshold,
        num_perm,
        shingle_size,
        exact_only,
    };
    config.validate().map_err(PyValueError::new_err)?;

    // Release the GIL (detach): dedup is pure Rust and can run for seconds on
    // large corpora — other Python threads shouldn't be blocked meanwhile.
    let result = py.detach(|| deduplicate(&texts, &config));
    Ok(result.into())
}

/// Deduplicate dict-like records on one or more string fields.
///
/// Returns `(clean_records, result)`: the kept records (cluster
/// representatives, plus any record missing a requested field — those pass
/// through as unique, matching the CLI), and a `DedupResult` whose arrays
/// align 1:1 with the INPUT records list.
#[pyfunction]
#[pyo3(signature = (records, fields, threshold=0.8, num_perm=128, shingle_size=5, exact_only=false))]
#[allow(clippy::too_many_arguments)]
fn dedup_records<'py>(
    py: Python<'py>,
    records: Vec<Bound<'py, PyAny>>,
    fields: Vec<String>,
    threshold: f64,
    num_perm: usize,
    shingle_size: usize,
    exact_only: bool,
) -> PyResult<(Vec<Bound<'py, PyAny>>, DedupResult)> {
    if fields.is_empty() {
        return Err(PyValueError::new_err("fields must not be empty"));
    }
    let config = DedupConfig {
        threshold,
        num_perm,
        shingle_size,
        exact_only,
    };
    config.validate().map_err(PyValueError::new_err)?;

    // Extract the dedup key per record under the GIL. None = some field
    // missing or non-string → record passes through as unique.
    let keys: Vec<Option<String>> = records
        .iter()
        .map(|rec| {
            let parts: Option<Vec<String>> = fields
                .iter()
                .map(|f| {
                    rec.get_item(f)
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                })
                .collect();
            parts.map(|p| {
                if p.len() == 1 {
                    p.into_iter().next().unwrap()
                } else {
                    p.join(" \u{1F} ")
                }
            })
        })
        .collect();

    let texts: Vec<String> = keys.iter().flatten().cloned().collect();
    let core = py.detach(|| deduplicate(&texts, &config));

    // Re-align the result onto the full records list: keyless records get a
    // fresh cluster after the dedup clusters, like the CLI's --clusters mode.
    let n = records.len();
    let mut cluster_ids = Vec::with_capacity(n);
    let mut is_representative = Vec::with_capacity(n);
    let mut similarity = Vec::with_capacity(n);
    let mut next_cluster = core.unique_clusters;
    let mut text_idx = 0usize;
    let mut missing = 0usize;

    for key in &keys {
        if key.is_some() {
            cluster_ids.push(core.cluster_ids[text_idx]);
            is_representative.push(core.is_representative[text_idx]);
            similarity.push(core.similarity[text_idx]);
            text_idx += 1;
        } else {
            cluster_ids.push(next_cluster);
            next_cluster += 1;
            is_representative.push(true);
            similarity.push(1.0);
            missing += 1;
        }
    }

    let clean: Vec<Bound<'py, PyAny>> = records
        .into_iter()
        .zip(is_representative.iter())
        .filter(|(_, &rep)| rep)
        .map(|(r, _)| r)
        .collect();

    let result = DedupResult {
        cluster_ids,
        is_representative,
        similarity,
        total: n,
        exact_dupes: core.exact_dupes,
        near_dupes: core.near_dupes,
        unique_clusters: core.unique_clusters + missing,
    };

    Ok((clean, result))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(dedup, m)?)?;
    m.add_function(wrap_pyfunction!(dedup_records, m)?)?;
    m.add_class::<DedupResult>()?;
    Ok(())
}
