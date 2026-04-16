use axum::extract::ws::{Message, Utf8Bytes};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("unexpected binary message")]
    UnexpectedBinaryMessage,
}

pub(crate) enum IncomingMessage {
    Text(Utf8Bytes),
    Control,
}

pub(crate) fn decode_message(message: Message) -> Result<IncomingMessage, Error> {
    match message {
        Message::Text(message) => Ok(IncomingMessage::Text(message)),
        // We just close the connection if we receive a binary message because we don't
        // expect any binary messages
        Message::Binary(_) => {
            tracing::error!("received unexpected binary message");
            Err(Error::UnexpectedBinaryMessage)
        }
        // The library handles control messages
        Message::Close(_) | Message::Ping(_) | Message::Pong(_) => {
            tracing::debug!("received control message");
            Ok(IncomingMessage::Control)
        }
    }
}
