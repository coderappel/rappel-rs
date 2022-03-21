use std::num::ParseIntError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestContext {
  pub user_id: i64,
}

pub fn extract_context<T>(request: &tonic::Request<T>) -> Result<RequestContext, tonic::Status> {
  let user_id = match request.metadata().get("x-user-id") {
    Some(u) => u.to_str().unwrap().parse(),
    None => return Err(tonic::Status::unauthenticated("Missing `x-user-id` in metadata")),
  }.map_err(|error: ParseIntError| {
    tracing::error!(message = "failed to get user_id from metadata `x-user-id`", %error);
    tonic::Status::unauthenticated(error.to_string())
  })?;

  Ok(RequestContext {user_id})
} 
