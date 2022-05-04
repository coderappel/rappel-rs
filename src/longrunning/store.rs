use std::fmt::Debug;

use chrono::Utc;
#[allow(unused_imports)]
use prost::Message;
use redis::aio::Connection;
use redis::AsyncCommands;
use redis::FromRedisValue;
use redis::RedisError;
use redis::RedisResult;
use redis::RedisWrite;
use redis::ToRedisArgs;
use redis::Value;
use serde::Deserialize;
use serde::Serialize;
use serde_redis::RedisDeserialize;
use tracing::Instrument;
use uuid::Uuid;

use crate::grpc::RequestContext;
use crate::longrunning::{Error, TaskResult};
use crate::longrunning::Performable;
use crate::longrunning::TaskState;
use crate::longrunning::TaskStore;
use crate::proto::longrunning::Operation;

#[async_trait::async_trait]
pub trait OperationsStore: Send + Sync + Debug {
  async fn get(&self, id: &str, ctx: &RequestContext) -> Result<Operation, Error>;

  async fn set(&mut self, op: Operation, ctx: &RequestContext) -> Result<(), Error>;

  async fn delete(&mut self, id: &str, ctx: &RequestContext) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct RedisStore {
  inner: RedisTaskStore,
}

impl RedisStore {
  pub fn new(client: redis::Client) -> Self {
    Self { inner: RedisTaskStore::new(client) }
  }
}

#[async_trait::async_trait]
impl OperationsStore for RedisStore {
  async fn get(&self, id: &str, _ctx: &RequestContext) -> Result<Operation, Error> {
    let task_state: TaskState<serde_json::Value, TaskResult> = self.inner.get(id.to_string()).await?;
    Ok(task_state.into())
  }

  async fn set(&mut self, _op: Operation, _ctx: &RequestContext) -> Result<(), Error> {
    todo!()
  }

  async fn delete(&mut self, id: &str, _ctx: &RequestContext) -> Result<(), Error> {
    self.inner.remove(id.to_string()).await.map(|_| ())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  pub url: String,
}

#[derive(Debug, Clone)]
pub struct RedisTaskStore {
  client: redis::Client,
}

impl RedisTaskStore {
  pub fn new(client: redis::Client) -> Self {
    Self {
      client
    }
  }

  pub fn new_from_config(config: Config) -> Self {
    let client = redis::Client::open(config.url).unwrap();

    Self::new(client)
  }

  async fn get_task_state<T: Performable>(id: String, conn: &mut Connection) -> Result<TaskState<T, TaskResult>, Error> {
    let value = conn.hget(id, "task")
      .instrument(tracing::info_span!("redis-task-store:get")).await?;

    match value {
      redis::Value::Nil => Err(Error::NotFound),
      data => Ok(data.deserialize()?),
    }
  }
}

impl<T: Performable> ToRedisArgs for TaskState<T, TaskResult> {
  fn write_redis_args<W>(&self, out: &mut W) where W: ?Sized + RedisWrite {
    out.write_arg(serde_json::to_string(self).unwrap().as_bytes())
  }
}

impl<T: Performable> FromRedisValue for TaskState<T, TaskResult> {
  fn from_redis_value(v: &Value) -> RedisResult<Self> {
    let buf: String = redis::FromRedisValue::from_redis_value(v)?;

    serde_json::from_str(&buf).map_err(|error| {
      tracing::debug!(message = "Failed to decode value", %error);
      RedisError::from((
        redis::ErrorKind::IoError,
        "Deserialization failed",
        error.to_string())
      )
    })
  }
}

#[async_trait::async_trait]
impl super::TaskStore for RedisTaskStore {
  async fn get<T: Performable>(&self, id: String) -> Result<TaskState<T, TaskResult>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    Self::get_task_state(id, &mut conn).await
  }

  async fn start<T: Performable>(&self, id: String) -> Result<TaskState<T, TaskResult>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    match Self::get_task_state(id.clone(), &mut conn).await {
      Ok(mut task_state) => {
        task_state.started_at = Some(Utc::now().naive_utc());
        let _ = conn.hset(id.clone(), "task", task_state.clone())
          .instrument(tracing::info_span!("redis-task-store:set")).await?;
        Ok(task_state)
      }
      Err(error) => {
        tracing::debug!(message = "Missing task for id", %id, %error);
        Err(error)
      }
    }
  }

