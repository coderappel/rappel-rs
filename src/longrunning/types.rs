use crate::proto::longrunning::Operation;

#[async_trait::async_trait]
pub trait Performable<S, C> {
  type Error;

  async fn perform(&self, ctx: C) -> Result<S, Self::Error>;
}

#[async_trait::async_trait]
pub trait Broker<P> {
  type Error;

  async fn enqueue(&self, task: P, ctx: &Context) -> Result<Operation, Self::Error>;

  async fn cancel(&self, id: &str, ctx: &Context) -> Result<Operation, Self::Error>;
}

pub trait Message<T> {
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
  type Item;

  type ReceivedItem: Message<Self::Item>;

  type Error;

  async fn offer(&self, item: Self::Item, ctx: &Context) -> Result<String, Self::Error>;

  async fn pull(&self) -> Result<Option<Self::ReceivedItem>, Self::Error>;

  async fn ack(&self, ack_id: &str) -> Result<(), Self::Error>;
}

#[async_trait::async_trait]
pub trait Performer<S, C, P>
where
  P: Performable<S, C>,
{
  type Error;

  fn worker_id(&self) -> &str;

  async fn perform(&mut self, task: P) -> Result<(), Self::Error>;
}

pub trait TaskLoop {
  fn start(&self);

  fn abort(&self);

  fn await_termination(&self);
}
