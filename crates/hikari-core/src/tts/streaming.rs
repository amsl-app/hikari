use crate::openai::streaming::MessageStream;
use crate::openai::{Content, Message};
use crate::tts::error::{CombinedError, TTSError};
use crate::tts::{demoji, send_to_stream};
use async_stream::stream;
use futures::{Stream, StreamExt};
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

pub type TextStream = Pin<Box<dyn Stream<Item = String> + Send>>;
pub type CombinedStream = Pin<Box<dyn Stream<Item = Result<CombinedStreamItem, CombinedError>> + Send>>;
pub type AudioOutputStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>, TTSError>> + Send>>;

#[derive(Debug, Clone)]
pub enum CombinedStreamItem {
    Message(Message),
    Audio(Vec<u8>),
}

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

pub(crate) fn attach_text_stream(mut message_stream: ArcMutexStream<MessageStream>) -> (MessageStream, TextStream) {
    tracing::debug!("Split message stream into message and text streams");
    let (text_tx, text_rc) = tokio::sync::mpsc::channel(10);
    let text_stream = ReceiverStream::new(text_rc);

    let local_message_stream: MessageStream = Box::pin(stream! {
        let mut buffer = String::new();
        while let Some(data) = message_stream.next().await {
            if let Ok(message) = data {
                match &message.content {
                    Content::Text(text) => {
                        let text = demoji(&text);
                        buffer.push_str(&text);
                        let add_white_space = buffer.ends_with(' ');
                        tracing::debug!(?buffer, "Received message in text stream");
                        let buffer_clone = buffer.clone();
                        let words = buffer_clone.split_whitespace().collect::<Vec<&str>>();
                        if words.len() > 1 {
                            buffer = (*words.last().unwrap_or(&"")).to_string();
                            if add_white_space {
                                buffer.push(' ');
                            }
                            for word in words.iter().take(words.len() - 1) {
                                let mut word = (*word).to_string();
                                word.push(' ');
                                tracing::debug!(sended = ?word,  "Send text to text stream");
                                send_to_stream(&text_tx, word.clone()).await;
                            }
                        }
                        yield Ok(message)
                    }
                    Content::Tool(_) => {
                        tracing::warn!("Received unexpected tool response");
                    }
                }

            } else {
                yield data
            }
        }
        if !buffer.is_empty() {
            if !buffer.ends_with(' ') {
                buffer.push(' '); // For elevenlabs to work properly
            }
            tracing::debug!(?buffer, "Send remaining text");
            send_to_stream(&text_tx, buffer.clone()).await;
        }
    });

    let text_stream = Box::pin(text_stream);

    (local_message_stream, text_stream)
}
