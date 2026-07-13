use serde::{Deserialize, Serialize};
use crate::request_clients::request_errors::server_other_body_error::ServerOtherBodyError;
use crate::request_clients::request_errors::server_validation_error::ServerValidationBodyError;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ServerBodyError {
    Other(ServerOtherBodyError),
    Validation(ServerValidationBodyError),
}

impl std::fmt::Display for ServerBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerBodyError::Other(err) => write!(f, "{}", err),
            ServerBodyError::Validation(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for ServerBodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerBodyError::Other(err) => Some(err),
            ServerBodyError::Validation(err) => Some(err),
        }
    }
}

impl ServerBodyError {
    pub fn as_other_error(&self) -> Option<&ServerOtherBodyError> {
        match self {
            Self::Other(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn as_validation_error(&self) -> Option<&ServerValidationBodyError> {
        match self {
            Self::Validation(inner) => Some(inner),
            _ => None,
        }
    }
}