use std::num::ParseIntError;

use tonic::metadata::errors::ToStrError;

#[derive(Debug, Clone)]
pub struct Context {
  user_id: i64,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("Malformed context information")]
  Malformed,

  #[error("Missing context information")]
  NotFound,
}

impl From<Error> for tonic::Status {
    fn from(_: Error) -> Self {
        tonic::Status::unauthenticated("Unauthenticated")
    }
}

impl Context {
  pub fn from_request<T>(r: &tonic::Request<T>) -> Result<Context, Error> {
    let user_id = match r.metadata().get("x-user-id") {
      Some(u) => u.to_str().map(|uid| uid.parse()),
      None => return Err(Error::NotFound),
    }
    .map_err(|error: ToStrError| {
      tracing::debug!(message = "Invalid encoding for `x-user-id`", %error);
      Error::Malformed
    })?
    .map_err(|error: ParseIntError| {
      tracing::debug!(message = "Failed to parse `x-user-id`", %error);
      Error::Malformed
    })?;

    Ok(Context { user_id })
  }

  pub fn user_id(&self) -> i64 {
      self.user_id
  }
}
