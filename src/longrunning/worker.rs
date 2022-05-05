use uuid::Uuid;

use crate::longrunning::{Broker, Error, Performable, State, TaskResult, TaskState, TaskStore};
use crate::longrunning::store::RedisTaskStore;

#[derive(Debug, Clone)]
pub struct DefaultContext {
  task_id: String,
  task_store: RedisTaskStore,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct DefaultWorker {
  pub worker_id: String,
  pub queue: String,
  task_store: RedisTaskStore,
}

#[derive(Debug, Clone)]
pub struct DefaultBroker {
  task_store: RedisTaskStore,
  queue: String,
}

impl DefaultContext {
  pub fn new(task_id: String, task_store: RedisTaskStore) -> Self {
    Self {
      task_id,
      task_store,
    }
  }
}

#[async_trait::async_trait]
impl super::Context for DefaultContext {
  fn task_id(&self) -> &str {
    &self.task_id
  }

  async fn failure<T: Performable, R: TaskResult>(&mut self, result: R, msg: String) -> Result<(), Error> {
    let code = match result.as_any().downcast_ref::<i32>() {
      Some(i) => *i,
      None => 1
    };

    self.task_store.fail::<T, R>(self.task_id.clone(), code, msg).await.map(|_| ())
  }

  async fn success<T: Performable, R: TaskResult>(&mut self, result: R) -> Result<(), Error> {
    self.task_store.complete::<T, R>(self.task_id.clone(), result).await.map(|_| ())
  }
}


impl DefaultWorker {
  pub fn new(queue: String, task_store: RedisTaskStore) -> Self {
    Self {
      queue,
      task_store,
      worker_id: Uuid::new_v4().to_string(),
    }
  }
}

#[async_trait::async_trait]
impl super::Worker for DefaultWorker {
  async fn start(&mut self) -> Result<(), Error> {
    tracing::info!(message = "Starting worker", %self.worker_id);
    todo!()
  }

  async fn stop(&mut self) -> Result<(), Error> {
    todo!()
  }
}

impl DefaultBroker {
  pub fn new(queue: String, task_store: RedisTaskStore) -> Self {
    Self {
      queue,
      task_store,
    }
  }
}

#[async_trait::async_trait]
impl Broker for DefaultBroker {
  async fn enqueue<T: Performable, R: TaskResult>(&mut self, task: T) -> Result<TaskState<T, R>, Error> {
    self.task_store.enqueue(task, self.queue.clone()).await
  }

  async fn get<T: Performable, R: TaskResult>(&self, id: String) -> Result<TaskState<T, R>, Error> {
    self.task_store.get(id).await
  }

  async fn cancel<T: Performable, R: TaskResult>(&mut self, id: String) -> Result<TaskState<T, R>, Error> {
    match self.task_store.get(id.clone()).await {
      Ok(t) if t.state != State::Running => {
        let _ = self.task_store.remove(id.clone()).await?;
        Ok(t)
      }
      Ok(_) => {
        tracing::debug!(message = "Cannot cancel a running task", %id);
        Err(Error::InvalidRequest("Already running"))
      }
      Err(error) => {
        tracing::debug!(message = "Get task failed", %id, %error);
        Err(error)
      }
    }
  }
}
