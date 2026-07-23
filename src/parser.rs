// File: src/parser.rs
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;

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

    let document = Html::parse_document(html);
    let script_selector = Selector::parse("script").unwrap();

    let key_regex = Regex::new(r#"key\s*:\s*['"](ds:\d+)['"]"#).unwrap();

    // Mở rộng bộ lọc data_regex: Cho phép quét đến hết mảng JSON lớn của Google
    let data_regex =
        Regex::new(r#"data\s*:\s*(\[[\s\S]*\])\s*,\s*(?:sideChannel|\w+\s*:\s*function|$)?"#)
            .unwrap();

    let re_empty_array_start = Regex::new(r"\[\s*,").unwrap();
    let re_empty_elements = Regex::new(r",\s*,").unwrap();

    for element in document.select(&script_selector) {
        let script_text = element.inner_html();
        if script_text.contains("AF_initDataCallback") {
            if let Some(key_cap) = key_regex.captures(&script_text) {
                // Thử bắt mảng JSON từ thẻ script
                if let Some(data_cap) = data_regex.captures(&script_text) {
                    let key = key_cap.get(1).unwrap().as_str().to_string();
                    let mut json_str = data_cap.get(1).unwrap().as_str().to_string();

                    // Làm sạch mảng JSON bẩn của Google
                    json_str = re_empty_array_start
                        .replace_all(&json_str, "[null,")
                        .to_string();
                    while re_empty_elements.is_match(&json_str) {
                        json_str = re_empty_elements
                            .replace_all(&json_str, ",null,")
                            .to_string();
                    }

                    // Cắt bỏ phần đuôi rác nếu regex lỡ bắt quá đà
                    if let Some(idx) = json_str.rfind("];") {
                        json_str.truncate(idx + 1);
                    } else if let Some(idx) = json_str.rfind("}]") {
                        // tìm dấu ngoặc vuông cuối cùng
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

pub fn parse_batchexecute_response(raw_response: &str) -> Option<Value> {
    let clean_str = if raw_response.starts_with(")]}'") {
        raw_response[4..].trim()
    } else {
        raw_response.trim()
    };

    let outer_json: Value = serde_json::from_str(clean_str).ok()?;

    let inner_str = outer_json
        .as_array()?
        .get(0)?
        .as_array()?
        .get(2)?
        .as_str()?;

    serde_json::from_str(inner_str).ok()
}
