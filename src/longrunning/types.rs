use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

use chrono::{NaiveDateTime, Timelike};
use chrono::Utc;
use prost_types::Timestamp;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use crate::proto::longrunning::Operation;

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("Read error")]
  RedisError(#[from] redis::RedisError),

  #[error("Decode error")]
  SerdeRedisDecodeError(#[from] serde_redis::decode::Error),

  #[error("Deserialization failed")]
  ProstDecodeError(#[from] prost::DecodeError),

  #[error("Serialization failed")]
  ProstEncodeError(#[from] prost::EncodeError),

  #[error("Internal Error")]
  Internal(#[from] anyhow::Error),

  #[error("{0}")]
  InvalidRequest(&'static str),

  #[error("not found")]
  NotFound,

  #[error("unknown error")]
  Unknown,
}

#[async_trait::async_trait]
pub trait Context: Send {
  fn task_id(&self) -> &str;

  async fn failure<T: Performable, R: TaskResult>(&mut self, result: R, msg: String) -> Result<(), Error>;

  async fn success<T: Performable, R: TaskResult>(&mut self, result: R) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait Performable: Serialize + DeserializeOwned + Send + Sync + Sized + Clone + Any {
  async fn perform<C: Context>(&self, ctx: C) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl Performable for serde_json::Value {
  async fn perform<C: Context>(&self, _ctx: C) -> Result<(), Error> {
    panic!("method should never be invoked")
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum State {
  Unknown = 0,
  Waiting = 1,
  Running = 2,
  Terminating = 3,
  Terminated = 4,
}

pub trait TaskResult: Debug + Clone + Send + Sync + Serialize + DeserializeOwned + Any {
  fn as_any(&self) -> &dyn Any;
}

impl <T: Debug + Clone + Send + Sync + Serialize + DeserializeOwned + Any>TaskResult for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState<T, R> {
  pub id: String,
  pub task: T,
  pub result: Option<R>,
  pub state: State,
  pub exit_code: i32,
  pub message: String,
  pub created_at: NaiveDateTime,
  pub started_at: Option<NaiveDateTime>,
  pub processed_at: Option<NaiveDateTime>,
}

impl<T: Performable, R> TaskState<T, R> {
  pub fn new(id: String, task: T) -> Self {
    Self {
      id,
      task,
      result: None,
      state: State::Unknown,
      exit_code: -1,
      message: String::default(),
      created_at: Utc::now().naive_utc(),
      started_at: None,
      processed_at: None,
    }
  }
}

#[async_trait::async_trait]
pub trait TaskStore {
  async fn get<T: Performable, R: TaskResult>(&self, id: String) -> Result<TaskState<T, R>, Error>;

  async fn start<T: Performable, R: TaskResult>(&self, id: String) -> Result<TaskState<T, R>, Error>;

  async fn complete<T: Performable, R: TaskResult>(&self, id: String, result: R) -> Result<TaskState<T, R>, Error>;

  async fn fail<T: Performable, R: TaskResult>(&self, id: String, code: i32, msg: String) -> Result<TaskState<T, R>, Error>;

  async fn enqueue<T: Performable, R: TaskResult>(&mut self, task: T, q: String) -> Result<TaskState<T, R>, Error>;

  async fn dequeue<T: Performable, R: TaskResult>(&mut self, q: String) -> Result<TaskState<T, R>, Error>;

  async fn remove(&mut self, id: String) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait Broker {
  async fn enqueue<T: Performable, R: TaskResult>(&mut self, task: T) -> Result<TaskState<T, R>, Error>;

  async fn get<T: Performable, R: TaskResult>(&self, id: String) -> Result<TaskState<T, R>, Error>;

  async fn cancel<T: Performable, R: TaskResult>(&mut self, id: String) -> Result<TaskState<T, R>, Error>;
}

#[async_trait::async_trait]
pub trait Worker {
  async fn start(&mut self) -> Result<(), Error>;

  async fn stop(&mut self) -> Result<(), Error>;
}

impl<T: Serialize, R> Into<Operation> for TaskState<T, R> {
  fn into(self) -> Operation {
    let task = &self.task;

    Operation {
      operation_id: self.id,
      done: self.exit_code >= 0,
      metadata: HashMap::from([
        ("task".to_string(), serde_json::to_string(task).unwrap())
      ]),
      creation_ts: Some(Timestamp {
        seconds: self.created_at.timestamp(),
        nanos: self.created_at.nanosecond() as i32,
      }),
      start_ts: self.started_at.map(|ts| Timestamp {
        seconds: ts.timestamp(),
        nanos: ts.nanosecond() as i32,
      }),
      end_ts: self.processed_at.map(|ts| Timestamp {
        seconds: ts.timestamp(),
        nanos: ts.nanosecond() as i32,
      }),
      result: None,
    }
  }
}
