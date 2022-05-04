use uuid::Uuid;

use crate::longrunning::{Broker, Error, Performable, State, TaskResult, TaskState, TaskStore};
use crate::longrunning::store::RedisTaskStore;

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
  async fn enqueue<T: Performable>(&mut self, task: T) -> Result<String, Error> {
    self.task_store.enqueue(task, self.queue.clone()).await
  }

  async fn get<T: Performable>(&self, id: String) -> Result<TaskState<T, TaskResult>, Error> {
    self.task_store.get(id).await
  }

  async fn cancel<T: Performable>(&mut self, id: String) -> Result<TaskState<T, TaskResult>, Error> {
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
