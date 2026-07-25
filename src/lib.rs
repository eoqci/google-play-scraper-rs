pub mod details;
pub mod error;
pub mod models;
pub mod reviews;

mod client;
mod parser;

pub use details::*;
pub use error::ScraperError;
pub use models::{AppDetails, Review, ReviewsResult, SortType};
pub use reviews::get_reviews as reviews;
