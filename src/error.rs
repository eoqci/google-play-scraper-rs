use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("HTTP request failed: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Failed to parse HTML structure")]
    ParseError,
    #[error("App not found on Play Store")]
    NotFound,
}
