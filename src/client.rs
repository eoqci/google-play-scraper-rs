// File: src/client.rs
use crate::error::ScraperError;
use reqwest::{Client, header};
use serde_json::Value;

pub async fn fetch_html(url: &str) -> Result<String, ScraperError> {
    let mut headers = header::HeaderMap::new();

    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    );

    headers.insert(
        header::ACCEPT_LANGUAGE,
        header::HeaderValue::from_static("en-US;q=0.9,en;q=0.8"),
    );

    let client = Client::builder().default_headers(headers).build()?;

    let response = client.get(url).send().await?;

    if response.status() == 404 {
        return Err(ScraperError::NotFound);
    }

    let text = response.text().await?;

    Ok(text)
}

// HÀM MỚI: Dành riêng cho Pagination / Search / Reviews
pub async fn fetch_batchexecute(url: &str, form_body: &str) -> Result<String, ScraperError> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        ),
    );
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/x-www-form-urlencoded;charset=UTF-8"),
    );

    let client = Client::builder()
        .default_headers(headers)
        // Nếu làm nghiêm túc, bạn cần cookie_store(true) ở đây
        // để Google không chặn khi request nhiều trang
        .cookie_store(true)
        .build()?;

    let response = client.post(url).body(form_body.to_string()).send().await?;

    if !response.status().is_success() {
        return Err(ScraperError::NotFound); // Bạn có thể thêm lỗi NetworkError chi tiết hơn
    }

    let text = response.text().await?;
    Ok(text)
}
