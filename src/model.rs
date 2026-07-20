use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppDetails {
    pub app_id: String,
    pub title: String,
    pub description: String,
    pub icon_url: String,
}
