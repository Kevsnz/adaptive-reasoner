use std::fmt;

#[derive(Debug, Clone)]
pub enum ReasonerError {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn test_error_display_validation_error() {
        let error = ReasonerError::ValidationError("test message".to_string());
        assert_eq!(error.to_string(), "Validation error: test message");
    }

    #[test]
    fn test_error_display_api_error() {
        let error = ReasonerError::ApiError("API failed".to_string());
        assert_eq!(error.to_string(), "API error: API failed");
    }

    #[test]
    fn test_error_display_parse_error() {
        let error = ReasonerError::ParseError("Invalid JSON".to_string());
        assert_eq!(error.to_string(), "Parse error: Invalid JSON");
    }

    #[test]
    fn test_error_display_config_error() {
        let error = ReasonerError::ConfigError("Missing config".to_string());
        assert_eq!(error.to_string(), "Config error: Missing config");
    }

    #[test]
    fn test_error_display_network_error() {
        let error = ReasonerError::NetworkError("Connection refused".to_string());
        assert_eq!(error.to_string(), "Network error: Connection refused");
    }

    #[test]
    fn test_error_from_string() {
        let error: ReasonerError = "test error".to_string().into();
        match error {
            ReasonerError::ValidationError(msg) => {
                assert_eq!(msg, "test error");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_error_from_str() {
        let error: ReasonerError = "test error".into();
        match error {
            ReasonerError::ValidationError(msg) => {
                assert_eq!(msg, "test error");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_error_debug() {
        let error = ReasonerError::ValidationError("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ValidationError"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_error_source() {
        let error = ReasonerError::ApiError("API error".to_string());
        assert!(error.source().is_none());
    }

    #[test]
    fn test_error_clone() {
        let error = ReasonerError::NetworkError("timeout".to_string());
        let cloned = error.clone();
        assert_eq!(error.to_string(), cloned.to_string());
    }
}
