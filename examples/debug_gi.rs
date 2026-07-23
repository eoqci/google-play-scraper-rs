use googleplay_scraper_rs::{
    details::get_app_details,
    models::{AppDetails, ReviewsResult, SortType},
    reviews::get_reviews,
};
use serde::Serialize;
use std::fs::File;
use std::io::Write;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportData {
    pub app_info: AppDetails,
    pub reviews: ReviewsResult,
}

#[tokio::main]
async fn main() {
    let app_id = "com.miHoYo.GenshinImpact";

    println!("🚀 Bắt đầu cào dữ liệu cho: {}", app_id);

    // 1. Kéo thông tin chi tiết
    println!("⏳ Đang tải thông tin ứng dụng...");
    let app_info = match get_app_details(app_id).await {
        Ok(app) => app,
        Err(e) => {
            eprintln!("❌ Lỗi lấy app details: {:?}", e);
            return;
        }
    };
    println!("✅ Đã lấy xong thông tin ứng dụng!");

    // 2. Kéo 5 bình luận mới nhất
    println!("⏳ Đang tải bình luận...");
    let reviews = match get_reviews(app_id, SortType::Newest, 5, None).await {
        Ok(rev) => rev,
        Err(e) => {
            eprintln!("❌ Lỗi lấy reviews: {:?}", e);
            ReviewsResult {
                data: vec![],
                next_pagination_token: None,
            }
        }
    };
    println!("✅ Đã lấy xong {} bình luận!", reviews.data.len());

    // 3. Đóng gói vào struct tổng
    let export_data = ExportData { app_info, reviews };

    // 4. Serialize thành chuỗi JSON và lưu file
    println!("💾 Đang lưu ra file...");
    match serde_json::to_string_pretty(&export_data) {
        Ok(json_string) => {
            let file_name = format!("{}_data.json", app_id);
            let mut file = File::create(&file_name).expect("Không thể tạo file!");

            file.write_all(json_string.as_bytes())
                .expect("Lỗi ghi file!");

            println!("🎉 Xong! Mở file `{}` để xem thành quả.", file_name);
        }
        Err(e) => {
            eprintln!("❌ Lỗi khi xuất JSON: {:?}", e);
        }
    }
}
