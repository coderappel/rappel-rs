#[cfg(feature = "redis")]
pub mod redis;
mod types;

use std::time::Duration;
pub use types::*;

use crate::proto::longrunning::GetOperationRequest;
use crate::proto::longrunning::Operation;
use crate::service::OperationsSvcClient;

pub async fn wait(
  client: &mut OperationsSvcClient,
  operation_id: &str,
) -> Result<Operation, tonic::Status> {
  let id = operation_id.to_string();
  loop {
    let operation_id = id.clone();
    tracing::trace!(message = "Polling the operation status", %operation_id);
    match client.get(GetOperationRequest { operation_id }).await {
      Ok(response) => {
        let operation = response.into_inner();

        if operation.done {
          tracing::debug!(message = "Operation completed", operation_id=%id);
          return Ok(operation);
        }
      }
      Err(error) => {
        tracing::debug!(message = "Polling the operation failed", operation_id=%id, %error);
        return Err(error);
      }
    }
    let _ = tokio::time::sleep(Duration::from_millis(1000)).await;
  }
}
