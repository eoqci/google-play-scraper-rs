use crate::client::fetch_batchexecute;
use crate::error::ScraperError;
use crate::model::{Review, ReviewsResult, SortType};
use crate::parser::{get_json_val, parse_batchexecute_response};
use serde_json::Value;

// Giống hàm getBodyForRequests trong reviews.js
fn build_reviews_body(app_id: &str, sort: i32, count: usize, token: Option<&str>) -> String {
    // Nếu có token thì chèn token vào chuỗi, nếu không thì dùng `null`
    let token_str = match token {
        Some(t) => format!("\\\"{}\\\"", t),
        None => "null".to_string(),
    };

    let inner_req = format!(
        "[null,null,[2,{},[{},null,{}],null,[]],[\"{}\",7]]",
        sort, count, token_str, app_id
    );

    // Encode URL nhẹ nhàng (thay " bằng %22, [ thành %5B, v.v...)
    // Trong JS họ hardcode encode luôn: f.req=%5B%5B%5B%22UsvDTd%22%2C%22...
    // Ở Rust, ta dùng thư viện `url` để encode cho an toàn.

    use url::form_urlencoded;
    let req_value = format!("[[[\"UsvDTd\",\"{}\",null,\"generic\"]]]", inner_req);

    form_urlencoded::Serializer::new(String::new())
        .append_pair("f.req", &req_value)
        .finish()
}
// Hàm parse thời gian cực dị của Google (Mảng 2 phần tử giây và nano giây)
fn parse_date(date_arr: Option<&Value>) -> String {
    if let Some(arr) = date_arr.and_then(|v| v.as_array()) {
        if let (Some(secs), Some(nanos)) = (arr.get(0), arr.get(1)) {
            let s = secs.as_u64().unwrap_or(0);
            let n_str = format!("{:03}", nanos.as_u64().unwrap_or(0));
            // Cắt 3 số đầu của nano ghép vào giây để thành miliseconds
            let ms = format!("{}{}", s, &n_str[0..3]);
            return ms; // Bạn có thể dùng crate `chrono` để convert sang chuẩn ISO8601 nếu thích
        }
    }
    String::new()
}

fn map_review(app_id: &str, val: &Value) -> Option<Review> {
    // val chính là mảng chứa 1 bình luận

    let id = get_json_val(val, &[0])?.as_str()?.to_string();
    let user_name = get_json_val(val, &[1, 0])
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let user_image = get_json_val(val, &[1, 1, 3, 2])
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let score = get_json_val(val, &[2])
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let text = get_json_val(val, &[4])
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let thumbs_up = get_json_val(val, &[6])
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let date = parse_date(get_json_val(val, &[5]));
    let url = format!(
        "https://play.google.com/store/apps/details?id={}&reviewId={}",
        app_id, id
    );

    let reply_date = get_json_val(val, &[7, 2]).map(|v| parse_date(Some(v)));
    let reply_text = get_json_val(val, &[7, 1])
        .and_then(|v| v.as_str())
        .map(String::from);
    let version = get_json_val(val, &[10])
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(Review {
        id,
        user_name,
        user_image,
        date,
        score,
        url,
        text,
        reply_date,
        reply_text,
        version,
        thumbs_up,
    })
}

pub async fn get_reviews(
    app_id: &str,
    sort: SortType,
    count: usize,
    pagination_token: Option<&str>,
) -> Result<ReviewsResult, ScraperError> {
    // 1. Tạo body
    let body = build_reviews_body(app_id, sort as i32, count, pagination_token);

    // 2. Fetch từ API
    let url = "https://play.google.com/_/PlayStoreUi/data/batchexecute?rpcids=qnKhOb&f.sid=-697906427155521722&hl=en&gl=us";
    let raw_resp = fetch_batchexecute(url, &body).await?;

    // 3. Lột vỏ )]}'
    let inner_json = parse_batchexecute_response(&raw_resp).ok_or(ScraperError::ParseError)?;

    // 4. Móc data
    let mut reviews = Vec::new();
    let mut next_token = None;

    // Trong mảng trả về, [0] chứa danh sách review, [1][1] chứa token trang tiếp theo (MAPPINGS trong js)
    if let Some(reviews_array) = get_json_val(&inner_json, &[0]).and_then(|v| v.as_array()) {
        for rev_val in reviews_array {
            if let Some(review) = map_review(app_id, rev_val) {
                reviews.push(review);
            }
        }
    }

    if let Some(token_val) = get_json_val(&inner_json, &[1, 1]).and_then(|v| v.as_str()) {
        next_token = Some(token_val.to_string());
    }

    Ok(ReviewsResult {
        data: reviews,
        next_pagination_token: next_token,
    })
}
