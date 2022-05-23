use std::collections::HashMap;
use std::marker::PhantomData;

use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing_futures::Instrument;
use uuid::Uuid;

use crate::codec::json::JsonCodec;
use crate::codec::Codec;
use crate::codec::Encoder;
use crate::proto::longrunning::Operation;

use super::Broker;
use super::Context;
use super::Performable;
use super::Queue;

#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
  #[error("Failed to enqueue the task: {0}")]
  QueueError(#[from] RedisQueueError),
}

#[derive(Clone, Debug)]
pub struct RedisBroker<T: Serialize + DeserializeOwned + Performable> {
  _client: redis::Client,
  queue: RedisQueue<T, JsonCodec<T, T>>,
  _phantom: PhantomData<T>,
}

impl<T: Send + Sync + Serialize + DeserializeOwned + Performable> RedisBroker<T> {
  pub fn new(client: redis::Client, queue_name: String) -> Self {
    Self {
      _client: client.clone(),
      queue: RedisQueue::new(client, queue_name, JsonCodec::new()),
      _phantom: PhantomData,
    }
  }
}

#[async_trait::async_trait]
impl<T: Performable> Broker<T> for RedisBroker<T>
where
  T: Send + Sync + Serialize + DeserializeOwned,
{
  type Error = BrokerError;

  async fn enqueue(&self, task: T, ctx: &Context) -> Result<Operation, Self::Error> {
    let id = self.queue.offer(task, ctx).await?;

    let operation = Operation {
      operation_id: id,
      metadata: HashMap::default(),
      done: false,
      error: None,
      response: HashMap::default(),
      creation_ts: None,
      start_ts: None,
      end_ts: None,
    };

    Ok(operation)
  }

  async fn cancel(&self, _id: &str, _ctx: &Context) -> Result<Operation, Self::Error> {
    todo!()
  }
}

#[derive(Clone, Debug)]
pub(crate) struct RedisQueue<T, C: Codec> {
  client: redis::Client,
  queue: String,
  codec: C,
  _phantom: PhantomData<T>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedisMessage<T> {
  ack_id: String,
  data: T,
}

#[derive(thiserror::Error, Debug)]
pub enum RedisQueueError {
  #[error("Redis command failed: {0}")]
  Redis(#[from] redis::RedisError),
}

impl<T> super::Task<T> for RedisMessage<T> {
  fn ack_id(&self) -> &str {
    &self.ack_id
  }

  fn data(&self) -> &T {
    &self.data
  }
}

impl<T: Performable, C: Codec> RedisQueue<T, C> {
  pub fn new(client: redis::Client, queue: String, codec: C) -> Self {
    Self {
      client,
      queue,
      codec,
      _phantom: PhantomData,
    }
  }
}

#[async_trait::async_trait]
impl<T: Send + Sync + Serialize + DeserializeOwned + Performable> super::Queue
  for RedisQueue<T, JsonCodec<T, T>>
{
  type Item = T;

  type ReceivedItem = RedisMessage<T>;

  type Error = RedisQueueError;

  async fn offer(&self, item: Self::Item, ctx: &Context) -> Result<String, Self::Error> {
    let mut encoder = self.codec.encoder();
    let id = Uuid::new_v4().to_string();
    let publish_ts = Utc::now().timestamp_nanos();

    let mut task = Vec::default();
    let _ = encoder.encode(&item, &mut task);
    let task = String::from_utf8_lossy(&task);

    let mut conn = self.client.get_async_connection().await?;

    let _ = redis::pipe()
      .atomic()
      .lpush(format!("queue:{}", self.queue), id.clone())
      .ignore()
      .hset_multiple(
        format!("operation:{}", id),
        &[
          ("status", "New"),
          ("queue", &self.queue),
          ("publish_ts", &publish_ts.to_string()),
          ("task", &task),
          ("user_id", ctx.user_id()),
          ("task_type", std::any::type_name::<Self::Item>()),
        ],
      )
      .ignore()
      .query_async(&mut conn)
      .instrument(tracing::info_span!("redis-queue-offer", operation_id=%id))
      .await?;

    Ok(id)
  }

  async fn pull(&self) -> Result<Option<Self::ReceivedItem>, Self::Error> {
    todo!()
  }

  async fn ack(&self, _ack_id: &str) -> Result<(), Self::Error> {
    todo!()
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use chrono::Utc;
  use redis::AsyncCommands;
  use serde::Deserialize;

  use crate::{longrunning::Queue, proto::google::protobuf::Empty};

  use super::*;

  #[derive(Serialize, Deserialize, Clone)]
  struct Task {
    item: i32,
  }

  #[async_trait::async_trait]
  impl Performable for Task {
    type Error = std::io::Error;
    type Context = ();
    type Output = Empty;

    async fn perform(&self, _: Self::Context) -> Result<Self::Output, Self::Error> {
      Ok(Empty::default())
    }
  }

  #[tokio::test]
  async fn offer_should_add_item_to_queue() {
    let ctx = Context::new(Uuid::new_v4().to_string());
    let queue = Uuid::new_v4().to_string();
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let q: RedisQueue<Task, JsonCodec<Task, Task>> =
      RedisQueue::new(client.clone(), queue.clone(), JsonCodec::new());
    let task = Task { item: 10 };

    let id = q.offer(task, &ctx).await.unwrap();

    let mut conn = client.get_async_connection().await.unwrap();
    let result: Vec<String> = conn
      .lrange(format!("queue:{}", queue), 0, -1)
      .await
      .unwrap();
    assert_eq!(vec![id], result);
  }

  #[tokio::test]
  async fn offer_should_set_metadata_while_adding_item_to_queue() {
    let queue = Uuid::new_v4().to_string();
    let ctx = Context::new(Uuid::new_v4().to_string());
    let ts = Utc::now().timestamp_nanos();
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let q: RedisQueue<Task, JsonCodec<Task, Task>> =
      RedisQueue::new(client.clone(), queue.clone(), JsonCodec::new());
    let task = Task { item: 10 };

    let id = q.offer(task, &ctx).await.unwrap();

    let mut conn = client.get_async_connection().await.unwrap();

    let result: HashMap<String, String> = conn.hgetall(format!("operation:{}", id)).await.unwrap();

    assert_eq!(result["status"], "New");
    assert!(result["publish_ts"].parse::<i64>().unwrap() >= ts);
    assert_eq!(
      result["task_type"],
      "rappel::longrunning::redis::tests::Task"
    );
    assert_eq!(result["task"], "{\"item\":10}");
    assert_eq!(result["queue"], queue);
    assert_eq!(result["user_id"], ctx.user_id());
  }

  #[tokio::test]
  async fn should_enqueue_task_to_broker() {
    let ctx = Context::new(Uuid::new_v4().to_string());
    let queue = Uuid::new_v4().to_string();
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let q: RedisBroker<Task> = RedisBroker::new(client.clone(), queue.clone());
    let task = Task { item: 10 };

    let operation = q.enqueue(task, &ctx).await.unwrap();

    let mut conn = client.get_async_connection().await.unwrap();
    let result: Vec<String> = conn
      .lrange(format!("queue:{}", queue), 0, -1)
      .await
      .unwrap();
    assert_eq!(vec![operation.operation_id], result);
  }
}
