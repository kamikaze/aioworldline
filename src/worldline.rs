use std::time::Duration;

use chrono::NaiveDate;
use reqwest::{Client, ClientBuilder};
use scraper::{Html, Selector};
use secrecy::{ExposeSecret, SecretString};
use tracing::{debug, error};

use crate::error::WorldlineError;

const LOGIN_PAGE_URL: &str = "https://portal.baltic.worldline-solutions.com/fdmp/login.jsp";
const AUTH_URL: &str = "https://portal.baltic.worldline-solutions.com/fdmp/j_security_check";
const MERCHANT_SWITCH_URL: &str =
    "https://portal.baltic.worldline-solutions.com/fdmp/transaction_info";
const DETAILED_TURNOVER_PAGE_URL: &str =
    "https://portal.baltic.worldline-solutions.com/fdmp/detailed_turnover";
const EXPORT_LIST_DATA_URL: &str =
    "https://portal.baltic.worldline-solutions.com/fdmp/export_list_data";

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/111.0";

/// Extract the hidden `__CSRF` input field value from an HTML page.
///
/// The portal currently accepts `"null"` for this field, but the function is
/// retained so callers can opt in to proper CSRF handling if the portal begins
/// enforcing it.
pub fn extract_csrf(html: &str) -> Result<String, WorldlineError> {
    let document = Html::parse_document(html);

    // Select every <input> that has an `id` attribute, then filter by value.
    // CSS attribute selectors are case-sensitive, so we normalise in Rust.
    let selector = Selector::parse("input[id]").expect("static selector is always valid");

    for element in document.select(&selector) {
        if element
            .attr("id")
            .is_some_and(|id| id.eq_ignore_ascii_case("__csrf"))
        {
            return Ok(element.attr("value").unwrap_or("").to_owned());
        }
    }

    error!("Unable to extract CSRF value from the page");
    Err(WorldlineError::CsrfNotFound)
}

/// An authenticated session backed by a `reqwest::Client` with a persistent
/// cookie jar.  `reqwest::Client` uses `Arc` internally, so `Clone` is cheap
/// and shares the same underlying connection pool and cookie store.
#[derive(Clone)]
pub struct WorldlineSession {
    client: Client,
}

impl WorldlineSession {
    /// Perform the two-step login sequence (GET login page → POST credentials)
    /// and return a live, authenticated session.
    pub async fn login(
        username: &str,
        password: &SecretString,
        timeout: Option<Duration>,
    ) -> Result<Self, WorldlineError> {
        let mut builder = ClientBuilder::new()
            .user_agent(USER_AGENT)
            .cookie_store(true)
            // Redirect following is on by default in reqwest; keep it so that
            // post-login redirects are handled transparently.
            .redirect(reqwest::redirect::Policy::limited(10));

        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }

        let client = builder.build()?;

        // ── Step 1: obtain session cookie ─────────────────────────────────────
        debug!("Opening login page to obtain session cookie");
        client.get(LOGIN_PAGE_URL).send().await?;

        // Brief pause to mimic human interaction; the portal is sensitive to
        // requests arriving too quickly.
        tokio::time::sleep(Duration::from_secs(5)).await;

