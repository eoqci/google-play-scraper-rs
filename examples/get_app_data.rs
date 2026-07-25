use google_play_scraper_rs::{
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

const APP_ID: &str = "com.shopee.vn";

#[tokio::main]
async fn main() {
    println!("Starting scrape for app: {}", APP_ID);

    // 1. Fetch app details
    println!("Fetching app details...");
    let app_info = match get_app_details(APP_ID).await {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error fetching app details: {:?}", e);
            return;
        }
    };
    println!("App details fetched successfully.");

    // 2. Fetch latest reviews
    println!("Fetching reviews...");
    let reviews = match get_reviews(APP_ID, SortType::Newest, 5, None).await {
        Ok(rev) => rev,
        Err(e) => {
            eprintln!("Error fetching reviews: {:?}", e);
            ReviewsResult {
                data: vec![],
                next_pagination_token: None,
            }
        }
    };
    println!("Fetched {} review(s).", reviews.data.len());

    // 3. Bundle data
    let export_data = ExportData { app_info, reviews };

    // 4. Serialize and write to file
    println!("Saving data to file...");
    match serde_json::to_string_pretty(&export_data) {
        Ok(json_string) => {
            let file_name = format!("{}_data.json", APP_ID);
            match File::create(&file_name) {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(json_string.as_bytes()) {
                        eprintln!("Error writing file: {:?}", e);
                        return;
                    }
                    println!("Done. Output saved to `{}`.", file_name);
                }
                Err(e) => {
                    eprintln!("Error creating file `{}`: {:?}", file_name, e);
                }
            }
        }
        Err(e) => {
            eprintln!("Error serializing JSON: {:?}", e);
        }
    }
}
