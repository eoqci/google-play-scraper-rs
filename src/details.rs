use crate::client::fetch_html;
use crate::error::ScraperError;
use crate::model::{AppDetails, Category, Feature};
use crate::parser::{extract_init_data, get_json_val};
use serde_json::Value;

// Hàm phụ trợ nhỏ để giả lập `cheerio.text()` - gỡ tag HTML cơ bản
fn strip_html(html: &str) -> String {
    let mut text = html.replace("<br>", "\n").replace("<br/>", "\n");
    // Xóa tất cả các thẻ <...>
    while let Some(start) = text.find('<') {
        if let Some(end) = text[start..].find('>') {
            text.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }
    // Decode vài HTML entity cơ bản
    text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    text
}

pub fn map_app_details(app_id: &str, ds5: &Value) -> AppDetails {
    let mut app = AppDetails::default();
    app.app_id = app_id.to_string();
    app.url = format!(
        "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
        app_id
    );

    // ==========================================
    // CÁC CLOSURES RÚT GỌN THAO TÁC LẤY DỮ LIỆU
    // ==========================================
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

    // ==========================================
    // BẮT ĐẦU MAPPING DỮ LIỆU
    // ==========================================

    app.title = get_str(&[1, 2, 0, 0]).unwrap_or_default();

    // Description: Ưu tiên bản dịch (translation), nếu không có dùng bản gốc (original)
    app.description_html = get_str(&[1, 2, 12, 0, 0, 1]) // localized
        .or_else(|| get_str(&[1, 2, 72, 0, 1])) // original
        .unwrap_or_default();
    app.description = strip_html(&app.description_html);

    app.summary = get_str(&[1, 2, 73, 0, 1]).unwrap_or_default();

    // Lượt cài đặt
    app.installs = get_str(&[1, 2, 13, 0]).unwrap_or_default();
    app.min_installs = get_u64(&[1, 2, 13, 1]).unwrap_or(0);
    app.max_installs = get_u64(&[1, 2, 13, 2]).unwrap_or(0);

    // Điểm đánh giá (Score & Ratings)
    app.score = get_f64(&[1, 2, 51, 0, 1]).unwrap_or(0.0);
    app.score_text = get_str(&[1, 2, 51, 0, 0]).unwrap_or_default();
    app.ratings = get_u64(&[1, 2, 51, 2, 1]).unwrap_or(0);
    app.reviews = get_u64(&[1, 2, 51, 3, 1]).unwrap_or(0);

    // Histogram (1 sao đến 5 sao)
    if let Some(arr) = get_val(&[1, 2, 51, 1]).and_then(|v| v.as_array()) {
        for i in 1..=5 {
            if let Some(star_data) = arr.get(i) {
                // Index 1, 2, 3, 4, 5
                if let Some(count) = star_data.get(1).and_then(|c| c.as_u64()) {
                    app.histogram.insert(i.to_string(), count);
                }
            }
        }
    }

    // Giá & Tiền tệ (Google lưu giá vi mô: x 1,000,000)
    let price_micros = get_f64(&[1, 2, 57, 0, 0, 0, 0, 1, 0, 0]).unwrap_or(0.0);
    app.price = price_micros / 1_000_000.0;
    app.free = price_micros == 0.0;
    app.currency = get_str(&[1, 2, 57, 0, 0, 0, 0, 1, 0, 1]).unwrap_or_default();
    app.price_text =
        get_str(&[1, 2, 57, 0, 0, 0, 0, 1, 0, 2]).unwrap_or_else(|| "Free".to_string());

    // In-App Purchases (IAP)
    app.offers_iap = get_val(&[1, 2, 19, 0]).is_some();
    app.iap_range = get_str(&[1, 2, 19, 0]);

    // Phiên bản Android
    app.android_version = get_str(&[1, 2, 140, 1, 1, 0, 0, 1])
        .or_else(|| get_str(&[1, 2, 141, 1, 1, 0, 0, 1]))
        .unwrap_or_else(|| "VARY".to_string());

    app.android_version_text = get_str(&[1, 2, 140, 1, 1, 0, 0, 1])
        .or_else(|| get_str(&[1, 2, 141, 1, 1, 0, 0, 1]))
        .unwrap_or_else(|| "Varies with device".to_string());

    app.android_max_version =
        get_str(&[1, 2, 140, 1, 1, 0, 1, 1]).or_else(|| get_str(&[1, 2, 141, 1, 1, 0, 1, 1]));

    // Thông tin Developer
    app.developer = get_str(&[1, 2, 68, 0]).unwrap_or_default();

    let dev_url = get_str(&[1, 2, 68, 1, 4, 2]).unwrap_or_default();
    app.developer_id = dev_url.split("id=").nth(1).unwrap_or("").to_string();
    app.developer_internal_id = app.developer_id.clone();

    app.developer_email = get_str(&[1, 2, 69, 1, 0]);
    app.developer_website = get_str(&[1, 2, 69, 0, 5, 2]);
    app.developer_address = get_str(&[1, 2, 69, 2, 0]);

    // Legal Developer Info (Tên công ty đóng thuế...)
    app.developer_legal_name = get_str(&[1, 2, 69, 4, 0]);
    app.developer_legal_email = get_str(&[1, 2, 69, 4, 1, 0]);
    app.developer_legal_address = get_str(&[1, 2, 69, 4, 2, 0]).map(|s| s.replace('\n', ", "));
    app.developer_legal_phone_number = get_str(&[1, 2, 69, 4, 3]);

    app.privacy_policy = get_str(&[1, 2, 99, 0, 5, 2]);

    // Thể loại (Genre & Category)
    app.genre = get_str(&[1, 2, 79, 0, 0, 0]).unwrap_or_default();
    app.genre_id = get_str(&[1, 2, 79, 0, 0, 2]).unwrap_or_default();

    // Mảng Categories: ['ds:5', 1, 2, 118]
    if let Some(cat_arr) = get_val(&[1, 2, 118]).and_then(|v| v.as_array()) {
        for c in cat_arr {
            if let Some(c_arr) = c.as_array() {
                if c_arr.len() >= 4 {
                    // Structure theo hàm extractCategories
                    let name = c_arr[0].as_str().unwrap_or_default().to_string();
                    let id = c_arr[2].as_str().map(|s| s.to_string());
                    app.categories.push(Category { name, id });
                }
            }
        }
    }
    // Fallback nếu không có categories
    if app.categories.is_empty() {
        app.categories.push(Category {
            name: app.genre.clone(),
            id: Some(app.genre_id.clone()),
        });
    }

    // Hình ảnh & Media
    app.icon = get_str(&[1, 2, 95, 0, 3, 2]).unwrap_or_default();
    app.header_image = get_str(&[1, 2, 96, 0, 3, 2]);

    // Screenshots array
    if let Some(screens) = get_val(&[1, 2, 78, 0]).and_then(|v| v.as_array()) {
        app.screenshots = screens
            .iter()
            .filter_map(|s| get_json_val(s, &[3, 2])) // Map tới path [3, 2]
            .filter_map(|url| url.as_str().map(String::from))
            .collect();
    }

    app.video = get_str(&[1, 2, 100, 0, 0, 3, 2]);
    app.video_image = get_str(&[1, 2, 100, 1, 0, 3, 2]);
    app.preview_video = get_str(&[1, 2, 100, 1, 2, 0, 2]);

    // Các thông tin khác
    app.content_rating = get_str(&[1, 2, 9, 0]).unwrap_or_default();
    app.content_rating_description = get_str(&[1, 2, 9, 2, 1]);

    app.ad_supported = get_val(&[1, 2, 48]).is_some();
    app.released = get_str(&[1, 2, 10, 0]);

    // Updated Timestamp (đôi khi đổi từ 145 sang 146)
    if let Some(ts) = get_u64(&[1, 2, 145, 0, 1, 0]).or_else(|| get_u64(&[1, 2, 146, 0, 1, 0])) {
        app.updated = ts * 1000;
    }

    app.version = get_str(&[1, 2, 140, 0, 0, 0])
        .or_else(|| get_str(&[1, 2, 141, 0, 0, 0]))
        .unwrap_or_else(|| "VARY".to_string());

    app.recent_changes = get_str(&[1, 2, 144, 1, 1]).or_else(|| get_str(&[1, 2, 145, 1, 1]));

    app.preregister = get_u64(&[1, 2, 18, 0]) == Some(1);
    app.early_access_enabled = get_str(&[1, 2, 18, 2]).is_some(); // String if early access
    app.is_available_in_play_pass = get_val(&[1, 2, 62]).is_some();
    app.editors_choice = false; // Thuộc tính này thường được map riêng qua logo, tạm để false

    app
}

pub async fn get_app_details(app_id: &str) -> Result<AppDetails, ScraperError> {
    let url = format!(
        "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
        app_id
    );
    let html = fetch_html(&url).await?;

    let parsed_data = extract_init_data(&html);

    // Dữ liệu chi tiết nằm trong khối ds:5 (Đôi khi Google chuyển qua ds:4)
    let ds5 = parsed_data
        .get("ds:5")
        .or_else(|| parsed_data.get("ds:4"))
        .ok_or(ScraperError::ParseError)?;

    let app = map_app_details(app_id, ds5);

    Ok(app)
}
