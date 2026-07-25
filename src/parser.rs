use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;

/// Walks a [`Value`] by following a sequence of array indices and returns
/// a reference to the value found at that path.
///
/// This is the low-level primitive used throughout the crate to read
/// fields out of Google Play's unlabeled, deeply nested JSON arrays (see
/// [`crate::details::map_app_details`] for typical usage), since those
/// arrays have no keys — every field is addressed purely by position.
///
/// # Parameters
/// - `root`: the JSON value to start from.
/// - `path`: a sequence of indices to follow, one per nesting level. An
///   empty path returns `root` itself.
///
/// # Returns
/// `Some(&Value)` if every index in `path` resolves successfully (i.e. each
/// intermediate value is an array/object containing that index), or `None`
/// as soon as any index is out of bounds or the current value doesn't
/// support indexing.
///
/// # Example
/// ```ignore
/// // Equivalent to root[1][2][0]
/// let title = get_json_val(&root, &[1, 2, 0]);
/// ```
pub fn get_json_val<'a>(root: &'a Value, path: &[usize]) -> Option<&'a Value> {
    let mut current = root;
    for &idx in path {
        if let Some(next) = current.get(idx) {
            current = next;
        } else {
            return None;
        }
    }
    Some(current)
}

/// Extracts all `AF_initDataCallback` payloads embedded in a Google Play
/// Store HTML page.
///
/// # Background
/// Play Store pages embed their initial state as multiple inline
/// `<script>` blocks that call `AF_initDataCallback({key: 'ds:N', data:
/// [...], ...})`. Each block corresponds to a different data slice (e.g.
/// `ds:5` typically holds the main app details payload). This function
/// scans every `<script>` tag on the page, and for each one that contains
/// an `AF_initDataCallback` call, extracts:
/// 1. The slice key (`ds:N`), via `key_regex`.
/// 2. The raw JSON array assigned to `data`, via `data_regex`.
///
/// # Why the JSON needs cleaning
/// The `data` array as written in the page source is **not valid JSON on
/// its own** — Google's JS source uses sparse array syntax (e.g.
/// `[1, , , 4]`) to represent omitted elements, which `serde_json` cannot
/// parse directly. Before parsing, this function rewrites those gaps into
/// explicit `null`s:
/// - `re_empty_array_start`: fixes a leading gap right after `[` (e.g.
///   `[, 2]` → `[null, 2]`).
/// - `re_empty_elements`: repeatedly fixes internal gaps (e.g. `1, , 3` →
///   `1, null, 3`), looping until no more consecutive commas remain, since
///   a single pass only fixes non-overlapping matches.
///
/// After cleanup, the function also trims trailing garbage that the greedy
/// `data_regex` may have over-captured, by truncating at the last `];`
/// (or, failing that, the last standalone `]`) so that only the intended
/// array remains.
///
/// # Returns
/// A map from slice key (e.g. `"ds:5"`) to its parsed [`Value`]. Script
/// blocks that don't match the expected shape (missing key, malformed
/// data, or invalid JSON even after cleanup) are silently skipped rather
/// than causing the whole function to fail — the goal is to recover as
/// many usable slices as possible from a single page.
///
/// # Notes
/// - This function assumes callers already know which `ds:N` key holds the
///   data they want (e.g. app details typically live under `ds:5`); it
///   does not interpret the semantics of any slice itself.
/// - Because it relies on regex rather than a JS parser, it is inherently
///   best-effort: if Google changes the surrounding script formatting,
///   these patterns may need to be updated.
pub fn extract_init_data(html: &str) -> HashMap<String, Value> {
    let mut parsed_data = HashMap::new();
    let document = Html::parse_document(html);
    let script_selector = Selector::parse("script").unwrap();
    let key_regex = Regex::new(r#"key\s*:\s*['"](ds:\d+)['"]"#).unwrap();
    // Broadened to allow matching all the way through Google's large JSON array.
    let data_regex =
        Regex::new(r#"data\s*:\s*(\[[\s\S]*\])\s*,\s*(?:sideChannel|\w+\s*:\s*function|$)?"#)
            .unwrap();
    let re_empty_array_start = Regex::new(r"\[\s*,").unwrap();
    let re_empty_elements = Regex::new(r",\s*,").unwrap();
    for element in document.select(&script_selector) {
        let script_text = element.inner_html();
        if script_text.contains("AF_initDataCallback") {
            if let Some(key_cap) = key_regex.captures(&script_text) {
                // Attempt to capture the JSON array from the script tag.
                if let Some(data_cap) = data_regex.captures(&script_text) {
                    let key = key_cap.get(1).unwrap().as_str().to_string();
                    let mut json_str = data_cap.get(1).unwrap().as_str().to_string();
                    // Clean up Google's malformed sparse-array JSON.
                    json_str = re_empty_array_start
                        .replace_all(&json_str, "[null,")
                        .to_string();
                    while re_empty_elements.is_match(&json_str) {
                        json_str = re_empty_elements
                            .replace_all(&json_str, ",null,")
                            .to_string();
                    }
                    // Trim trailing garbage in case the regex over-matched.
                    if let Some(idx) = json_str.rfind("];") {
                        json_str.truncate(idx + 1);
                    } else if let Some(_idx) = json_str.rfind("}]") {
                        // Find the last closing square bracket.
                        if let Some(bracket_idx) = json_str.rfind(']') {
                            json_str.truncate(bracket_idx + 1);
                        }
                    }
                    if let Ok(value) = serde_json::from_str(&json_str) {
                        parsed_data.insert(key, value);
                    }
                }
            }
        }
    }
    parsed_data
}

/// Parses a raw response body from Google's internal `batchexecute` RPC
/// endpoint (used for reviews and other non-page-load requests) into a
/// [`Value`].
///
/// # Background
/// `batchexecute` responses come in two layers that both need to be
/// unwrapped before reaching the actual data:
///
/// 1. **XSSI protection prefix**: Google prefixes the body with `)]}'` to
///    prevent the response from being directly executable as a script if
///    fetched via a `<script>` tag. This prefix is stripped before parsing.
/// 2. **Double-encoded payload**: the outer JSON is itself an envelope
///    (chunked response format) whose actual payload is embedded as a
///    *JSON-encoded string* somewhere inside one of its arrays, rather than
///    as native nested JSON. [`find_nested_json_string`] recursively walks
///    the outer structure looking for a string value that looks like a
///    serialized JSON array (starts with `[` and is reasonably long, to
///    avoid false positives on short unrelated strings), and that string
///    is then parsed a second time to get the real data.
///
/// # Returns
/// `Some(Value)` with the fully unwrapped, parsed payload, or `None` if
/// either parsing step fails — e.g. the outer body isn't valid JSON after
/// stripping the prefix, or no nested JSON-like string can be found inside
/// it.
///
/// # Notes
/// - The heuristic in `find_nested_json_string` (`starts_with('[')` and
///   `len() > 100`) is intentionally loose; it works because in practice
///   the actual payload string is the only long, array-shaped string in
///   the envelope. If Google's response format changes, this threshold may
///   need adjusting.
/// - This function only performs the two unwrapping steps — it does not
///   know anything about the semantics of the resulting data (e.g. review
///   fields). See [`crate::reviews::get_reviews`] for how the result is
///   interpreted.
pub fn parse_batchexecute_response(raw_response: &str) -> Option<Value> {
    let clean_str = if raw_response.starts_with(")]}'") {
        raw_response[4..].trim()
    } else {
        raw_response.trim()
    };
    let outer_json: Value = serde_json::from_str(clean_str).ok()?;

    /// Recursively searches for a string value that looks like a
    /// serialized nested JSON array — the hallmark of batchexecute's
    /// double-encoded response payloads.
    fn find_nested_json_string(root: &Value) -> Option<String> {
        if let Some(arr) = root.as_array() {
            for item in arr {
                if let Some(s) = item.as_str() {
                    // A nested JSON string typically starts with '[' and is long.
                    if s.starts_with('[') && s.len() > 100 {
                        return Some(s.to_string());
                    }
                }
                if let Some(found) = find_nested_json_string(item) {
                    return Some(found);
                }
            }
        }
        None
    }

    if let Some(inner_str) = find_nested_json_string(&outer_json) {
        return serde_json::from_str(&inner_str).ok();
    }
    None
}
