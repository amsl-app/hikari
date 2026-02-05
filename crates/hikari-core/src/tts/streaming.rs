use crate::openai::streaming::{BoxedStream, MessageStream};
use crate::openai::{Content, Message};
use crate::tts::error::{CombinedError, TTSError};
use crate::tts::prepare_text_for_voice;
use async_stream::try_stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio_stream::wrappers::ReceiverStream;

pub type TextStream = Pin<Box<dyn Stream<Item = String> + Send>>;
pub type CombinedStream = Pin<Box<dyn Stream<Item = Result<CombinedStreamItem, CombinedError>> + Send>>;
pub type AudioOutputStream = Pin<Box<dyn Stream<Item = Result<Vec<u8>, TTSError>> + Send>>;

#[derive(Debug, Clone)]
pub enum CombinedStreamItem {
    Message(Message),
    Audio(Vec<u8>),
}

pub(crate) fn attach_text_stream(mut message_stream: MessageStream) -> (BoxedStream, TextStream) {
    tracing::debug!("Split message stream into message and text streams");
    let (text_tx, text_rc) = tokio::sync::mpsc::channel(10);
    let text_stream = ReceiverStream::new(text_rc);

    let local_message_stream: BoxedStream = Box::pin(try_stream! {
        let mut buffer = String::new();
        while let Some(data) = message_stream.0.next().await {
            let message = data?;
            match &message.content {
                Content::Text(text) => {
                    let text = prepare_text_for_voice(text);
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
                            tracing::debug!(sent = ?word,  "Send text to text stream");
                            text_tx.send(word.clone())
                            .await
                            .inspect_err(|error| {
                                tracing::error!(error = error as &dyn std::error::Error, "Faild sending text to text stream")
                            })?;
                        }
                    }
                    yield message
                }
                Content::Tool(_) => {
                    tracing::warn!("Received unexpected tool response");
                }
            }
        }
        if !buffer.is_empty() {
            if !buffer.ends_with(' ') {
                buffer.push(' '); // For elevenlabs to work properly
            }
            tracing::debug!(?buffer, "Send remaining text");
            text_tx.send(buffer.clone())
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn std::error::Error, "Failed sending remaining text to text stream")
            })?;
        }
    });

    let text_stream = Box::pin(text_stream);

    (local_message_stream, text_stream)
}
