use k8s_openapi::http::uri::InvalidUri;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Config Error")]
  ConfigError(#[from] config::ConfigError),

  #[error("Invalid Uri")]
  InvalidUriError(#[from] InvalidUri),

  #[error("Cannot field client for key: {0}")]
  MissingClient(String),

  #[error("Tonic Transport Error")]
  TonicTransportError(#[from] tonic::transport::Error),
}
