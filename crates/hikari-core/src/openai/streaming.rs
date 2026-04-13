use crate::openai::Message;
use crate::openai::error::StreamingError;
use futures::Stream;
use std::fmt;
use std::pin::Pin;

pub type BoxedStream = Pin<Box<dyn Stream<Item = Result<Message, StreamingError>> + Send>>;

pub struct MessageStream(pub BoxedStream);

impl MessageStream {
    pub fn new(stream: BoxedStream) -> Self {
        MessageStream(stream)
    }
}

impl fmt::Debug for MessageStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageStream(...)")
    }
}
