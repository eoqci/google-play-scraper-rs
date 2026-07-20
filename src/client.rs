use reqwest::{Client, header};

use crate::error::ScrapperError;

pub async fn fetch_html(url: &str) -> Result<String, ScrapperError> {
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
        return Err(ScrapperError::NotFound);
    }

    let text = response.text().await?;

    Ok(text)
}
