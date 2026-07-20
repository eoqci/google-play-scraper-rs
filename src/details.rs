use crate::{client::fetch_html, error::ScrapperError, model::AppDetails};
use scraper::{Html, Selector};

pub async fn get_app_details(app_id: &str) -> Result<AppDetails, ScrapperError> {
    let url = format!(
        "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
        app_id
    );
    let html = fetch_html(&url).await?;

    let document = Html::parse_document(&html);

    // Tạo các CSS selectors
    let title_selector = Selector::parse("meta[property='og:title']").unwrap();
    let desc_selector = Selector::parse("meta[name='description']").unwrap();
    let icon_selector = Selector::parse("meta[property='og:image']").unwrap();

    // SỬA Ở ĐÂY: Trả về Option<String> thay vì Option<Html>
    let extract_meta = |selector: &Selector| -> Option<String> {
        document
            .select(selector)
            .next()?
            .value()
            .attr("content")
            .map(|s| s.to_string())
    };

    let title = extract_meta(&title_selector).unwrap_or_default();
    let description = extract_meta(&desc_selector).unwrap_or_default();
    let icon_url = extract_meta(&icon_selector).unwrap_or_default();

    Ok(AppDetails {
        app_id: app_id.to_string(),
        title,
        description,
        icon_url,
    })
}

// Thêm vào cuối file (ví dụ: src/details.rs)

#[cfg(test)]
mod tests {
    use super::*; // Import tất cả các hàm từ module bên ngoài vào module test

    #[tokio::test]
    async fn test_get_app_details_debug() {
        // Lấy app YouTube làm ví dụ
        let app_id = "com.google.android.youtube";

        // Gọi hàm và unwrap kết quả
        let result = get_app_details(app_id).await;

        // Nếu lỗi thì in ra lỗi, nếu pass thì chạy tiếp
        assert!(result.is_ok(), "Lỗi khi fetch data: {:?}", result.err());

        let app = result.unwrap();

        // In ra màn hình console những gì lấy được
        println!("=== DỮ LIỆU CÀO ĐƯỢC ===");
        println!("{:#?}", app);
        println!("========================");
    }
}
