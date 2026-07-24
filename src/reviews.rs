// File: src/reviews.rs
use crate::client::fetch_batchexecute;
use crate::error::ScraperError;
use crate::models::{Review, ReviewsResult, SortType};
use crate::parser::parse_batchexecute_response;
use serde_json::Value;

// Hàm tạo body POST gửi lên Google Play
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

// Smart Scanner: Tự động lùng sục tìm mảng chứa danh sách các bình luận
fn find_reviews_list(root: &Value) -> Option<&Vec<Value>> {
    if let Some(arr) = root.as_array() {
        if !arr.is_empty() {
            // Nhận diện mảng reviews: Phần tử con đầu tiên của nó phải là mảng, và bắt đầu bằng chuỗi UUID (ID bình luận)
            if let Some(first_item) = arr[0].as_array() {
                if let Some(id_str) = first_item.get(0).and_then(|v| v.as_str()) {
                    if id_str.len() >= 32 && id_str.contains('-') {
                        return Some(arr);
                    }
                }
            }
        }
        // Đệ quy tìm sâu bên trong
        for child in arr {
            if let Some(found) = find_reviews_list(child) {
                return Some(found);
            }
        }
    }
    None
}

// Smart Scanner: Tự động tìm chuỗi Token phân trang (Next Page Token)
fn find_pagination_token(root: &Value) -> Option<String> {
    if let Some(arr) = root.as_array() {
        // Token thường nằm trong mảng dạng [null, "Chuỗi_Rất_Dài"]
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
                // Map từng trường dữ liệu theo đúng index của Google Play
                let id = arr
                    .get(0)
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

                // ĐÂY CHÍNH LÀ INDEX SỐ 10 BẠN ĐANG TÌM!
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
