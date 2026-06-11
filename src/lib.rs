// Public API: `pipeline` (deduplicate, DedupConfig, DedupResult) and the CLI
// surface. Everything else is implementation detail — `pub` so integration
// tests can reach it, but hidden from docs and not covered by semver.
pub mod cli;
pub mod pipeline;

#[doc(hidden)]
pub mod cluster;
#[doc(hidden)]
pub mod exact;
#[doc(hidden)]
pub mod io;
#[doc(hidden)]
pub mod lsh;
#[doc(hidden)]
pub mod minhash;
#[doc(hidden)]
pub mod shingle;

#[cfg(feature = "python")]
#[doc(hidden)]
pub mod python;

pub use pipeline::{deduplicate, DedupConfig, DedupResult};

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn textsift(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register(m)
}
