use crate::grpc::RequestContext;
use crate::proto::longrunning::Operation;

#[cfg(feature = "redis-store")]
use redis::AsyncCommands;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[cfg(feature = "redis-store")]
  #[error("Read error")]
  RedisError(#[from] redis::RedisError),

  #[error("Deserialization failed")]
  ProstDecodeError(#[from] prost::DecodeError),

  #[error("Serialization failed")]
  ProstEncodeError(#[from] prost::EncodeError),

  #[error("Internal Error")]
  Internal(#[from] anyhow::Error),
  
  #[error("Not Found")]
  NotFound,

  #[error("Unknown Error")]
  Unknown,
}

#[async_trait::async_trait]
pub trait OperationsStore {
  async fn get(&self, id: &str, ctx: &RequestContext) -> Result<Operation, Error>;

  async fn set(&mut self, op: Operation, ctx: &RequestContext) -> Result<(), Error>;

  async fn delete(&mut self, id: &str, ctx: &RequestContext) -> Result<(), Error>;
}

#[cfg(feature = "redis-store")]
#[derive(Debug)]
pub struct RedisStore {
  client: redis::Client,
}

#[cfg(feature = "redis-store")]
impl RedisStore {
  pub fn new(client: redis::Client) -> Self {
    Self { client }
  }
}

#[cfg(feature = "redis-store")]
#[async_trait::async_trait]
impl OperationsStore for RedisStore {
  async fn get(&self, id: &str, _ctx: &RequestContext) -> Result<Operation, Error> {
    let mut conn = self.client.get_async_connection().await?;
    let value = conn.get(id).await?;

    let value: Result<bytes::Bytes, Error> = match value {
      redis::Value::Nil => Err(Error::NotFound),
      data => redis::FromRedisValue::from_redis_value(&data).map_err(|e| e.into()),
    };

    match value {
      Ok(buf) => prost::Message::decode(buf).map_err(|e| e.into()),
      Err(error) => Err(error),
    }
  }

  async fn set(&mut self, op: Operation, _ctx: &RequestContext) -> Result<(), Error> {
    let mut buf = bytes::BytesMut::with_capacity(op.encoded_len());
    op.encode(&mut buf)?;
    let buf = buf.to_vec();

    let mut conn = self.client.get_async_connection().await?;
    conn.set(op.operation_id, buf).await.map_err(|e| e.into())
  }

  async fn delete(&mut self, _id: &str, _ctx: &RequestContext) -> Result<(), Error> {
    unimplemented!("Not Implemented")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use redis::AsyncCommands;
  use super::OperationsStore;
  use crate::proto::longrunning::Operation;

  #[cfg(feature = "redis-store")]
  #[tokio::test]
  async fn test_redis_store_to_get_value() {
    let ctx = RequestContext { user_id: 1 };
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let store = RedisStore::new(client);

    let error = store.get("non_existing_id", &ctx).await.err().unwrap();

    match error {
      Error::NotFound => assert!(true),
      _ => assert!(false, "Should be a RedisError"),
    }
  }

  #[cfg(feature = "redis-store")]
  #[tokio::test]
  async fn test_redis_store_to_get_existing_value() {
    let ctx = RequestContext { user_id: 1 };
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut store = RedisStore::new(client);
    let operation_id = uuid::Uuid::new_v4().to_string();
    let mut operation = Operation::default();
    operation.operation_id = operation_id.clone();

    store.set(operation.clone()).await;
    let result = store.get(&operation_id, &ctx).await.unwrap();

    assert_eq!(result, operation);
  }
}
