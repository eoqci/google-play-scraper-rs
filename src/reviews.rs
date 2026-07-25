//! Fetches and parses user reviews for a Google Play Store app.
//!
//! Reviews are not exposed through a stable, documented endpoint. Instead,
//! this module talks to Google's internal `batchexecute` RPC mechanism
//! (the same one the Play Store web client uses), which returns a large,
//! deeply nested, unlabeled JSON array. Because the exact shape of that
//! array can shift slightly between requests or Play Store versions, this
//! module locates the data it needs by *pattern* (structural heuristics)
//! rather than by a single fixed index path — see [`find_reviews_list`]
//! and [`find_pagination_token`] below.

use crate::client::fetch_batchexecute;
use crate::error::ScraperError;
use crate::models::{Review, ReviewsResult, SortType};
use crate::parser::parse_batchexecute_response;
use serde_json::Value;

/// Builds the URL-encoded `f.req` body for the `UsvDTd` batchexecute RPC,
/// which is the internal call the Play Store web UI uses to fetch reviews.
///
/// # Parameters
/// - `app_id`: package name of the target app (e.g. `"com.shopee.vn"`).
/// - `sort`: numeric sort order expected by the RPC (see [`SortType`] for
///   the mapping — e.g. `2` = newest).
/// - `count`: number of reviews to request per page.
/// - `token`: pagination token from a previous [`ReviewsResult`], or `None`
///   to fetch the first page.
///
/// # Format notes
/// The request payload is itself a JSON array encoded as a string and then
/// URL-encoded, following the `batchexecute` protocol's convention of
/// nesting a JSON-in-JSON payload inside the `f.req` form field. The
/// pagination token, when present, must be embedded as an escaped JSON
/// string (hence the manual `\"..\"` escaping) since it lives inside the
/// already-stringified inner request.
fn build_reviews_body(app_id: &str, sort: i32, count: usize, token: Option<&str>) -> String {
    let token_str = match token {
        Some(t) => format!("\\\"{}\\\"", t),
        None => "null".to_string(),
    };

    let inner_req = format!(
        "[null,null,[2,{},[{},null,{}],null,[]],[\"{}\",7]]",
        sort, count, token_str, app_id
    );

    let req_value = format!("[[[\"UsvDTd\",\"{}\",null,\"generic\"]]]", inner_req);
    format!("f.req={}", urlencoding::encode(&req_value))
}

/// Recursively scans a parsed batchexecute response to locate the array
/// that holds the list of individual reviews.
///
/// # Why a scan instead of a fixed path
/// The review list's position within the overall response can vary, so
/// instead of indexing to a known offset, this function walks the tree
/// depth-first and identifies the reviews array **structurally**: a
/// candidate array qualifies if its first element is itself an array whose
/// first entry is a string that looks like a review ID (a UUID — at least
/// 32 characters long and containing a hyphen). This heuristic is specific
/// enough in practice to avoid false positives elsewhere in the response.
///
/// Returns `None` if no array matching this shape is found anywhere in the
/// tree.
fn find_reviews_list(root: &Value) -> Option<&Vec<Value>> {
    if let Some(arr) = root.as_array() {
        if !arr.is_empty() {
            // A reviews array is recognized by its first entry: itself an
            // array whose first element is a UUID-shaped review ID.

            // if let Some(first_item) = arr[0].as_array() {
            //     if let Some(id_str) = first_item.get(0).and_then(|v| v.as_str()) {
            //         if id_str.len() >= 32 && id_str.contains('-') {
            //             return Some(arr);
            //         }
            //     }
            // }

            if let Some(first_item) = arr[0].as_array()
                && let Some(id_str) = first_item.first().and_then(|v| v.as_str())
            {
                if id_str.len() >= 32 && id_str.contains('-') {
                    return Some(arr);
                }
            }
        }
        // Not a match at this level — recurse into children.
        for child in arr {
            if let Some(found) = find_reviews_list(child) {
                return Some(found);
            }
        }
    }
    None
}

/// Recursively scans a parsed batchexecute response to locate the
/// pagination token for the next page of reviews.
///
/// # Heuristic
/// The token is identified structurally rather than by fixed index: it
/// looks for a two-element array of the shape `[null, "<token>"]` where the
/// string is longer than 40 characters and contains no spaces (pagination
/// tokens are long, opaque, whitespace-free strings; this rules out
/// unrelated two-element `[null, string]` pairs elsewhere in the payload).
///
/// Returns `None` if the response represents the last page (no more
/// reviews available) or if no matching shape is found.
fn find_pagination_token(root: &Value) -> Option<String> {
    if let Some(arr) = root.as_array() {
        if arr.len() == 2 && arr[0].is_null() && arr[1].is_string() {
            let s = arr[1].as_str().unwrap();
            if s.len() > 40 && !s.contains(' ') {
                return Some(s.to_string());
            }
        }
        for child in arr {
            if let Some(token) = find_pagination_token(child) {
                return Some(token);
            }
        }
    }
    None
}

