use std::error::Error;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task;

pub struct Refresh<T> {
    pub(crate) value: T,
    valid_until: Instant,
    not_before: Instant,
}

impl<T> Refresh<T> {
    pub fn new(value: T, wait: Duration, leeway: Duration) -> Self {
        let not_before = Instant::now() + wait;
        let valid_until = not_before + leeway;
        Self {
            value,
            valid_until,
            not_before,
        }
    }
}

pub trait Refresher {
    type Error;
    type Output;
    type Future: Future<Output = Result<Refresh<Self::Output>, Self::Error>> + Send + Sync;

    fn refresh(&self) -> Self::Future;
}

pub struct RefreshableValue<T, R, E>
where
    R: Refresher<Output = T, Error = E> + Send + Sync,
{
    value: Arc<RwLock<Arc<Refresh<T>>>>,
    refresher: R,
    active_refresh: Arc<Semaphore>,
}

impl<T, R, E> RefreshableValue<T, R, E>
where
    R: Refresher<Output = T, Error = E> + Send + Sync,
{
    pub fn should_refresh(&self) -> bool {
        let now = Instant::now();
        let value = self.value.read().expect("poisoned lock");
        value.not_before <= now
    }

    pub fn valid(&self) -> bool {
        let now = Instant::now();
        let value = self.value.read().expect("poisoned lock");
        value.valid_until <= now
    }

    pub fn get_unchecked(&self) -> Arc<Refresh<T>> {
        let value = self.value.read().expect("poisoned lock");
        Arc::clone(&value)
    }
}

impl<T, R, E> RefreshableValue<T, R, E>
where
    T: Sync + Send + 'static,
    R: Refresher<Output = T, Error = E> + Send + Sync,
    E: Error + 'static,
{
    pub async fn new(refresher: R) -> Result<Self, E> {
        refresher.refresh().await.map(|value| Self {
            value: Arc::new(RwLock::new(Arc::new(value))),
            refresher,
            active_refresh: Arc::new(Semaphore::new(1)),
        })
    }
}

impl<T, R, E> RefreshableValue<T, R, E>
where
    R: Refresher<Output = T, Error = E> + Send + Sync,
    E: Error + 'static,
    T: Send + Sync + 'static,
{
    pub fn refresh(&self) -> bool
    where
        <R as Refresher>::Future: 'static,
    {
        let Ok(permit) = Arc::clone(&self.active_refresh).try_acquire_owned() else {
            return false;
        };
        let value = self.value.read().expect("poisoned lock");
        if Instant::now() > value.not_before {
            return false;
        }
        drop(value);
        let value = Arc::clone(&self.value);
        let refresh_future = self.refresher.refresh();
        let update_future = async move {
            let refresh = match refresh_future.await {
                Ok(refresh) => refresh,
                Err(error) => {
                    tracing::error!(error = &error as &dyn Error, "refresh failed");
                    return;
                }
            };
            let mut value = value.write().expect("poisoned lock");
            *value = Arc::new(refresh);
            // explicitly drop the permit so it is moved into the future and not dropped at the
            // end of the surrounding function
            drop(permit);
        };
        task::spawn(update_future);
        true
    }
}
