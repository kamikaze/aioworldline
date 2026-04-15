pub mod conf;
pub mod error;
pub mod worldline;

pub use conf::Settings;
pub use error::WorldlineError;
pub use worldline::{ReportOptions, WorldlineSession, extract_csrf};

#[cfg(feature = "python")]
mod python;

#[cfg(feature = "python")]
static TOKIO_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

/// Python extension module entry point, compiled only when the `python` feature
/// is enabled (e.g. via `maturin build`).
#[cfg(feature = "python")]
#[pyo3::pymodule]
fn aioworldline(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {
    let rt = TOKIO_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build Tokio runtime")
    });
    // Enter the runtime permanently on this thread so that pyo3's native async
    // coroutines (polled by Python's asyncio event loop) have Tokio reactor
    // context for reqwest and tokio::time calls.
    std::mem::forget(rt.enter());
    python::register(m)
}
