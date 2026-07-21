// use crate::{client::fetch_html, error::ScrapperError, model::AppDetails};
// use scraper::{Html, Selector};

// pub async fn get_app_details(app_id: &str) -> Result<AppDetails, ScrapperError> {
//     let url = format!(
//         "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
//         app_id
//     );
//     let html = fetch_html(&url).await?;

//     let document = Html::parse_document(&html);

//     // Tạo các CSS selectors
//     let title_selector = Selector::parse("meta[property='og:title']").unwrap();
//     let desc_selector = Selector::parse("meta[name='description']").unwrap();
//     let icon_selector = Selector::parse("meta[property='og:image']").unwrap();

//     // SỬA Ở ĐÂY: Trả về Option<String> thay vì Option<Html>
//     let extract_meta = |selector: &Selector| -> Option<String> {
//         document
//             .select(selector)
//             .next()?
//             .value()
//             .attr("content")
//             .map(|s| s.to_string())
//     };

//     let title = extract_meta(&title_selector).unwrap_or_default();
//     let description = extract_meta(&desc_selector).unwrap_or_default();
//     let icon_url = extract_meta(&icon_selector).unwrap_or_default();

//     Ok(AppDetails {
//         app_id: app_id.to_string(),
//         title,
//         description,
//         icon_url,
//     })
// }

// // Thêm vào cuối file (ví dụ: src/details.rs)

// #[cfg(test)]
// mod tests {
//     use super::*; // Import tất cả các hàm từ module bên ngoài vào module test

//     #[tokio::test]
//     async fn test_get_app_details_debug() {
//         // Lấy app YouTube làm ví dụ
//         let app_id = "com.google.android.youtube";

//         // Gọi hàm và unwrap kết quả
//         let result = get_app_details(app_id).await;

//         // Nếu lỗi thì in ra lỗi, nếu pass thì chạy tiếp
//         assert!(result.is_ok(), "Lỗi khi fetch data: {:?}", result.err());

//         let app = result.unwrap();

//         // In ra màn hình console những gì lấy được
//         println!("=== DỮ LIỆU CÀO ĐƯỢC ===");
//         println!("{:#?}", app);
//         println!("========================");
//     }
// }

use crate::client::fetch_html;
use crate::error::ScrapperError;
use crate::model::AppDetails;
use crate::parser::{extract_init_data, get_json_val};

pub async fn get_app_details(app_id: &str) -> Result<AppDetails, ScrapperError> {
    let url = format!(
        "https://play.google.com/store/apps/details?id={}&hl=en&gl=us",
        app_id
    );
    let html = fetch_html(&url).await?;

    let parsed_data = extract_init_data(&html);

    // Hầu hết data chi tiết nằm ở ds:5
    let ds5 = parsed_data.get("ds:5").ok_or(ScrapperError::ParseError)?;

    // Bắt đầu quá trình mapping "trâu bò"
    // Lưu ý: Các index này dựa trên cấu trúc cũ của Google, họ có thể thay đổi.
    // Bạn cần đối chiếu với lib gốc hoặc tự debug in ra `ds5` để check index chính xác.

    let mut app = AppDetails::default();
    app.app_id = app_id.to_string();

    // Ví dụ Title: ds:5 -> index [1, 2, 0, 0]
    if let Some(title_val) = get_json_val(ds5, &[1, 2, 0, 0]) {
        if let Some(t) = title_val.as_str() {
            app.title = t.to_string();
        }
    }

    // Ví dụ Description (HTML): ds:5 -> index [1, 2, 72, 0, 1]
    if let Some(desc_val) = get_json_val(ds5, &[1, 2, 72, 0, 1]) {
        if let Some(d) = desc_val.as_str() {
            app.description_html = d.to_string();
            // Có thể dùng một thư viện khác hoặc regex để strip HTML tags cho `app.description`
        }
    }

    // Score: ds:5 -> index [1, 2, 51, 0, 1]
    if let Some(score_val) = get_json_val(ds5, &[1, 2, 51, 0, 1]) {
        if let Some(s) = score_val.as_f64() {
            app.score = s;
        }
    }

    // Installs (dạng chuỗi): ds:5 -> index [1, 2, 13, 0]
    if let Some(installs_val) = get_json_val(ds5, &[1, 2, 13, 0]) {
        if let Some(i) = installs_val.as_str() {
            app.installs = i.to_string();
        }
    }

    Ok(app)
}
