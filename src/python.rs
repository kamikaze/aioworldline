//! Python asyncio bindings for the Worldline client.
//!
//! Compiled only when the `python` feature is enabled.  Exposes
//! `WorldlineSession` as an awaitable Python class backed by Tokio.
//!
//! # Usage
//!
//! ```python
//! import asyncio
//! from datetime import date
//! from aioworldline import WorldlineSession
//!
//! async def main():
//!     session = await WorldlineSession.login("user", "s3cr3t", timeout=900)
//!     csv_bytes: bytes = await session.get_transaction_report(
//!         date_from=date(2024, 1, 1),
//!         date_till=date(2024, 1, 31),
//!         account_id="123456",
//!     )
//!     print(csv_bytes.decode())
//!
//! asyncio.run(main())
//! ```

use std::time::Duration;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyType;
use secrecy::SecretString;

use crate::worldline::WorldlineSession;

/// An authenticated Worldline portal session.
///
/// Obtain one by awaiting `WorldlineSession.login(...)`.
#[pyclass(name = "WorldlineSession")]
pub struct PyWorldlineSession {
    inner: WorldlineSession,
}

#[pymethods]
impl PyWorldlineSession {
    /// Perform the two-step login sequence and return an authenticated session.
    ///
    /// Args:
    ///     `username`:     Portal username.
    ///     `password`:     Portal password (not stored after login).
    ///     `timeout`: Optional per-request HTTP timeout in seconds.
    #[classmethod]
    #[pyo3(signature = (username, password, timeout = None))]
    async fn login(
        _cls: Py<PyType>,
        username: String,
        password: String,
        timeout: Option<u64>,
    ) -> PyResult<Self> {
        let timeout = timeout.map(Duration::from_secs);
        WorldlineSession::login(&username, &SecretString::new(password.into()), timeout)
            .await
            .map(|inner| Self { inner })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Fetch raw CSV bytes for the given date range.
    ///
    /// The portal returns UTF-8 with a BOM (`\\xEF\\xBB\\xBF`); strip it with
    /// `data.lstrip(b'\\xef\\xbb\\xbf')` or `data.decode('utf-8-sig')` if needed.
    ///
    /// Args:
    ///     `date_from`:    Start of the report period (`datetime.date`).
    ///     `date_till`:    End of the report period (`datetime.date`).
    ///     `account_id`:   Portal merchant account ID.
    ///     `date_type`:    `"D"` for settlement date, `"T"` for transaction date.
    ///     `use_date`:     Date reference type (default `"TR"`).
    ///     merchant:     Optional merchant filter.
    ///     `term_id`:      Optional terminal ID filter.
    ///     `export_type`:  Export format sent to portal (default `"csv"`).
    ///
    /// Returns:
    ///     Raw `bytes` payload from the portal.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        date_from,
        date_till,
        account_id,
        date_type = None,
        use_date = None,
        merchant = None,
        term_id = None,
        export_type = None,
    ))]
    async fn get_transaction_report(
        &self,
        date_from: chrono::NaiveDate,
        date_till: chrono::NaiveDate,
        account_id: String,
        date_type: Option<String>,
        use_date: Option<String>,
        merchant: Option<String>,
        term_id: Option<String>,
        export_type: Option<String>,
    ) -> PyResult<Vec<u8>> {
        let date_type = date_type.unwrap_or_else(|| "D".to_owned());
        let use_date = use_date.unwrap_or_else(|| "TR".to_owned());
        let export_type = export_type.unwrap_or_else(|| "csv".to_owned());
        let opts = crate::worldline::ReportOptions {
            account_id: &account_id,
            date_type: &date_type,
            use_date: &use_date,
            merchant: merchant.as_deref(),
            term_id: term_id.as_deref(),
            export_type: &export_type,
        };
        self.inner
            .get_transaction_report(date_from, date_till, &opts)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
}

/// Register all Python-exposed types and functions into the extension module.
pub fn register(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
    m.add_class::<PyWorldlineSession>()?;
    Ok(())
}
