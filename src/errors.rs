use std::fmt;

#[derive(Debug, Clone)]
pub(crate) enum ReasonerError {
    ValidationError(String),
    ApiError(String),
    ParseError(String),
    ConfigError(String),
    NetworkError(String),
}

impl fmt::Display for ReasonerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReasonerError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ReasonerError::ApiError(msg) => write!(f, "API error: {}", msg),
            ReasonerError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ReasonerError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            ReasonerError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for ReasonerError {}

impl From<String> for ReasonerError {
    fn from(msg: String) -> Self {
        ReasonerError::ValidationError(msg)
    }
}

impl From<&str> for ReasonerError {
    fn from(msg: &str) -> Self {
        ReasonerError::ValidationError(msg.to_string())
    }
}

impl From<reqwest::Error> for ReasonerError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() || err.is_connect() {
            ReasonerError::NetworkError(err.to_string())
        } else {
            ReasonerError::ApiError(err.to_string())
        }
    }
}

impl From<reqwest::header::ToStrError> for ReasonerError {
    fn from(err: reqwest::header::ToStrError) -> Self {
        ReasonerError::ParseError(err.to_string())
    }
}

impl From<serde_json::Error> for ReasonerError {
    fn from(err: serde_json::Error) -> Self {
        ReasonerError::ParseError(err.to_string())
    }
}

impl From<actix_web::error::Error> for ReasonerError {
    fn from(err: actix_web::error::Error) -> Self {
        ReasonerError::ApiError(err.to_string())
    }
}

impl From<std::io::Error> for ReasonerError {
    fn from(err: std::io::Error) -> Self {
        ReasonerError::ConfigError(err.to_string())
    }
}

impl From<actix_web::mime::FromStrError> for ReasonerError {
    fn from(err: actix_web::mime::FromStrError) -> Self {
        ReasonerError::ParseError(err.to_string())
    }
}