        // ── Step 2: authenticate ──────────────────────────────────────────────
        debug!("Posting credentials to authentication endpoint");
        let params = [
            ("__Action", "login:b_login#Save#"),
            ("j_username", username),
            ("j_password", password.expose_secret()),
        ];
        client.post(AUTH_URL).form(&params).send().await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        Ok(Self { client })
    }

    /// Fetch a raw CSV byte payload for the given date range.
    ///
    /// The portal returns a UTF-8 file with a BOM (`\xEF\xBB\xBF`); stripping
    /// it is the caller's responsibility (see `main.rs`).
    pub async fn get_transaction_report(
        &self,
        date_from: NaiveDate,
        date_till: NaiveDate,
        account_id: &str,
        date_type: &str,
        use_date: &str,
        merchant: Option<&str>,
        term_id: Option<&str>,
        export_type: &str,
    ) -> Result<Vec<u8>, WorldlineError> {
        // ── Step 1: switch to the target merchant account ─────────────────────
        let switch_params = [
            ("__Action", "merchant:parent_id"),
            ("__CSRF", "null"),
            ("merchant:parent_id", account_id),
            ("transaction_info:news_id", ""),
        ];

        debug!("Switching merchant account to {account_id}");
        let resp = self
            .client
            .post(MERCHANT_SWITCH_URL)
            .query(&switch_params)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(WorldlineError::MerchantSwitchFailed {
                status: resp.status(),
            });
        }

        tokio::time::sleep(Duration::from_secs(10)).await;

        // ── Step 2: load the detailed turnover page (sets portal state) ───────
        debug!("Loading detailed turnover page");
        let resp = self
            .client
            .get(DETAILED_TURNOVER_PAGE_URL)
            .query(&[("group", "tab.detailed_turnover")])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(WorldlineError::TurnoverPageFailed {
                status: resp.status(),
            });
        }

        tokio::time::sleep(Duration::from_secs(10)).await;

        // ── Step 3: export ────────────────────────────────────────────────────
        let date_from_str = date_from.format("%d.%m.%Y").to_string();
        let date_till_str = date_till.format("%d.%m.%Y").to_string();

        // The portal uses placeholder-style return strings for lookup fields.
        // Unwrap to empty string when merchant / terminal are not filtered.
        let merchant_val = merchant.unwrap_or_default();
        let term_id_val = term_id.unwrap_or_default();

        let merchant_ret = format!(
            "detailed_turnover:merchant~{merchant_val}|\
             detailed_turnover:merchant_txt~{merchant_val} {{full_name}}|\
             detailed_turnover:merchant_order~{merchant_val}"
        );
        let term_ret = format!(
            "detailed_turnover:term_id~{term_id_val}|\
             detailed_turnover:term_id_txt~{term_id_val} {{term_type}}|\
             detailed_turnover:term_id_order~{term_id_val}"
        );

        let export_params: &[(&str, &str)] = &[
            ("uniqueid", "detailed_turnover:detailed_turnover_search_result"),
            ("exportType", export_type),
            ("page", "1"),
            ("countRow", "15"),
            ("sortField", ""),
            ("sortType", "0"),
            ("detailed_turnover:date_type", date_type),
            ("detailed_turnover:parent", account_id),
            ("detailed_turnover:shipm_date_from", &date_from_str),
            ("detailed_turnover:shipm_date_till", &date_till_str),
            ("detailed_turnover:use_date", use_date),
            ("detailed_turnover:merchant_ret", &merchant_ret),
            ("detailed_turnover:term_id_ret", &term_ret),
        ];

        debug!("Exporting transactions for {date_from} \u{2013} {date_till}");
        let resp = self
            .client
            .get(EXPORT_LIST_DATA_URL)
            .query(export_params)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(WorldlineError::ExportFailed {
                status: resp.status(),
            });
        }

        Ok(resp.bytes().await?.to_vec())
    }
}

/// Builder-style options for [`WorldlineSession::get_transaction_report`].
///
/// All fields have idiomatic defaults matching the Python original.
pub struct ReportOptions<'a> {
    pub account_id: &'a str,
    /// `"D"` = settlement date, `"T"` = transaction date.
    pub date_type: &'a str,
    /// `"TR"` = transaction date reference.
    pub use_date: &'a str,
    /// Filter by merchant ID; `None` means no filter.
    pub merchant: Option<&'a str>,
    /// Filter by terminal ID; `None` means no filter.
    pub term_id: Option<&'a str>,
    /// Output format sent to the portal (`"csv"`, `"xls"`, …).
    pub export_type: &'a str,
}

impl Default for ReportOptions<'_> {
    fn default() -> Self {
        Self {
            account_id: "",
            date_type: "D",
            use_date: "TR",
            merchant: None,
            term_id: None,
            export_type: "csv",
        }
    }
}
