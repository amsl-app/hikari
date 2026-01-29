use crate::openai::Message;
use crate::openai::error::StreamingError;
use futures::{Stream, StreamExt};
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type BoxedStream = Pin<Box<dyn Stream<Item = Result<Message, StreamingError>> + Send>>;
pub struct MessageStream(Arc<Mutex<BoxedStream>>);

impl fmt::Debug for MessageStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageStream(...)")
    }
}

impl Clone for MessageStream {
    fn clone(&self) -> Self {
        MessageStream(Arc::clone(&self.0))
    }
}

impl MessageStream {
    #[must_use]
    pub fn new(stream: BoxedStream) -> Self {
        MessageStream(Arc::new(Mutex::new(stream)))
    }

    pub async fn next(&self) -> Option<Result<Message, StreamingError>> {
        let mut stream = self.0.lock().await;
        stream.next().await
    }
}
