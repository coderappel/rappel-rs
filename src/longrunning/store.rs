use std::fmt::Debug;

use chrono::Utc;
#[allow(unused_imports)]
use prost::Message;
use redis::aio::Connection;
use redis::AsyncCommands;
use redis::ErrorKind::TypeError;
use redis::FromRedisValue;
use redis::RedisError;
use redis::RedisResult;
use redis::RedisWrite;
use redis::ToRedisArgs;
use redis::Value;
use serde::Deserialize;
use serde::Serialize;
use tracing::Instrument;
use uuid::Uuid;

use crate::grpc::RequestContext;
use crate::longrunning::{Context, DefaultContext, Error, State};
use crate::longrunning::Performable;
use crate::longrunning::TaskResult;
use crate::longrunning::TaskState;
use crate::longrunning::TaskStore;
use crate::longrunning::WorkerStore;
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
    let task_state: TaskState<DefaultContext, serde_json::Value, serde_json::Value> = self.inner.get(id.to_string()).await?;
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

  async fn get_task_state<C: Context, T: Performable<C>, R: TaskResult>(id: String, conn: &mut Connection) -> Result<TaskState<C,T, R>, Error> {
    let response = conn.hget(id, "task")
      .instrument(tracing::info_span!("redis-task-store:get")).await
      .map_err(|error| error.into());

    match response {
      Err(Error::RedisError(err)) if err.kind() == TypeError => Err(Error::NotFound),
      Err(error) => Err(error),
      Ok(t) => Ok(t),
    }
  }
}

impl<C: Context,T: Performable<C>, R: TaskResult> ToRedisArgs for TaskState<C,T, R> {
  fn write_redis_args<W>(&self, out: &mut W) where W: ?Sized + RedisWrite {
    out.write_arg(serde_json::to_string(self).unwrap().as_bytes())
  }
}

