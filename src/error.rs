#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Failed to parse module")]
    ParseModule(swc_ecma_parser::error::Error),
    #[error("{0}")]
    Error(String),
    #[error("Failed to compile regex: {0}")]
    Regex(#[from] regex::Error),
    #[error("Failed to build: {0}")]
    Ignore(#[from] ignore::Error),
}
