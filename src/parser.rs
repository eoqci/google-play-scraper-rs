use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

// Không cần import ScrapperError ở đây nữa trừ khi bạn dùng nó để trả về Err.

// SỬA Ở ĐÂY: Trả về HashMap<String, Value> thay vì HashMap<String, ScrapperError>
pub fn extract_init_data(html: &str) -> HashMap<String, Value> {
    let mut parsed_data = HashMap::new();

    let re = Regex::new(
        r"AF_initDataCallback\s*\(.*?key:\s*'([^']+)'\s*,.*?data:\s*(\[.*?\])\s*}\s*\);",
    )
    .unwrap();

    for cap in re.captures_iter(html) {
        if let (Some(key_match), Some(data_match)) = (cap.get(1), cap.get(2)) {
            let key = key_match.as_str().to_string();
            // Lưu ý: Đừng quên thêm code "làm sạch" chuỗi JSON như mình đề cập ở Step 3
            // của câu trả lời trước, nếu không đoạn from_str bên dưới sẽ văng lỗi rất nhiều.
            let mut json_str = data_match.as_str().to_string();
            json_str = json_str.replace("[,", "[null,");
            while json_str.contains(",,") {
                json_str = json_str.replace(",,", ",null,");
            }

            // Rust giờ đã biết Value ở đây là serde_json::Value nhờ chữ ký hàm
            match serde_json::from_str(&json_str) {
                Ok(value) => {
                    parsed_data.insert(key, value);
                }
                Err(e) => {
                    eprintln!("Lỗi parse JSON cho key {}: {}", key, e);
                }
            }
        }
    }

    parsed_data
}

// src/parser.rs
// Hàm phụ trợ để điều hướng trong mảng đa chiều bằng index
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_init_data_basic() {
        // Giả lập một đoạn HTML có chứa script data của Google Play
        // Chú ý: Cấu trúc JSON cố tình có các phần tử rỗng như [1, , 3] (mô phỏng lỗi của Google)
        let mock_html = r#"
            <html>
                <head></head>
                <body>
                    <script nonce="xxx">
                        AF_initDataCallback({key: 'ds:1', isError:  false , hash: '1', data: ["Test App", 4.5, 1000] });
                    </script>
                    <div>Some random content</div>
                    <script nonce="yyy">
                        AF_initDataCallback({key: 'ds:5', isError:  false , hash: '2', data: [["Google Translate", , "Translate stuff", , , "1B+"]] });
                    </script>
                </body>
            </html>
        "#;

        let result = extract_init_data(mock_html);

        // 1. Kiểm tra xem Regex có bắt được đúng 2 khối dữ liệu không
        assert_eq!(result.len(), 2, "Phải bắt được chính xác 2 khối data");
        assert!(result.contains_key("ds:1"));
        assert!(result.contains_key("ds:5"));

        // 2. Kiểm tra mảng JSON chuẩn (ds:1) có được parse đúng không
        let ds1 = result.get("ds:1").unwrap();
        assert_eq!(ds1[0].as_str(), Some("Test App"));
        assert_eq!(ds1[1].as_f64(), Some(4.5));

        // 3. Kiểm tra mảng JSON bị thiếu phần tử (ds:5) có được dọn dẹp và parse thành null không
        let ds5 = result.get("ds:5").unwrap();
        let inner_array = &ds5[0];

        assert_eq!(inner_array[0].as_str(), Some("Google Translate"));
        // Phần tử trống ở index 1 phải được parse thành chuỗi null
        assert!(
            inner_array[1].is_null(),
            "Phần tử trống phải biến thành null"
        );
        assert_eq!(inner_array[2].as_str(), Some("Translate stuff"));
        assert!(inner_array[3].is_null());
        assert!(inner_array[4].is_null());
        assert_eq!(inner_array[5].as_str(), Some("1B+"));
    }

    #[test]
    fn test_get_json_val_helper() {
        use serde_json::json;

        // Tạo một cấu trúc JSON đa chiều giả lập
        let mock_json = json!([
            "Item 0",
            [
                "Item 1.0",
                [
                    "Target Value", // Index: [1, 1, 0]
                    42              // Index: [1, 1, 1]
                ]
            ]
        ]);

        // Trích xuất thành công
        let target_str = get_json_val(&mock_json, &[1, 1, 0]);
        assert_eq!(target_str.unwrap().as_str(), Some("Target Value"));

        let target_num = get_json_val(&mock_json, &[1, 1, 1]);
        assert_eq!(target_num.unwrap().as_u64(), Some(42));

        // Báo None khi đi sai path (Out of bounds)
        let invalid_path = get_json_val(&mock_json, &[1, 2, 0]);
        assert!(invalid_path.is_none());
    }
}
