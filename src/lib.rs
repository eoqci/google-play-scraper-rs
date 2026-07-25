//! # google_play_scraper_rs
//!
//! A Rust crate for extracting application details and user reviews from
//! the Google Play Store.
//!
//! Google Play does not provide a public API for this data, so this crate
//! works by parsing the same internal data the Play Store web app itself
//! relies on: an embedded JSON blob (`ds:5`) for app details, and the
//! `batchexecute` RPC mechanism for reviews. Because none of this is
//! officially documented or guaranteed stable, parsing here is
//! intentionally defensive — see the [`error`] module for how failures
//! are reported.
//!
//! # Quick start
//!
//! ```no_run
//! use google_play_scraper_rs::{details::get_app_details, reviews::get_reviews, models::SortType};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let app = get_app_details("com.miHoYo.GenshinImpact").await?;
//! println!("{} — {} stars", app.title, app.score);
//!
//! let reviews = get_reviews("com.miHoYo.GenshinImpact", SortType::Newest, 20, None).await?;
//! println!("Fetched {} reviews", reviews.data.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Module overview
//!
//! - [`details`]: fetches and parses an app's details page into
//!   [`AppDetails`].
//! - [`reviews`]: fetches and parses paginated user reviews into
//!   [`ReviewsResult`].
//! - [`models`]: the data structures returned by this crate
//!   ([`AppDetails`], [`Review`], [`ReviewsResult`], [`SortType`], etc.).
//! - [`error`]: the [`ScraperError`] type returned by fallible operations.
//! - `client` and `parser` are internal implementation details (HTTP
//!   requests and raw response parsing) and are not part of the public
//!   API.
//!
//! # A note on the top-level re-exports
//!
//! For convenience, the most commonly used items are re-exported at the
//! crate root, so `google_play_scraper_rs::AppDetails` works just as well
//! as `google_play_scraper_rs::models::AppDetails`. One re-export in
//! particular is worth calling out explicitly: `get_reviews` is
//! re-exported *as* `reviews`, meaning `google_play_scraper_rs::reviews`
//! refers to two different things depending on context — the `reviews`
//! *module* (`google_play_scraper_rs::reviews::get_reviews`, the full
//! path) and the re-exported `reviews` *function*
//! (`google_play_scraper_rs::reviews(...)`, called directly). This is
//! valid Rust (modules and functions live in separate namespaces), but it
//! means `google_play_scraper_rs::reviews(...)` and
//! `google_play_scraper_rs::reviews::get_reviews(...)` both work and do
//! the same thing — pick whichever reads better at the call site.

mod client;
pub mod details;
pub mod error;
pub mod models;
mod parser;
pub mod reviews;

pub use details::*;
pub use error::ScraperError;
pub use models::{AppDetails, Review, ReviewsResult, SortType};
pub use reviews::get_reviews as reviews;
