use crate::worldline::{ReportOptions, WorldlineSession};
use chrono::NaiveDate;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDate, PyDateAccess};
use pyo3_async_runtimes::tokio::future_into_py;
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

#[pyclass(name = "WorldlineSession")]
pub struct PyWorldlineSession {
    inner: WorldlineSession,
}

impl PyWorldlineSession {
    pub fn new(inner: WorldlineSession) -> Self {
        Self { inner }
    }
}

/// Python date -> chrono `NaiveDate`
fn pydate_to_naive(d: &Bound<'_, PyDate>) -> PyResult<NaiveDate> {
    NaiveDate::from_ymd_opt(d.get_year(), d.get_month().into(), d.get_day().into())
        .ok_or_else(|| PyRuntimeError::new_err("invalid date"))
}

#[pymethods]
impl PyWorldlineSession {
    fn get_transaction_report<'py>(
        &self,
        py: Python<'py>,
        date_from: &Bound<'py, PyAny>,
        date_till: &Bound<'py, PyAny>,
        account_id: String,
    ) -> PyResult<Bound<'py, PyAny>> {
        let wl_session = self.inner.clone();

        let date_from = pydate_to_naive(date_from.cast::<PyDate>()?)?;
        let date_till = pydate_to_naive(date_till.cast::<PyDate>()?)?;

        let account_id_str = SecretString::new(account_id.into())
            .expose_secret()
            .to_owned();

        future_into_py(py, async move {
            let opts = ReportOptions {
                account_id: &account_id_str,
                ..Default::default()
            };

            let bytes: Vec<u8> = wl_session
                .get_transaction_report(date_from, date_till, &opts)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            Python::try_attach(|py| Ok(PyBytes::new(py, &bytes).into_any().unbind()))
                .ok_or_else(|| PyRuntimeError::new_err("Python interpreter not attached"))?
        })
    }
}

#[pyfunction]
fn login(
    py: Python<'_>,
    username: String,
    password: String,
    timeout: Option<u64>,
) -> PyResult<Bound<'_, PyAny>> {
    let timeout = timeout.map(Duration::from_secs);

    future_into_py(py, async move {
        let password = SecretString::new(password.into());
        let session = WorldlineSession::login(&username, &password, timeout)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Python::try_attach(|py| {
            Ok(PyWorldlineSession::new(session)
                .into_pyobject(py)?
                .into_any()
                .unbind())
        })
        .ok_or_else(|| PyRuntimeError::new_err("Python interpreter not attached"))?
    })
}

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorldlineSession>()?;
    m.add_function(wrap_pyfunction!(login, m)?)?;
    Ok(())
}
