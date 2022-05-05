use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;
use std::time::Duration;

use futures::executor::block_on;
use uuid::Uuid;

use crate::longrunning::{Broker, Context, Error, Performable, State, TaskResult, TaskState, TaskStore, Worker, WorkerStore};
use crate::longrunning::store::{RedisTaskStore, RedisWorkerStore};

#[derive(Debug, Clone)]
pub struct DefaultContext {
  task_id: String,
  task_store: RedisTaskStore,
}

pub trait ContextProvider<T: Context>: Clone + Send + Sync + Sized {
  fn get(&self, task_id: String) -> T;
}

#[derive(Clone, Debug)]
pub struct DefaultContextProvider(RedisTaskStore);

impl ContextProvider<DefaultContext> for DefaultContextProvider {
  fn get(&self, task_id: String) -> DefaultContext {
    DefaultContext::new(task_id, self.0.clone())
  }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct DefaultWorker<C, T, R, P>
  where
    C: Context,
    T: Performable<C>,
    R: TaskResult,
    P: 'static + ContextProvider<C>
{
  pub worker_id: String,
  pub queue: String,
  task_store: RedisTaskStore,
  worker_store: RedisWorkerStore,
  handle: Option<JoinHandle<()>>,
  running: Arc<AtomicBool>,
  ctx_provider: Arc<P>,
  _phantom1: PhantomData<T>,
  _phantom2: PhantomData<R>,
  _phantom3: PhantomData<C>,
}

#[derive(Debug, Clone)]
pub struct DefaultBroker<C: Context, T: Performable<C>, R: TaskResult> {
  task_store: RedisTaskStore,
  queue: String,
  _phantom1: PhantomData<T>,
  _phantom2: PhantomData<R>,
  _phantom3: PhantomData<C>,
}

impl DefaultContext {
  pub fn new(task_id: String, task_store: RedisTaskStore) -> Self {
    Self {
      task_id,
      task_store,
    }
  }
}

impl<C, T, R, P> DefaultWorker<C, T, R, P>
  where
    T: Performable<C>,
    R: TaskResult,
    C: Context,
    P: 'static + ContextProvider<C>
{
  pub fn new(queue: String, task_store: RedisTaskStore, worker_store: RedisWorkerStore, ctx_provider: P) -> Self {
    Self {
      queue,
      ctx_provider: Arc::new(ctx_provider),
      task_store,
      worker_store,
      worker_id: format!("workers/{}", Uuid::new_v4()),
      handle: None,
      running: Arc::new(AtomicBool::new(false)),
      _phantom1: PhantomData,
      _phantom2: PhantomData,
      _phantom3: PhantomData,
    }
  }

  async fn run(queue: String, mut task_store: RedisTaskStore, token: Arc<AtomicBool>, ctx_provider: Arc<P>) {
    while token.load(Ordering::SeqCst) {
      match task_store.dequeue::<C, T, R>(queue.clone()).await {
        Ok(t) => {
          let task = t.task;
          let task_id = t.id;
          let context = ctx_provider.get(task_id.clone());
          match task.perform(context).await {
            Ok(_) => tracing::info!(message = "Task execution succeeded", % task_id),
            Err(error) => {
              tracing::error!(message = "Task execution failed", %task_id, % error);
              let _ = task_store.fail::<C, T, R>(task_id, 128, error.to_string()).await;
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

#[async_trait::async_trait]
impl super::Context for DefaultContext {
  fn task_id(&self) -> &str {
    &self.task_id
  }

  async fn failure<T: Performable<Self>, R: TaskResult>(&mut self, result: R, msg: String) -> Result<(), Error> {
    let code = match result.as_any().downcast_ref::<i32>() {
      Some(i) => *i,
      None => 1
    };

    self.task_store.fail::<Self, T, R>(self.task_id.clone(), code, msg).await.map(|_| ())
  }

  async fn success<T: Performable<Self>, R: TaskResult>(&mut self, result: R) -> Result<(), Error> {
    self.task_store.complete::<Self, T, R>(self.task_id.clone(), result).await.map(|_| ())
  }
}

impl<C: Context, T: Performable<C>, R: TaskResult, P: 'static + ContextProvider<C>> super::Worker for DefaultWorker<C, T, R, P> {
  fn start(&mut self) -> Result<(), Error> {
    let worker_id = self.worker_id.clone();
    let queue = self.queue.clone();
    let task_store = self.task_store.clone();
    let token = self.running.clone();
    let ctx_provider = self.ctx_provider.clone();
    tracing::debug!(message = "Starting worker", %worker_id, %queue);

    block_on(self.worker_store.register(worker_id.clone(), vec![queue.clone()]))?;
    let handle = std::thread::spawn(move || block_on(Self::run(queue, task_store, token, ctx_provider)));

    self.handle = Some(handle);
    self.running.swap(true, Ordering::SeqCst);
    tracing::info!(message = "Started worker", %worker_id);
    Ok(())
  }

  fn stop(&mut self) -> Result<(), Error> {
    self.running.swap(false, Ordering::SeqCst);
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

impl<C: Context, T: Performable<C>, R: TaskResult, P: ContextProvider<C>> Drop for DefaultWorker<C, T, R, P> {
  fn drop(&mut self) {
    let worker_id = self.worker_id.clone();
    tracing::info!(message = "Unregistering worker", %worker_id);

    let _ = block_on(self.worker_store.unregister(worker_id));
    let _ = self.stop();
  }
}

impl<C: Context, T: Performable<C>, R: TaskResult> DefaultBroker<C, T, R> {
  pub fn new(queue: String, task_store: RedisTaskStore) -> Self {
    Self {
      queue,
      task_store,
      _phantom1: PhantomData,
      _phantom2: PhantomData,
      _phantom3: PhantomData,
    }
  }
}

#[async_trait::async_trait]
impl<C: Context, T: Performable<C>, R: TaskResult> Broker<C, T, R> for DefaultBroker<C, T, R> {
  async fn enqueue(&mut self, task: T) -> Result<TaskState<C, T, R>, Error> {
    self.task_store.enqueue(task, self.queue.clone()).await
  }

  async fn get(&self, id: String) -> Result<TaskState<C, T, R>, Error> {
    self.task_store.get(id).await
  }

  async fn cancel(&mut self, id: String) -> Result<TaskState<C, T, R>, Error> {
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
