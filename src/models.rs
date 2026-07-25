use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")] // Cực kỳ quan trọng: Tự động map snake_case (Rust) thành camelCase (JSON)
pub struct AppDetails {
    pub title: String,
    pub description: String,
    #[serde(rename = "descriptionHTML")]
    pub description_html: String,
    pub summary: String,
    pub installs: String,
    pub min_installs: u64,
    pub max_installs: u64,
    pub score: f64,
    pub score_text: String,
    pub ratings: u64,
    pub reviews: u64,
    pub histogram: HashMap<String, u64>, // Hứng object { "1": ..., "2": ... }
    pub price: f64,
    pub free: bool,
    pub currency: String,
    pub price_text: String,
    #[serde(rename = "offersIAP")]
    pub offers_iap: bool,
    #[serde(rename = "IAPRange")]
    pub iap_range: Option<String>, // Option vì JS trả về undefined
    pub android_version: String,
    pub android_version_text: String,
    pub android_max_version: Option<String>,
    pub developer: String,
    pub developer_id: String,
    pub developer_email: Option<String>,
    pub developer_website: Option<String>,
    pub developer_address: Option<String>,
    pub developer_legal_name: Option<String>,
    pub developer_legal_email: Option<String>,
    pub developer_legal_address: Option<String>,
    pub developer_legal_phone_number: Option<String>,
    pub privacy_policy: Option<String>,
    #[serde(rename = "developerInternalID")]
    pub developer_internal_id: String,
    pub genre: String,
    pub genre_id: String,
    pub categories: Vec<Category>,
    pub icon: String,
    pub header_image: Option<String>,
    pub screenshots: Vec<String>,
    pub video: Option<String>,
    pub video_image: Option<String>,
    pub preview_video: Option<String>,
    pub content_rating: String,
    pub content_rating_description: Option<String>,
    pub ad_supported: bool,
    pub released: Option<String>,
    pub updated: u64,
    pub version: String,
    pub recent_changes: Option<String>,
    pub comments: Vec<String>,
    pub preregister: bool,
    pub early_access_enabled: bool,
    pub is_available_in_play_pass: bool,
    pub editors_choice: bool,
    pub features: Vec<Feature>,
    pub app_id: String,
    pub url: String,
}

// Bóc tách các object con ra thành struct riêng cho sạch sẽ
#[derive(Debug, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub id: Option<String>, // id có thể là null
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feature {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub user_name: String,
    pub user_image: String,
    pub date: String,
    pub score: u64,
    pub url: String,
    pub text: String,
    pub reply_date: Option<String>,
    pub reply_text: Option<String>,
    pub version: Option<String>,
    pub thumbs_up: u64,
    // Criterias (ví dụ: Gameplay: 5 sao, Đồ họa: 4 sao) có thể bỏ qua cho gọn,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewsResult {
    pub data: Vec<Review>,
    pub next_pagination_token: Option<String>,
}

pub enum SortType {
    Newest = 2,
    Rating = 3,
    Helpfulness = 1,
}