/// Fetches a page of user reviews for the given app from the Google Play
/// Store.
///
/// # Parameters
/// - `app_id`: the app's package name (e.g. `"com.miHoYo.GenshinImpact"`).
/// - `sort`: review sort order — see [`SortType`].
/// - `count`: maximum number of reviews to fetch in this page.
/// - `pagination_token`: pass `None` for the first page. To fetch
///   subsequent pages, pass the `next_pagination_token` returned in the
///   previous [`ReviewsResult`].
///
/// # Returns
/// A [`ReviewsResult`] containing the parsed reviews (`data`) and, if more
/// reviews are available, a `next_pagination_token` to pass into a
/// follow-up call. If the underlying response contains no reviews array,
/// `data` will simply be empty rather than an error.
///
/// # Errors
/// Returns [`ScraperError`] if the network request fails or if the
/// response cannot be parsed as a valid batchexecute payload at all
/// (`ScraperError::ParseError`). Missing individual fields within a
/// well-formed response are treated as absent/default values rather than
/// hard errors (see field-level notes below).
///
/// # Field mapping
/// Each review is a fixed-position array within the reviews list. The
/// following indices are currently known and extracted:
///
/// | Index | Field                          |
/// |-------|--------------------------------|
/// | 0     | Review ID (UUID)                |
/// | 1[0]  | User name                       |
/// | 1[1][3][2] | User avatar image URL      |
/// | 2     | Star rating (score)             |
/// | 4     | Review text                     |
/// | 5[0]  | Review date (Unix timestamp)     |
/// | 6     | Thumbs-up (helpful) count        |
/// | 7[1]  | Developer reply text (if any)    |
/// | 7[2][0] | Developer reply date (if any)  |
/// | 10    | App version the review was left on |
///
/// Any field that is missing or of an unexpected type falls back to a
/// default (empty string / `0` / `None`) rather than failing the whole
/// request, since Google does not guarantee every review carries every
/// field (e.g. a review without a developer reply naturally has no data at
/// index 7).
pub async fn get_reviews(
    app_id: &str,
    sort: SortType,
    count: usize,
    pagination_token: Option<&str>,
) -> Result<ReviewsResult, ScraperError> {
    let url = "https://play.google.com/_/PlayStoreUi/data/batchexecute?rpcids=UsvDTd&f.sid=-8958226065532581605&bl=boq_playuiserver_20240103.04_p0&hl=en&gl=us&authuser&soc-app=121&soc-platform=1&soc-device=1&_reqid=132338";

    let body = build_reviews_body(app_id, sort as i32, count, pagination_token);

    let raw_response = fetch_batchexecute(url, &body).await?;
    let parsed_json = parse_batchexecute_response(&raw_response).ok_or(ScraperError::ParseError)?;

    let mut result = ReviewsResult {
        data: vec![],
        next_pagination_token: find_pagination_token(&parsed_json),
    };

    if let Some(reviews_array) = find_reviews_list(&parsed_json) {
        for rev_val in reviews_array {
            if let Some(arr) = rev_val.as_array() {
                let id = arr
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let user_name = arr
                    .get(1)
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let user_image = arr
                    .get(1)
                    .and_then(|v| v.get(1))
                    .and_then(|v| v.get(3))
                    .and_then(|v| v.get(2))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let score = arr.get(2).and_then(|v| v.as_u64()).unwrap_or(0);
                let text = arr
                    .get(4)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let date = arr
                    .get(5)
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string();
                let thumbs_up = arr.get(6).and_then(|v| v.as_u64()).unwrap_or(0);

                let reply_text = arr
                    .get(7)
                    .and_then(|v| v.get(1))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let reply_date = arr
                    .get(7)
                    .and_then(|v| v.get(2))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_u64())
                    .map(|ts| ts.to_string());

                // Version the review was left on.
                let version = arr.get(10).and_then(|v| v.as_str()).map(String::from);

                let review = Review {
                    id: id.clone(),
                    url: format!(
                        "https://play.google.com/store/apps/details?id={}&reviewId={}",
                        app_id, id
                    ),
                    user_name,
                    user_image,
                    date,
                    score,
                    text,
                    reply_date,
                    reply_text,
                    version,
                    thumbs_up,
                };
                result.data.push(review);
            }
        }
    }

    Ok(result)
}
