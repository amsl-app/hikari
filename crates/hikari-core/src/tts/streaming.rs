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
    tracing::debug!("split message stream into message and text streams");
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
                    tracing::debug!(?buffer, "received message in text stream");
                    let buffer_clone = buffer.clone();
                    let mut words = buffer_clone.split_whitespace();
                    if let Some(word) = words.next_back() {
                        buffer.clear();
                        buffer.push_str(word);
                        if add_white_space {
                            buffer.push(' ');
                        }
                    }
                    for word in words {
                        let text = format!("{word} ");
                        tracing::debug!(?text,  "send text to text stream");
                        text_tx.send(text).await
                        .inspect_err(|error| {
                            tracing::error!(error = error as &dyn std::error::Error, "failed sending text to text stream")
                        })?;
                    }
                    yield message
                }
                Content::Tool(_) => {
                    tracing::warn!("received unexpected tool response");
                }
            }
        }
        if !buffer.is_empty() {
            if !buffer.ends_with(' ') {
                buffer.push(' '); // For elevenlabs to work properly
            }
            tracing::debug!(?buffer, "send remaining text");
            text_tx.send(buffer.clone())
            .await
            .inspect_err(|error| {
                tracing::error!(error = error as &dyn std::error::Error, "failed sending remaining text to text stream")
            })?;
        }
    });

    let text_stream = Box::pin(text_stream);

    (local_message_stream, text_stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_stream::stream;

    #[tokio::test]
    async fn test_attach_text_stream() {
        let stream = stream! {
            yield Ok(Message::new(Content::Text("A B C".to_string()), None));
            yield Ok(Message::new(Content::Text("1 2 3".to_string()), None));
            yield Ok(Message::new(Content::Text("XYZ".to_string()), None));
        };
        let stream = Box::pin(stream);
        let message_stream = MessageStream::new(stream);
        let (mut message_stream, mut word_stream) = attach_text_stream(message_stream);
        let mut buffer = String::new();
        while let Some(message) = message_stream.next().await {
            let message = match message.unwrap().content {
                Content::Text(message) => message,
                _ => panic!("Expected text, got something else"),
            };

            buffer.push_str(&message);
            buffer.push_str(";")
        }
        assert_eq!(buffer, "A B C;1 2 3;XYZ;");
        buffer.clear();
        while let Some(word) = word_stream.next().await {
            let word = word;
            buffer.push_str(&word);
            buffer.push_str(";")
        }
        assert_eq!(buffer, "A ;B ;C1 ;2 ;3XYZ ;");
    }
}