impl<C: Context,T: Performable<C>, R: TaskResult> FromRedisValue for TaskState<C,T, R> {
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
  async fn get<C: Context,T: Performable<C>, R: TaskResult>(&self, id: String) -> Result<TaskState<C,T, R>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    Self::get_task_state(id, &mut conn).await
  }

  async fn start<C: Context,T: Performable<C>, R: TaskResult>(&self, id: String) -> Result<TaskState<C,T, R>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    match Self::get_task_state(id.clone(), &mut conn).await {
      Ok(mut task_state) => {
        task_state.state = State::Running;
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

  async fn complete<C: Context,T: Performable<C>, R: TaskResult>(&self, id: String, result: R) -> Result<TaskState<C,T, R>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    match Self::get_task_state(id.clone(), &mut conn).await {
      Ok(mut task_state) => {
        task_state.processed_at = Some(Utc::now().naive_utc());
        task_state.result = Some(result);
        task_state.state = State::Terminated;
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

  async fn fail<C: Context,T: Performable<C>, R: TaskResult>(&self, id: String, code: i32, msg: String) -> Result<TaskState<C,T, R>, Error> {
    let mut conn = self.client.get_async_connection().await?;

    match Self::get_task_state(id.clone(), &mut conn).await {
      Ok(mut task_state) => {
        task_state.processed_at = Some(Utc::now().naive_utc());
        task_state.result = None;
        task_state.exit_code = code;
        task_state.message = msg;
        task_state.state = State::Terminated;

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

  async fn enqueue<C: Context,T: Performable<C>, R: TaskResult>(&mut self, task: T, q: String) -> Result<TaskState<C,T, R>, Error> {
    let id = format!("tasks/{}", Uuid::new_v4());
    let mut conn = self.client.get_async_connection().await?;
    let mut task_state: TaskState<C,T, R> = TaskState::new(id.clone(), task);
    task_state.state = State::Waiting;

    let _ = redis::pipe()
      .hset(id.clone(), "task", task_state.clone())
      .hset(id.clone(), "queue", q.clone())
      .rpush(format!("tasks-{}-pending", q), id.clone())
      .query_async(&mut conn)
      .instrument(tracing::info_span!("redis-task-store:enqueue")).await?;

    Ok(task_state)
  }

  async fn dequeue<C: Context,T: Performable<C>, R: TaskResult>(&mut self, q: String) -> Result<TaskState<C,T, R>, Error> {
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

#[derive(Debug, Clone)]
pub struct RedisWorkerStore {
  client: redis::Client,
}

impl RedisWorkerStore {
  pub fn new(client: redis::Client) -> Self {
    Self {
      client
    }
  }
}

#[async_trait::async_trait]
impl WorkerStore for RedisWorkerStore {
  async fn register(&mut self, worker_id: String, queue: Vec<String>) -> Result<(), Error> {
    let mut conn = self.client.get_async_connection().await?;
    let _ = redis::pipe()
      .hset(format!("worker/{}", worker_id), "queue", queue.join(","))
      .hset(format!("worker/{}", worker_id), "heartbeat", Utc::now().timestamp())
      .query_async(&mut conn)
      .instrument(tracing::info_span!("redis-worker-store:register")).await?;

    Ok(())
  }

  async fn heartbeat(&mut self, worker_id: String) -> Result<(), Error> {
    let mut conn = self.client.get_async_connection().await?;
    let _ = conn.hset(format!("worker/{}", worker_id), "heartbeat", Utc::now().timestamp())
      .instrument(tracing::info_span!("redis-worker-store:heartbeat")).await?;

    Ok(())
  }

  async fn unregister(&mut self, worker_id: String) -> Result<(), Error> {
    let mut conn = self.client.get_async_connection().await?;
    let _ = conn.del(format!("worker/{}", worker_id))
      .instrument(tracing::info_span!("redis-worker-store:unregister")).await?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::longrunning::{Context, State};
  use crate::longrunning::TaskStore;

  use super::*;

  #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
  struct DefaultContext;

  #[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
  struct ShutdownTask {
    id: String,
  }

  impl Default for ShutdownTask {
    fn default() -> Self {
      Self {
        id: Uuid::new_v4().to_string()
      }
    }
  }

  #[async_trait::async_trait]
  impl Performable<DefaultContext> for ShutdownTask {
    async fn perform(&self, _ctx: DefaultContext) -> Result<(), Error> {
      Ok(())
    }
  }

  #[allow(unused_variables)]
  #[async_trait::async_trait]
  impl Context for DefaultContext {
    fn task_id(&self) -> &str {
      "task-id"
    }

    async fn failure<C: Context, T: Performable<C>, R: TaskResult>(&mut self, result: R, msg: String) -> Result<(), Error> {
      Ok(())
    }

    async fn success<C: Context, T: Performable<C>, R: TaskResult>(&mut self, result: R) -> Result<(), Error> {
      Ok(())
    }
  }

  #[tokio::test]
  async fn test_get_non_existing_task() {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let store = RedisTaskStore::new(client);

    let error: Result<TaskState<DefaultContext, ShutdownTask, serde_json::Value>, Error> = store.get(String::from("non_existing_id")).await;

    if let Some(Error::NotFound) = error.err() {} else {
      panic!("Should not be Found");
    }
  }

  #[tokio::test]
  async fn test_enqueue_task() {
    let queue = Uuid::new_v4().to_string();
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut store = RedisTaskStore::new(client.clone());
    let task = ShutdownTask::default();

    let task_state = store.enqueue::<DefaultContext, ShutdownTask, serde_json::Value>(task.clone(), queue.clone()).await.unwrap();
    assert_eq!(task_state.result, None);
    assert_eq!(task_state.state, State::Waiting);
    assert_eq!(task_state.exit_code, -1);
    assert_eq!(task_state.task, task);

    let mut conn = client.get_async_connection().await.unwrap();
    let queued: String = conn.hget(task_state.id, "queue").await.unwrap();

    assert_eq!(queue, queued);

    let task_state_deq = store.dequeue::<DefaultContext, ShutdownTask, serde_json::Value>(queue.clone()).await.unwrap();
    assert_eq!(task, task_state_deq.task);
  }

  #[tokio::test]
  async fn test_complete_task() {
    let queue = Uuid::new_v4().to_string();
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut store = RedisTaskStore::new(client.clone());
    let task = ShutdownTask::default();

    let task_state = store.enqueue::<DefaultContext, ShutdownTask, i32>(task.clone(), queue.clone()).await.unwrap();
    assert_eq!(task_state.result, None);
    assert_eq!(task_state.state, State::Waiting);

    let _ = store.complete::<DefaultContext, ShutdownTask, i32>(task_state.id.clone(), 99).await.unwrap();

    let mut conn = client.get_async_connection().await.unwrap();
    let t: TaskState<DefaultContext, ShutdownTask, i32> = conn.hget(task_state.id, "task").await.unwrap();

    assert_eq!(t.result, Some(99));
    assert_eq!(t.exit_code, 0);
    assert_eq!(t.state, State::Terminated);
  }

  #[tokio::test]
  async fn test_fail_task() {
    let queue = Uuid::new_v4().to_string();
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut store = RedisTaskStore::new(client.clone());
    let task = ShutdownTask::default();

    let task_state = store.enqueue::<DefaultContext, ShutdownTask, i32>(task.clone(), queue.clone()).await.unwrap();
    assert_eq!(task_state.result, None);
    assert_eq!(task_state.state, State::Waiting);

    let _ = store.fail::<DefaultContext, ShutdownTask, i32>(task_state.id.clone(), 99, "Failed".to_string()).await.unwrap();

    let mut conn = client.get_async_connection().await.unwrap();
    let t: TaskState<DefaultContext, ShutdownTask, i32> = conn.hget(task_state.id, "task").await.unwrap();

    assert_eq!(t.result, None);
    assert_eq!(t.exit_code, 99);
    assert_eq!(t.message, "Failed");
    assert_eq!(t.state, State::Terminated);
  }
}
