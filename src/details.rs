use crate::client::fetch_html;
use crate::error::ScraperError;
use crate::models::{AppDetails, Category};
use crate::parser::{extract_init_data, get_json_val};
use serde_json::Value;

// Hàm phụ trợ gỡ tag HTML cơ bản
fn strip_html(html: &str) -> String {
    let mut text = html.replace("<br>", "\n").replace("<br/>", "\n");
    while let Some(start) = text.find('<') {
        if let Some(end) = text[start..].find('>') {
            text.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn find_array_by_string_id<'a>(root: &'a Value, target_id: &str) -> Option<&'a Value> {
    // Case 1: Data is in a vec of pairs or a regular subvec
    if let Some(arr) = root.as_array() {
        for child in arr {
            if let Some(child_arr) = child.as_array()
                && let Some(first_elem) = child_arr.first().and_then(|v| v.as_str())
                && first_elem == target_id
            {
                return Some(child);
            }
            // Recursively search deeper if necessary
            if let Some(found) = find_array_by_string_id(child, target_id) {
                return Some(found);
            }
        }
    }
    // In case The data is a JSON object (like the block structure {"141": [...], "146": [...]})
    else if let Some(obj) = root.as_object() {
        if let Some(val) = obj.get(target_id) {
            return Some(val);
        }
        for (_, val) in obj {
            if let Some(found) = find_array_by_string_id(val, target_id) {
                return Some(found);
            }
        }
    }
    None
}

/// Parses raw Google Play Store data (`ds5`) into a structured [`AppDetails`] object.
///
/// # Background
///
/// Google Play's app detail page embeds its data as a large, deeply nested,
/// unnamed JSON array (commonly referred to as `ds:5` in scraping tooling,
/// hence the parameter name). Because the array has no keys, every field is
/// extracted by walking a fixed sequence of indices (a "path") into that
/// structure via `get_json_val` (an internal helper in the `parser` module).
///
/// This function is the single source of truth for how each `AppDetails`
/// field maps to its corresponding path in the raw payload. If Google changes
/// the shape of this payload, this is the function that needs to be updated.
///
/// # Parameters
/// - `app_id`: The package name of the app (e.g. `"com.miHoYo.GenshinImpact"`).
///   Used to populate `app.app_id` and to build the canonical Play Store URL.
/// - `ds5`: The raw JSON value containing the scraped Play Store data blob.
///
/// # Returns
/// A fully populated [`AppDetails`] struct. Fields that cannot be found at
/// their expected path fall back to sensible defaults (empty string, `0`,
/// `false`, or `"Varies with device"` depending on the field), so this
/// function never fails — it degrades gracefully instead of returning
/// `Option`/`Result`.
///
/// # Extraction strategy
///
/// Internally, several local closures are used to reduce boilerplate when
/// reading from the nested JSON:
/// - `get_val`: raw [`Value`] lookup at a given index path.
/// - `get_str` / `get_f64` / `get_u64`: typed lookups with automatic
///   numeric coercion (Play Store sometimes encodes numbers as either
///   JSON integers or floats depending on the field).
/// - `is_truthy`: treats empty arrays/strings/objects and `0`/`false` as
///   "falsy", mirroring how Play Store encodes optional/boolean flags.
///
/// Most fields (title, description, install counts, ratings, pricing,
/// developer info, categories, media assets, content rating, etc.) are
/// read directly from fixed paths under `ds5[1][2][...]`.
///
/// # Fallback lookup ("index-independent" fields)
///
/// A subset of fields — `updated`, `version`, `android_version`,
/// `android_version_text`, `android_max_version`, and `recent_changes` — are
/// **not** guaranteed to live at a fixed index. Their position can shift
/// depending on the app (e.g. apps with additional metadata blocks, or
/// multi-APK / multi-device support). For these fields, the function first
/// tries the commonly-observed fixed path, and if that fails or returns a
/// known placeholder value (`"VARY"` / `"Varies with device"`), it falls
/// back to [`find_array_by_string_id`], which scans `ds5` for a sub-array
/// tagged with a known internal string ID (e.g. `"141"` for version info,
/// `"145"` for changelog, `"146"` for last-updated timestamp) and extracts
/// the value relative to that anchor instead of an absolute index.
///
/// This two-tier approach (fixed path first, ID-based scan as fallback)
/// keeps parsing fast for the common case while remaining resilient for
/// apps whose payload layout deviates from the norm.
///
/// # Notes / known limitations
/// - `editors_choice` is currently hardcoded to `false` — no reliable path
///   has been identified for this field yet.
/// - `developer_internal_id` is currently just a clone of `developer_id`;
///   if Play Store exposes a distinct internal developer ID at some path,
///   this should be updated.
/// - Timestamps (`app.updated`) are converted from seconds to milliseconds
///   (`ts * 1000`) to match the expected unit in [`AppDetails`].
pub fn map_app_details(app_id: &str, ds5: &Value) -> AppDetails {
    let mut app = AppDetails::default();
    app.app_id = app_id.to_string();
    app.url = format!(
        "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
        app_id
    );

    let get_val = |path: &[usize]| -> Option<&Value> { get_json_val(ds5, path) };

    let get_str = |path: &[usize]| -> Option<String> {
        get_val(path).and_then(|v| v.as_str()).map(String::from)
    };

    let get_f64 = |path: &[usize]| -> Option<f64> {
        get_val(path).and_then(|v| {
            if v.is_f64() {
                v.as_f64()
            } else if v.is_i64() {
                Some(v.as_i64().unwrap() as f64)
            } else {
                None
            }
        })
    };

    let get_u64 = |path: &[usize]| -> Option<u64> {
        get_val(path).and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
    };

    let is_truthy = |path: &[usize]| -> bool {
        match get_val(path) {
            Some(Value::Array(a)) => !a.is_empty(),
            Some(Value::String(s)) => !s.is_empty(),
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64() != Some(0.0),
            Some(Value::Object(o)) => !o.is_empty(),
            _ => false,
        }
    };

    app.title = get_str(&[1, 2, 0, 0]).unwrap_or_default();

    app.description_html = get_str(&[1, 2, 12, 0, 0, 1])
        .or_else(|| get_str(&[1, 2, 72, 0, 1]))
        .unwrap_or_default();
    app.description = strip_html(&app.description_html);

    app.summary = get_str(&[1, 2, 73, 0, 1]).unwrap_or_default();

    app.installs = get_str(&[1, 2, 13, 0]).unwrap_or_default();
    app.min_installs = get_u64(&[1, 2, 13, 1]).unwrap_or(0);
    app.max_installs = get_u64(&[1, 2, 13, 2]).unwrap_or(0);

    app.score = get_f64(&[1, 2, 51, 0, 1]).unwrap_or(0.0);
    app.score_text = get_str(&[1, 2, 51, 0, 0]).unwrap_or_default();
    app.ratings = get_u64(&[1, 2, 51, 2, 1]).unwrap_or(0);
    app.reviews = get_u64(&[1, 2, 51, 3, 1]).unwrap_or(0);

    if let Some(arr) = get_val(&[1, 2, 51, 1]).and_then(|v| v.as_array()) {
        for i in 1..=5 {
            if let Some(star_data) = arr.get(i)
                && let Some(count) = star_data.get(1).and_then(|c| c.as_u64())
            {
                app.histogram.insert(i.to_string(), count);
            }
        }
    }

    let price_micros = get_f64(&[1, 2, 57, 0, 0, 0, 0, 1, 0, 0]).unwrap_or(0.0);
    app.price = price_micros / 1_000_000.0;
    app.free = price_micros == 0.0;
    app.currency = get_str(&[1, 2, 57, 0, 0, 0, 0, 1, 0, 1]).unwrap_or_default();
    app.price_text =
        get_str(&[1, 2, 57, 0, 0, 0, 0, 1, 0, 2]).unwrap_or_else(|| "Free".to_string());

    app.offers_iap = get_val(&[1, 2, 19, 0]).is_some();
    app.iap_range = get_str(&[1, 2, 19, 0]);

    app.developer = get_str(&[1, 2, 68, 0]).unwrap_or_default();
    let dev_url = get_str(&[1, 2, 68, 1, 4, 2]).unwrap_or_default();
    app.developer_id = dev_url.split("id=").nth(1).unwrap_or("").to_string();
    app.developer_internal_id = app.developer_id.clone();

    app.developer_email = get_str(&[1, 2, 69, 1, 0]);
    app.developer_website = get_str(&[1, 2, 69, 0, 5, 2]);
    app.developer_address = get_str(&[1, 2, 69, 2, 0]);

    app.developer_legal_name = get_str(&[1, 2, 69, 4, 0]);
    app.developer_legal_email = get_str(&[1, 2, 69, 4, 1, 0]);
    app.developer_legal_address = get_str(&[1, 2, 69, 4, 2, 0]).map(|s| s.replace('\n', ", "));
    app.developer_legal_phone_number = get_str(&[1, 2, 69, 4, 3]);
    app.privacy_policy = get_str(&[1, 2, 99, 0, 5, 2]);

    app.genre = get_str(&[1, 2, 79, 0, 0, 0]).unwrap_or_default();
    app.genre_id = get_str(&[1, 2, 79, 0, 0, 2]).unwrap_or_default();

    if let Some(cat_arr) = get_val(&[1, 2, 118]).and_then(|v| v.as_array()) {
        for c in cat_arr {
            if let Some(c_arr) = c.as_array()
                && c_arr.len() >= 4
            {
                let name = c_arr[0].as_str().unwrap_or_default().to_string();
                let id = c_arr[2].as_str().map(|s| s.to_string());
                app.categories.push(Category { name, id });
            }
        }
    }
    if app.categories.is_empty() {
        app.categories.push(Category {
            name: app.genre.clone(),
            id: Some(app.genre_id.clone()),
        });
    }

    app.icon = get_str(&[1, 2, 95, 0, 3, 2]).unwrap_or_default();
    app.header_image = get_str(&[1, 2, 96, 0, 3, 2]);

    if let Some(screens) = get_val(&[1, 2, 78, 0]).and_then(|v| v.as_array()) {
        app.screenshots = screens
            .iter()
            .filter_map(|s| get_json_val(s, &[3, 2]))
            .filter_map(|url| url.as_str().map(String::from))
            .collect();
    }

    if let Some(video_url) = get_str(&[1, 2, 100, 0, 0, 3, 2]) {
        if video_url.contains("youtube.com/embed/") {
            app.video = Some(video_url);
        } else if let Some(id) = video_url
            .split("yt:movie:")
            .nth(1)
            .and_then(|s| s.split('?').next())
        {
            app.video = Some(format!(
                "https://www.youtube.com/embed/{}?vq=large&rel=0&autohide=1&showinfo=0",
                id
            ));
        } else {
            app.video = Some(video_url);
        }
    }

    app.video_image = get_str(&[1, 2, 100, 1, 0, 3, 2]);
    app.preview_video = get_str(&[1, 2, 100, 1, 2, 0, 2]);

    app.content_rating = get_str(&[1, 2, 9, 0]).unwrap_or_default();
    app.content_rating_description = get_str(&[1, 2, 9, 2, 1]);

    app.ad_supported = is_truthy(&[1, 2, 48]);
    app.released = get_str(&[1, 2, 10, 0]);

    // ==========================================
    // CƠ CHẾ DÒ TÌM RAMDA (-1 FALLBACK)
    // ==========================================
    let base_array = get_val(&[1, 2]);

    // 1. Updated Timestamp
    app.updated = find_array_by_string_id(ds5, "146")
        .and_then(|found| {
            found
                .get(0)
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.get(1))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_u64())
        })
        .map(|ts| ts * 1000)
        .unwrap_or(0);

    // 2. Version (Có tích hợp quét thông minh cho các app phức tạp)
    app.version = get_str(&[1, 2, 140, 0, 0, 0])
        // CHÌA KHÓA Ở ĐÂY: Nếu đọc ra VARY hoặc Varies with device -> Hủy kết quả, ép chạy xuống or_else
        .filter(|s| s != "VARY" && s != "Varies with device")
        .or_else(|| {
            find_array_by_string_id(ds5, "141").and_then(|found| {
                found
                    .get(0)
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
        })
        .unwrap_or_else(|| "Varies with device".to_string());

    // 3. Android Version
    app.android_version = get_str(&[1, 2, 140, 1, 1, 0, 0, 1])
        .or_else(|| {
            base_array
                .and_then(|arr| find_array_by_string_id(arr, "141"))
                .and_then(|found| get_json_val(found, &[1, 1, 0, 0, 1]))
                .and_then(|v| v.as_str().map(String::from))
        })
        .unwrap_or_else(|| "VARY".to_string());

    app.android_version_text = get_str(&[1, 2, 140, 1, 1, 0, 0, 1])
        .or_else(|| {
            base_array
                .and_then(|arr| find_array_by_string_id(arr, "141"))
                .and_then(|found| get_json_val(found, &[1, 1, 0, 0, 1]))
                .and_then(|v| v.as_str().map(String::from))
        })
        .unwrap_or_else(|| "Varies with device".to_string());

    app.android_max_version = get_str(&[1, 2, 140, 1, 1, 0, 1, 1]).or_else(|| {
        base_array
            .and_then(|arr| find_array_by_string_id(arr, "141"))
            .and_then(|found| get_json_val(found, &[1, 1, 0, 1, 1]))
            .and_then(|v| v.as_str().map(String::from))
    });

    // 4. Recent Changes
    app.recent_changes = find_array_by_string_id(ds5, "145").and_then(|found| {
        found
            .get(1)
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(1))
            .and_then(|v| v.as_str())
            .map(String::from)
    });
    // ==========================================

    app.preregister = get_u64(&[1, 2, 18, 0]) == Some(1);
    app.early_access_enabled = get_str(&[1, 2, 18, 2]).is_some();
    app.is_available_in_play_pass = is_truthy(&[1, 2, 62]);
    app.editors_choice = false;

    app
}

pub async fn get_app_details(app_id: &str) -> Result<AppDetails, ScraperError> {
    let url = format!(
        "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
        app_id
    );
    let html = fetch_html(&url).await?;

    let parsed_data = extract_init_data(&html);

    let ds5 = parsed_data
        .get("ds:5")
        .or_else(|| parsed_data.get("ds:4"))
        .ok_or(ScraperError::ParseError)?;

    let app = map_app_details(app_id, ds5);

    Ok(app)
}
