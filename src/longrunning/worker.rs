use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;
use std::time::Duration;

use futures::executor::block_on;
use uuid::Uuid;

use crate::longrunning::{Broker, Error, Performable, State, TaskResult, TaskState, TaskStore, WorkerStore};
use crate::longrunning::store::{RedisTaskStore, RedisWorkerStore};

#[derive(Debug, Clone)]
pub struct DefaultContext {
  task_id: String,
  task_store: RedisTaskStore,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct DefaultWorker<T: Performable, R> {
  pub worker_id: String,
  pub queue: String,
  task_store: RedisTaskStore,
  worker_store: RedisWorkerStore,
  handle: Option<JoinHandle<()>>,
  running: AtomicBool,
  _phantom1: PhantomData<T>,
  _phantom2: PhantomData<R>,
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

impl<T: Performable, R: TaskResult> DefaultWorker<T, R> {
  pub fn new(queue: String, task_store: RedisTaskStore, worker_store: RedisWorkerStore) -> Self {
    Self {
      queue,
      task_store,
      worker_store,
      worker_id: Uuid::new_v4().to_string(),
      handle: None,
      running: AtomicBool::new(false),
      _phantom1: PhantomData,
      _phantom2: PhantomData,
    }
  }

  async fn run(queue: String, mut task_store: RedisTaskStore) {
    loop {
      match task_store.dequeue::<T, R>(queue.clone()).await {
        Ok(t) => {
          let task = t.task;
          let task_id = t.id;
          let ctx = DefaultContext::new(task_id.clone(), task_store.clone());
          match task.perform(ctx).await {
            Ok(_) => tracing::info!(message = "Task execution succeeded", %task_id),
            Err(error) => {
              tracing::error!(message = "Task execution failed", %task_id, %error);
              let _ = task_store.fail::<T, R>(task_id, 128, error.to_string()).await;
            }
          }
        }
        Err(error) => {
          tracing::error!(message = "Dequeue task failed", %error);
          std::thread::sleep(Duration::from_millis(1000));
        }
      }
    }
  }
}

impl<T: Performable, R: TaskResult> super::Worker for DefaultWorker<T, R> {
  fn start(&mut self) -> Result<(), Error> {
    let worker_id = self.worker_id.clone();
    let queue = self.queue.clone();
    let task_store = self.task_store.clone();
    tracing::debug!(message = "Starting worker", %worker_id, %queue);

    block_on(self.worker_store.register(worker_id.clone(), vec![queue.clone()]))?;
    let handle = std::thread::spawn(move || block_on(Self::run(queue, task_store)));

    self.handle = Some(handle);
    self.running.swap(true, Ordering::SeqCst);
    tracing::info!(message = "Started worker", %worker_id);
    Ok(())
  }

  fn join(&mut self) -> Result<(), Error> {
    match self.handle.take() {
      Some(h) => h.join().map(|_| {
        self.running.swap(false, Ordering::SeqCst);
      }).map_err(|error| {
        let error = Error::BoxError(error);
        tracing::error!(message = "Thread join failed", %error);
        error
      }),
      None => Err(Error::InvalidRequest("Worker not running")),
    }
  }
}

impl<T: Performable, R> Drop for DefaultWorker<T, R> {
  fn drop(&mut self) {
    let worker_id = self.worker_id.clone();
    tracing::info!(message = "Unregistering worker", %worker_id);

    let _ = block_on(self.worker_store.unregister(worker_id));
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
