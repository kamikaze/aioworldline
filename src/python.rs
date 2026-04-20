use crate::worldline::{ReportOptions, WorldlineSession};
use chrono::NaiveDate;
use pyo3::IntoPyObjectExt;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDate, PyDateAccess, PyModule};
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

/// Convert a Python `datetime.date` to a chrono [`NaiveDate`].
fn pydate_to_naive(d: &Bound<'_, PyDate>) -> PyResult<NaiveDate> {
    NaiveDate::from_ymd_opt(d.get_year(), d.get_month().into(), d.get_day().into())
        .ok_or_else(|| PyRuntimeError::new_err("invalid date"))
}

// ── WorldlineSession ─────────────────────────────────────────────────────────

#[pyclass(name = "WorldlineSession")]
pub struct PyWorldlineSession {
    inner: WorldlineSession,
}

#[pymethods]
impl PyWorldlineSession {
    /// Fetch a transaction report and return the raw bytes.
    fn get_transaction_report<'py>(
        &self,
        py: Python<'py>,
        date_from: &Bound<'py, PyDate>,
        date_till: &Bound<'py, PyDate>,
        account_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let session = self.inner.clone();
        let date_from = pydate_to_naive(date_from)?;
        let date_till = pydate_to_naive(date_till)?;

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let account_id: SecretString = account_id.into();
            let opts = ReportOptions {
                account_id: account_id.expose_secret(),
                ..Default::default()
            };
            let bytes = session
                .get_transaction_report(date_from, date_till, &opts)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            Python::attach(|py| PyBytes::new(py, &bytes).into_py_any(py))
        })
    }
}

// ── Login context manager ────────────────────────────────────────────────────

#[pyclass(name = "Login")]
pub struct PyWorldlineLogin {
    username: String,
    password: String,
    timeout_secs: Option<u64>,
}

impl PyWorldlineLogin {
    fn new(username: String, password: String, timeout_secs: Option<u64>) -> Self {
        Self {
            username,
            password,
            timeout_secs,
        }
    }
}

#[pymethods]
impl PyWorldlineLogin {
    #[allow(clippy::needless_pass_by_value)]
    fn __aenter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let username = slf.username.clone();
        let password = slf.password.clone();
        let timeout = slf.timeout_secs.map(Duration::from_secs);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let password: SecretString = password.into();
            let session = WorldlineSession::login(&username, &password, timeout)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            Python::attach(|py| Py::new(py, PyWorldlineSession { inner: session })?.into_py_any(py))
        })
    }

    #[allow(clippy::unused_self)]
    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }
}

// ── Module registration ──────────────────────────────────────────────────────

#[pyfunction]
fn login(username: String, password: String, timeout: Option<u64>) -> PyWorldlineLogin {
    PyWorldlineLogin::new(username, password, timeout)
}

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorldlineSession>()?;
    m.add_class::<PyWorldlineLogin>()?;
    m.add_function(wrap_pyfunction!(login, m)?)?;

    // Re-export everything under a `worldline` submodule so that both
    // `import mylib` and `from mylib import worldline` work.
    let sub = PyModule::new(m.py(), "worldline")?;
    sub.add_class::<PyWorldlineSession>()?;
    sub.add_class::<PyWorldlineLogin>()?;
    sub.add_function(wrap_pyfunction!(login, &sub)?)?;
    m.add_submodule(&sub)?;
    // The double `add` is intentional: `add_submodule` registers the module in
    // `sys.modules` while `add` makes it accessible as an attribute.
    m.add("worldline", sub)?;

    Ok(())
}
