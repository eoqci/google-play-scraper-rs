//! Error types returned by this crate's public functions.

use thiserror::Error;

/// The error type returned by all fallible operations in this crate.
///
/// This covers both transport-level failures (network requests) and
/// scraping-level failures (the Play Store's HTML/JSON structure not
/// matching what the parser expects).
#[derive(Debug, Error)]
pub enum ScraperError {
    /// The underlying HTTP request failed — connection error, timeout,
    /// TLS failure, invalid response encoding, etc. Wraps the original
    /// [`reqwest::Error`] for inspection via `source()`.
    #[error("HTTP request failed: {0}")]
    Network(#[from] reqwest::Error),

    /// The response was received successfully, but its HTML or JSON
    /// structure did not match what the parser expected (e.g.
    /// [`crate::parser::extract_init_data`] found no matching
    /// `AF_initDataCallback` block, or
    /// [`crate::parser::parse_batchexecute_response`] could not unwrap
    /// the nested payload). This usually means Google has changed the
    /// page/response format since this crate was last updated, rather
    /// than an issue with the request itself.
    #[error("Failed to parse HTML structure")]
    ParseError,

    /// The requested app could not be found. Returned when the Play
    /// Store responds with `404` for [`crate::details::get_app_details`],
    /// or with a non-success status for a reviews request (see
    /// [`crate::client::fetch_batchexecute`], which currently maps any
    /// non-2xx response to this variant — so in the reviews case this can
    /// also indicate a rejected/rate-limited request rather than strictly
    /// "not found").
    #[error("App not found on Play Store")]
    NotFound,
}
