use std::sync::Arc;
use tokio::sync::Mutex;
use super::store::Error;
use super::store::OperationsStore;
use crate::proto::longrunning::Operation;
use crate::proto::longrunning::GetOperationRequest;
use crate::proto::longrunning::CancelOperationRequest;
use crate::proto::longrunning::operations_server;

#[derive(Debug, Clone)]
pub struct Service<S: OperationsStore> {
  store: Arc<Mutex<S>>,
}

impl<S: OperationsStore> Service<S> {
  pub fn new(store: S) -> Self {
    Self {
      store: Arc::new(Mutex::new(store))
    }
  }
}

type TonicResult<T> = Result<tonic::Response<T>, tonic::Status>;

#[async_trait::async_trait]
impl<S: 'static> operations_server::Operations for Service<S>
  where
    S: OperationsStore + Send + Sync
{
  #[tracing::instrument(skip(self))]
  async fn get(&self, request: tonic::Request<GetOperationRequest>) -> TonicResult<Operation> {
    let ctx = crate::grpc::extract_context(&request)?;
    let operation_id = request.into_inner().operation_id;
    let store = self.store.clone();
    let store = store.lock().await;
    let result = store.get(&operation_id, &ctx).await;

    result
      .map(|operation| tonic::Response::new(operation))
      .map_err(|error| match error {
        Error::NotFound => tonic::Status::not_found("Invalid Operation"),
        error => {
          tracing::error!(message = "Failed to fetch operation", %error, %operation_id);
          tonic::Status::internal(error.to_string())
        }
      })
  }

  #[tracing::instrument(skip(self))]
  async fn cancel(&self, _request: tonic::Request<CancelOperationRequest>) -> TonicResult<()> {
    Err(tonic::Status::unimplemented("unimplemented"))
  }
}
