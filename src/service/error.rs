use std::net::AddrParseError;
use tonic::codegen::http::uri::InvalidUri;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Config Error: {0}")]
  ConfigError(#[from] config::ConfigError),

  #[error("Invalid Uri: {0}")]
  InvalidUriError(#[from] InvalidUri),

  #[error("Cannot field client for key: {0}")]
  MissingClient(String),

  #[error("Tonic Transport Error: {0}")]
  TonicTransportError(#[from] tonic::transport::Error),

  #[error("AddrParseError: {0}")]
  AddrParseError(#[from] AddrParseError),
}
