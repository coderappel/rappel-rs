use prost::Message;

use crate::proto::google::rpc::Status;
use crate::proto::longrunning::Operation;

#[async_trait::async_trait]
pub trait Performable {
  type Error;

  type Context;

  type Output: Message;

  async fn perform(&self, ctx: Self::Context) -> Result<Self::Output, Self::Error>;
}

#[async_trait::async_trait]
pub trait Broker<P: Performable> {
  type Error;

  async fn enqueue(&self, task: P, ctx: &Context) -> Result<Operation, Self::Error>;

  async fn cancel(&self, id: &str, ctx: &Context) -> Result<Operation, Self::Error>;
}

pub trait Task<T> {
  fn ack_id(&self) -> &str;

  fn data(&self) -> &T;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
  user_id: String,
}

impl Context {
  pub fn new(user_id: String) -> Self {
    Self { user_id }
  }

  pub fn user_id(&self) -> &str {
    &self.user_id
  }
}

#[async_trait::async_trait]
pub(crate) trait Queue {
  type Item: Performable;

  type ReceivedItem: Task<Self::Item>;

  type Error;

  async fn offer(&self, item: Self::Item, ctx: &Context) -> Result<String, Self::Error>;

  async fn pull(&self) -> Result<Option<Self::ReceivedItem>, Self::Error>;

  async fn ack(&self, ack_id: &str) -> Result<(), Self::Error>;
}

#[async_trait::async_trait]
pub trait Performer<P: Performable> {
  type Error: Into<Status>;

  fn worker_id(&self) -> &str;

  async fn perform(&mut self, task: P) -> Result<(), Self::Error>;
}

pub trait TaskLoop {
  fn start(&self);

  fn abort(&self);

  fn await_termination(&self);
}
