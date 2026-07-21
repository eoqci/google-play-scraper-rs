use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

use crate::{client::fetch_html, error::ScraperError};

// Hàm hỗ trợ điều hướng trong mảng đa chiều bằng index
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

pub fn extract_init_data(html: &str) -> HashMap<String, Value> {
    let mut parsed_data = HashMap::new();

    // Regex bắt key và data
    let re_main = Regex::new(
        r"AF_initDataCallback\s*\(.*?key:\s*'([^']+)'\s*,.*?data:\s*(\[.*?\])\s*}\s*\);",
    )
    .unwrap();

    // Regex để dọn dẹp JSON: bắt các dấu phẩy trống có chứa khoảng trắng
    // Ví dụ: `[  ,` hoặc `,  ,`
    let re_empty_array_start = Regex::new(r"\[\s*,").unwrap();
    let re_empty_elements = Regex::new(r",\s*,").unwrap();

    for cap in re_main.captures_iter(html) {
        if let (Some(key_match), Some(data_match)) = (cap.get(1), cap.get(2)) {
            let key = key_match.as_str().to_string();
            let mut json_str = data_match.as_str().to_string();

            // 1. Dọn dẹp khoảng trống ngay sau dấu ngoặc vuông mở: `[ ,` -> `[null,`
            json_str = re_empty_array_start
                .replace_all(&json_str, "[null,")
                .to_string();

            // 2. Dọn dẹp các dấu phẩy liền kề nhau: `, ,` -> `,null,`
            // Phải chạy vòng lặp vì Regex có thể bị lướt qua các chuỗi như `,,,` do overlapping
            while re_empty_elements.is_match(&json_str) {
                json_str = re_empty_elements
                    .replace_all(&json_str, ",null,")
                    .to_string();
            }

            match serde_json::from_str(&json_str) {
                Ok(value) => {
                    parsed_data.insert(key, value);
                }
                Err(e) => {
                    eprintln!("Lỗi parse JSON cho key {}: {}", key, e);
                    // Bật dòng này lên để debug chuỗi JSON thực tế nếu vẫn còn lỗi
                    // eprintln!("Chuỗi lỗi: {}", json_str);
                }
            }
        }
    }

    parsed_data
}

// Hàm này chỉ dùng để debug xem lấy được cục data nào
pub async fn debug_raw_app_data(app_id: &str) -> Result<HashMap<String, Value>, ScraperError> {
    let url = format!(
        "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
        app_id
    );
    println!("Fetching from: {}", url);
    let html = fetch_html(&url).await?;

    let parsed_data = extract_init_data(&html);
    Ok(parsed_data) // Bây giờ thì HashMap khớp với khai báo hàm
}

pub fn parse_batchexecute_response(raw_response: &str) -> Option<Value> {
    // 1. Cắt bỏ tiền tố rác )]}' hoặc khoảng trắng thừa ở đầu
    let clean_str = if raw_response.starts_with(")]}'") {
        raw_response[4..].trim()
    } else {
        raw_response.trim()
    };

    // 2. Parse vỏ ngoài
    let outer_json: Value = serde_json::from_str(clean_str).ok()?;

    // 3. Lấy chuỗi JSON bị lồng bên trong (nằm ở input[0][2])
    let inner_str = outer_json
        .as_array()?
        .get(0)?
        .as_array()?
        .get(2)?
        .as_str()?;

    // 4. Parse ruột bên trong
    serde_json::from_str(inner_str).ok()
}
