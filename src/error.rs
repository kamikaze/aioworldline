use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorldlineError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("CSRF token not found in page HTML")]
    CsrfNotFound,

    #[error("failed to switch merchant account (HTTP {status})")]
    MerchantSwitchFailed { status: reqwest::StatusCode },

    #[error("failed to open detailed turnover page (HTTP {status})")]
    TurnoverPageFailed { status: reqwest::StatusCode },

    #[error("failed to export transaction data (HTTP {status})")]
    ExportFailed { status: reqwest::StatusCode },
}
