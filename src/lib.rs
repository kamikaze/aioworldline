pub mod conf;
pub mod error;
pub mod worldline;

pub use conf::Settings;
pub use error::WorldlineError;
pub use worldline::{ReportOptions, WorldlineSession, extract_csrf};

#[cfg(feature = "python")]
mod python;

/// Python extension module entry point.
///
/// Built when the `python` feature is enabled (e.g. via maturin).
#[cfg(feature = "python")]
#[pyo3::pymodule]
fn aioworldline(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {
    python::init_module(m)
}
