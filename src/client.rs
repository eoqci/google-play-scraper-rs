//! Low-level HTTP client functions used to talk to the Google Play Store.
//!
//! Google Play does not offer a public API for the data this crate needs
//! (app details, reviews), so requests here mimic what a real browser
//! sends when loading the Play Store web app. This means setting the same
//! `User-Agent` a browser would use and, where required, maintaining
//! cookies across requests. See each function's docs for specifics.

use crate::error::ScraperError;
use reqwest::{Client, header};

/// Fetches the raw HTML of a Google Play Store page (e.g. an app's details
/// page) at the given URL.
///
/// # Why a custom `User-Agent`
/// Play Store pages serve different markup (or block the request outright)
/// depending on the client's `User-Agent`. A desktop Chrome UA string is
/// set here so the response matches what this crate's parsers (e.g.
/// [`crate::parser::extract_init_data`]) expect. `Accept-Language` is
/// pinned to English so that scraped fields such as category names and
/// content ratings come back in a consistent, predictable language rather
/// than varying by the server's locale detection.
///
/// # Parameters
/// - `url`: the full Play Store page URL to fetch (e.g. an app's details
///   page URL as built by [`crate::details::map_app_details`]).
///
/// # Returns
/// The page's raw HTML body as a `String`.
///
/// # Errors
/// - Returns [`ScraperError::NotFound`] if the server responds with
///   `404` — this is the expected outcome for an invalid or delisted
///   `app_id`, so callers can match on it specifically rather than
///   treating it as a generic failure.
/// - Returns other [`ScraperError`] variants (via `?`/`From` conversions)
///   for connection failures, non-UTF8 bodies, or other `reqwest` errors.
///
/// # Notes
/// This function does not retry or follow any custom redirect logic beyond
/// `reqwest`'s defaults, and does not use a cookie store, since a single
/// page load does not require session continuity.
pub async fn fetch_html(url: &str) -> Result<String, ScraperError> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        header::HeaderValue::from_static("en-US;q=0.9,en;q=0.8"),
    );
    let client = Client::builder().default_headers(headers).build()?;
    let response = client.get(url).send().await?;
    if response.status() == 404 {
        return Err(ScraperError::NotFound);
    }
    Ok(response.text().await?)
}

/// Sends a request to Google's internal `batchexecute` RPC endpoint (used
/// for reviews and other non-page-load data) and returns the raw response
/// body.
///
/// # Parameters
/// - `url`: the full `batchexecute` endpoint URL, including its query
///   string (RPC id, session id, build label, etc. — see
///   [`crate::reviews::get_reviews`] for how this URL is constructed).
/// - `form_body`: the URL-encoded `f.req=...` request body, as built by
///   [`crate::reviews::build_reviews_body`].
///
/// # Why a cookie store is enabled here (unlike [`fetch_html`])
/// Unlike a plain page load, `batchexecute` calls are treated by Google as
/// part of an authenticated-ish session tied to cookies issued on first
/// contact. Enabling `cookie_store(true)` lets `reqwest` automatically
/// capture and resend any cookies the endpoint sets, which some
/// `batchexecute` calls expect to see echoed back to behave consistently
/// (e.g. across paginated review requests within the same client
/// instance).
///
/// # Returns
/// The raw response body as a `String`. This is still in
/// `batchexecute`'s wrapped/escaped format at this point — see
/// [`crate::parser::parse_batchexecute_response`] for how it's unwrapped
/// into usable JSON.
///
/// # Errors
/// - Returns [`ScraperError::NotFound`] if the response status is not a
///   success code. Note this is a coarser check than [`fetch_html`]'s
///   (any non-2xx here maps to `NotFound`, not just `404`); callers should
///   keep this in mind if they need to distinguish other failure modes
///   from `batchexecute` specifically.
/// - Returns other [`ScraperError`] variants for connection-level
///   failures.
///
/// # Notes
/// Each call builds a fresh `Client` (and therefore a fresh, empty cookie
/// jar) rather than reusing one across calls. This means the cookie store
/// only helps within the lifetime of a single request/response exchange —
/// it does not currently persist cookies across separate calls to this
/// function (e.g. across paginated calls to [`crate::reviews::get_reviews`]).
/// If session continuity across pages turns out to matter, the `Client`
/// would need to be constructed once and reused by the caller instead.
pub async fn fetch_batchexecute(url: &str, form_body: &str) -> Result<String, ScraperError> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        ),
    );
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/x-www-form-urlencoded;charset=UTF-8"),
    );
    let client = Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .build()?;
    let response = client.post(url).body(form_body.to_string()).send().await?;
    if !response.status().is_success() {
        return Err(ScraperError::NotFound);
    }
    Ok(response.text().await?)
}
