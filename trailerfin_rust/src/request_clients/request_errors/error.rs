use crate::request_clients::request_errors::server_other_body_error::ServerOtherBodyError;
use crate::request_clients::request_errors::server_validation_error::ServerValidationBodyError;

#[derive(Debug)]
pub enum Error {
    Request {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    Response {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    Validation(ServerValidationBodyError),
    Server {
        code: u16,
        content: ServerOtherBodyError,
    },
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    UnsupportedOperation(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Request { .. } => write!(f, "couldn't execute request"),
            Error::Response { .. } => write!(f, "couldn't read response"),
            Error::Validation(err) => write!(f, "validation failed: {}", err),
            Error::Server { code, content } => write!(f, "internal server error with code {}: {}", code, content),
            Error::Reqwest(err) => write!(f, "reqwest error: {}", err),
            Error::SerdeJson(err) => write!(f, "serde_json error: {}", err),
            Error::UnsupportedOperation(op) => write!(f, "unsupported operation: {}", op),
        }
    }
}

impl Error {
    pub fn as_validation_error(&self) -> Option<&ServerValidationBodyError> {
        match self {
            Self::Validation(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn as_server_error(&self) -> Option<&ServerOtherBodyError> {
        match self {
            Self::Server { code: _, content } => Some(content),
            _ => None,
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeJson(err)
    }
}