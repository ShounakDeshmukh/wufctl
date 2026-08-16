use thiserror::Error;

#[derive(Error, Debug)]
pub enum VclError {
    /// The request didn't get a response in time; the server may never have seen it.
    #[error("Request timed out")]
    Timeout,

    /// Couldn't reach the server at all (DNS, TLS, refused connection, etc).
    #[error("Couldn't connect: {0}")]
    ConnectionFailed(String),

    /// The server responded, but not with a 2xx status.
    #[error("HTTP {status}: {body}")]
    NonSuccessStatus { status: u16, body: String },

    /// Any other transport-level failure (body read, decode, redirect, builder).
    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("XML parse error: {0}")]
    XmlParseError(String),

    #[error("VCL API error: Fault [{code}] {message}")]
    ApiError { code: i64, message: String },

    #[error("Missing token")]
    MissingToken,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

pub type Result<T> = std::result::Result<T, VclError>;
