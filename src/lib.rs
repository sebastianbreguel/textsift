pub mod cli;
pub mod cluster;
pub mod exact;
pub mod io;
pub mod lsh;
pub mod minhash;
pub mod pipeline;
#[cfg(feature = "python")]
pub mod python;
pub mod shingle;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn textsift(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register(m)
}
