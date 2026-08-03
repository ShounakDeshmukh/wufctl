use thiserror::Error;

#[derive(Error, Debug)]
pub enum VclError {
    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("XML parse error: {0}")]
    XmlParseError(String),

    #[error("VCL API error: {0}")]
    ApiError(String),

    #[error("Missing token")]
    MissingToken,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

pub type Result<T> = std::result::Result<T, VclError>;
