<h1 align="center"> GOOGLE PLAY SCRAPER </h1>
<div align="center">
    
[![Crates.io](https://img.shields.io/crates/v/google_play_scraper_rs.svg)](https://crates.io/crates/google-play-scraper-rs)
[![Documentation](https://docs.rs/google-play-scraper-rs/badge.svg)](https://docs.rs/google-play-scraper-rs)
[![License](https://img.shields.io/crates/l/google_play_scraper_rs.svg)](LICENSE)

</div>
<p align="center">A Rust crate for extracting and filtering application data from Google Play.</p>




## Overview

`google_play_scraper_rs` lets you fetch application metadata and user reviews from the Google Play Store without using the official (limited) Play Developer API. It parses the same data the Play Store web page uses internally and maps it into strongly typed Rust structs.

This is useful for:

- Competitive analysis and app store monitoring
- Tracking ratings, install counts, and version history over time
- Collecting and analyzing user reviews
- Building internal tools, dashboards, or datasets around Play Store data

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
google-play-scraper-rs = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

Or install via cargo:

```bash
cargo add google-play-scraper-rs
```

## Quick Start

```rust
use google_play_scraper_rs::{details::get_app_details, reviews::get_reviews, models::SortType};

#[tokio::main]
async fn main() {
    let app_id = "com.shopee.vn";

    let app_info = get_app_details(app_id).await.unwrap();
    println!("{} - {} stars ({} ratings)", app_info.title, app_info.score, app_info.ratings);

    let reviews = get_reviews(app_id, SortType::Newest, 20, None).await.unwrap();
    println!("Fetched {} reviews", reviews.data.len());
}
```

## Usage

### Fetching app details

```rust
use google_play_scraper_rs::details::get_app_details;

let app = get_app_details("com.shopee.vn").await?;
```

Returns an `AppDetails` struct containing title, description, ratings, install counts, pricing, developer information, categories, media assets, and more.

### Fetching reviews

```rust
use google_play_scraper_rs::{reviews::get_reviews, models::SortType};

let result = get_reviews("com.shopee.vn", SortType::Newest, 50, None).await?;

for review in result.data {
    println!("{}: {}", review.user_name, review.text);
}
```

Supported sort types: `SortType::Newest`, `SortType::Rating`, `SortType::Helpfulness`.

Pagination is supported via `next_pagination_token`:

```rust
let mut token = None;

loop {
    let result = get_reviews("com.shopee.vn", SortType::Newest, 50, token.clone()).await?;
    // process result.data ...

    match result.next_pagination_token {
        Some(next) => token = Some(next),
        None => break,
    }
}
```

## Example Output

Calling `get_app_details` on `com.shopee.vn` produces a struct that serializes to JSON like this:

```json
{
  "title": "Shopee 8.8 Ưu Đãi Nửa Giá",
  "summary": "Xtra vouchers offer up to 50 percent off | Free shipping",
  "installs": "100,000,000+",
  "score": 4.280374,
  "scoreText": "4.3",
  "ratings": 2400581,
  "reviews": 343,
  "currency": "USD",
  "free": true,
  "developer": "Shopee VN",
  "genre": "Shopping",
  "categories": [{ "name": "Shopping", "id": "SHOPPING" }],
  "contentRating": "Everyone",
  "released": "Jul 16, 2019",
  "version": "3.78.28",
  "appId": "com.shopee.vn",
  "url": "https://play.google.com/store/apps/details?id=com.shopee.vn&hl=en&gl=us"
}
```

See [`examples/`](examples/) for a runnable script that fetches both app details and reviews and writes the combined result to a JSON file.

## Data Model

| Struct          | Description                                              |
| --------------- | --------------------------------------------------------- |
| `AppDetails`    | Full metadata for a single app                             |
| `Category`      | An app's store category (name + id)                       |
| `Feature`       | A listed app feature (title + description)                |
| `Review`        | A single user review                                       |
| `ReviewsResult` | A page of reviews plus a pagination token                  |
| `SortType`      | Review sort order: `Newest`, `Rating`, `Helpfulness`       |

All structs derive `Serialize` / `Deserialize` and use `camelCase` field names when serialized, matching the Play Store's own JSON conventions.

## Running the Examples

```bash
cargo run --example scrape_app
```

This fetches app details and reviews for a sample app ID and writes the output to `<app_id>_data.json` in the current directory.

## Limitations

- This crate scrapes publicly available Play Store pages; it is not an official Google API and may break if Google changes the page's internal data format.
- Some fields (e.g. `editors_choice`) are not reliably available and may always return a default value.
- Respect Google Play's Terms of Service and avoid sending excessive requests in a short period of time.

## Contributing

Issues and pull requests are welcome. If you find a field that stopped parsing correctly (Google occasionally reshuffles its internal data layout), please open an issue with the affected `app_id` so the mapping can be fixed.

## License

Licensed under either of

- [MIT LICENSE](https://github.com/eoqci/google-play-scraper-rs/blob/main/LICENSE)