  async fn complete<T: Performable>(&self, id: String, result: TaskResult) -> Result<TaskState<T, TaskResult>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    match Self::get_task_state(id.clone(), &mut conn).await {
      Ok(mut task_state) => {
        task_state.processed_at = Some(Utc::now().naive_utc());
        task_state.result = Some(result);
        task_state.exit_code = 0;

        let q: String = conn.hget(id.clone(), "queue")
          .instrument(tracing::info_span!("redis-task-store:hget"))
          .await?;

        let _ = redis::pipe()
          .hset(id.clone(), "task", task_state.clone())
          .hdel(id.clone(), "queue")
          .lrem(format!("tasks-{}-running", q), 1, id.clone())
          .query_async(&mut conn)
          .instrument(tracing::info_span!("redis-task-store:complete")).await?;

        Ok(task_state)
      }
      Err(error) => {
        tracing::debug!(message = "Missing task for id", %id, %error);
        Err(error)
      }
    }
  }

  async fn fail<T: Performable>(&self, id: String, code: i32, msg: String) -> Result<TaskState<T, TaskResult>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    match Self::get_task_state(id.clone(), &mut conn).await {
      Ok(mut task_state) => {
        task_state.processed_at = Some(Utc::now().naive_utc());
        task_state.result = None;
        task_state.exit_code = code;
        task_state.message = msg;

        let q: String = conn.hget(id.clone(), "queue")
          .instrument(tracing::info_span!("redis-task-store:hget"))
          .await?;

        let _ = redis::pipe()
          .hset(id.clone(), "task", task_state.clone())
          .hdel(id.clone(), "queue")
          .lrem(format!("tasks-{}-running", q), 1, id.clone())
          .query_async(&mut conn)
          .instrument(tracing::info_span!("redis-task-store:complete")).await?;

        Ok(task_state)
      }
      Err(error) => {
        tracing::debug!(message = "Missing task for id", %id, %error);
        Err(error)
      }
    }
  }

  async fn enqueue<T: Performable>(&mut self, task: T, q: String) -> Result<String, Error> {
    let id = Uuid::new_v4().to_string();
    let mut conn = self.client.get_async_connection().await?;
    let task_state = TaskState::new(id.clone(), task);

    let _ = redis::pipe()
      .hset(id.clone(), "task", task_state)
      .hset(id.clone(), "queue", q.clone())
      .rpush(format!("tasks-{}-pending", q), id.clone())
      .query_async(&mut conn)
      .instrument(tracing::info_span!("redis-task-store:enqueue")).await?;

    Ok(id)
  }

  async fn dequeue<T: Performable>(&mut self, q: String) -> Result<TaskState<T, TaskResult>, Error> {
    let mut conn = self.client.get_async_connection().await?;
    let task_id: String = conn.brpoplpush(format!("tasks-{}-pending", q), format!("tasks-{}-running", q), 1)
      .instrument(tracing::info_span!("redis-task-store:brpoplpush"))
      .await?;

    Self::get_task_state(task_id.clone(), &mut conn).await
  }

  async fn remove(&mut self, id: String) -> Result<(), Error> {
    let mut conn = self.client.get_async_connection().await?;
    let _ = redis::pipe()
      .del(id.clone())
      .query_async(&mut conn)
      .instrument(tracing::info_span!("redis-task-store:del")).await?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use serde::de::DeserializeOwned;

  use crate::longrunning::{Context, TaskStore};

  use super::*;

  #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
  struct DefaultContext;

  #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
  struct ShutdownTask;

  #[async_trait::async_trait]
  impl Performable for ShutdownTask {
    async fn perform<C: Context>(&self, ctx: C) -> Result<(), Error> {
      Ok(())
    }
  }

  #[async_trait::async_trait]
  impl Context for DefaultContext {
    fn task_id(&self) -> &str {
      "task-id"
    }

    async fn failure<E: Send>(&mut self, error: E, msg: String) -> Result<(), Error> {
      Ok(())
    }

    async fn success<R: Serialize + DeserializeOwned + Send>(&mut self, result: R) -> Result<(), Error> {
      Ok(())
    }
  }

  #[tokio::test]
  async fn test_enqueue_task() {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let store = RedisTaskStore::new(client);

    let error: Result<TaskState<ShutdownTask, TaskResult>, Error> = store.get(String::from("non_existing_id")).await;

    if let Some(Error::NotFound) = error.err() {} else {
      panic!("Should new NotFound");
    }
  }
}
