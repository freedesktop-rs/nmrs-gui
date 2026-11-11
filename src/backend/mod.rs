mod demo;
mod nm;

use std::future::Future;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::channel::{mpsc, oneshot};
use futures::future::BoxFuture;

use crate::model::{Action, ProfileDetail, State};

pub trait Service: Send + Sync + 'static {
    fn state(&self) -> BoxFuture<'static, Result<State>>;
    fn profile(&self, uuid: String) -> BoxFuture<'static, Result<ProfileDetail>>;
    fn execute(&self, action: Action) -> BoxFuture<'static, Result<()>>;
    fn watch(&self, tx: mpsc::UnboundedSender<()>) -> BoxFuture<'static, Result<()>>;
}

#[derive(Clone)]
pub struct Backend {
    service: Arc<dyn Service>,
    runtime: tokio::runtime::Handle,
    pub demo: bool,
}

impl Backend {
    pub fn start(demo: bool) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("nmrs-backend")
            .enable_all()
            .build()?;
        let handle = runtime.handle().clone();
        std::thread::Builder::new()
            .name("nmrs-runtime".into())
            .spawn(move || runtime.block_on(futures::future::pending::<()>()))?;

        let service: Arc<dyn Service> = if demo {
            Arc::new(demo::Demo::new())
        } else {
            Arc::new(nm::Nm::new())
        };

        Ok(Self {
            service,
            runtime: handle,
            demo,
        })
    }

    fn spawn<T: Send + 'static>(
        &self,
        fut: BoxFuture<'static, Result<T>>,
    ) -> impl Future<Output = Result<T>> + use<T> {
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let _ = tx.send(fut.await);
        });
        async move { rx.await.unwrap_or_else(|_| Err(anyhow!("backend stopped"))) }
    }

    pub fn state(&self) -> impl Future<Output = Result<State>> + use<> {
        self.spawn(self.service.state())
    }

    pub fn profile(&self, uuid: String) -> impl Future<Output = Result<ProfileDetail>> + use<> {
        self.spawn(self.service.profile(uuid))
    }

    pub fn execute(&self, action: Action) -> impl Future<Output = Result<()>> + use<> {
        self.spawn(self.service.execute(action))
    }

    pub fn dispatch(&self, action: Action) {
        let label = action.describe();
        let fut = self.service.execute(action);
        self.runtime.spawn(async move {
            if let Err(err) = fut.await {
                log::warn!("{label} failed: {err}");
            }
        });
    }

    pub fn watch(&self) -> mpsc::UnboundedReceiver<()> {
        let (tx, rx) = mpsc::unbounded();
        let fut = self.service.watch(tx);
        self.runtime.spawn(async move {
            if let Err(err) = fut.await {
                log::warn!("event stream ended: {err}");
            }
        });
        rx
    }
}
