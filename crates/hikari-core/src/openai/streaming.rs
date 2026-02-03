use crate::openai::Message;
use crate::openai::error::StreamingError;
use core::fmt;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;

pub type BoxedStream = Pin<Box<dyn Stream<Item = Result<Message, StreamingError>> + Send>>;

// We need this wrapper type because we need the stream to be clonable and shareable across threads.
pub type MessageStream = ArcMutexStream<BoxedStream>;

pub struct ArcMutexStream<S>
where
    S: Stream + Unpin,
{
    pub inner: Arc<Mutex<S>>,
}

impl<S> fmt::Debug for ArcMutexStream<S>
where
    S: Stream + Unpin,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArcMutexStream(...)")
    }
}

impl<S> Clone for ArcMutexStream<S>
where
    S: Stream + Unpin,
{
    fn clone(&self) -> Self {
        ArcMutexStream {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<S> ArcMutexStream<S>
where
    S: Stream + Unpin,
{
    pub fn new(stream: S) -> Self {
        ArcMutexStream {
            inner: Arc::new(Mutex::new(stream)),
        }
    }
}

impl<S> Stream for ArcMutexStream<S>
where
    S: Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Ok(mut inner) = self.get_mut().inner.try_lock() else {
            return Poll::Pending; // If lock is not available, return Pending
        };
        (*inner).poll_next_unpin(cx)
    }
}
