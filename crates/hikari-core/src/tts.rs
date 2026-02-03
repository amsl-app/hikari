use async_stream::{stream, try_stream};
use elevenlabs_rs::endpoints::genai::tts::ws::{BOSMessage, TTSWebSocketQuery, WebSocketTTS, WebSocketTTSBody};
use elevenlabs_rs::endpoints::genai::tts::{TextToSpeechBody, TextToSpeechQuery, TextToSpeechStream};
use elevenlabs_rs::{ElevenLabsClient, OutputFormat};
use futures::{Stream, StreamExt};
use regex::Regex;
use sea_orm::DatabaseConnection;
use std::convert::Into;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use crate::openai::Content;
use crate::openai::streaming::MessageStream;
use crate::tts::cache::{cache_speech, get_speech};
use crate::tts::config::TTSConfig;
use crate::tts::error::{CombinedError, TTSError};
use crate::tts::streaming::{CombinedStream, CombinedStreamItem, attach_text_stream};

pub mod cache;
pub mod config;
pub mod error;
pub mod streaming;

#[allow(clippy::result_large_err)]
fn build_client(config: &TTSConfig) -> (ElevenLabsClient, String, String) {
    let model_id = &config.model;
    let voice_id = &config.voice;
    let client = ElevenLabsClient::new(&config.api_key);

    (client, model_id.clone(), voice_id.clone())
}

pub fn text_to_speech_stream(
    text: &str,
    db: Arc<DatabaseConnection>,
    config: Arc<TTSConfig>,
) -> Pin<Box<dyn Stream<Item = Result<Vec<u8>, TTSError>> + Send>> {
    tracing::debug!("Converting text to speech");
    let text = demoji(text);
    Box::pin(try_stream! {
        let cached = get_speech(db.as_ref(), config.as_ref(), &text).await?;

        if let Some(audio) = cached {
            yield audio;
        } else {
            tracing::debug!("No cached audio found, generating new audio");

            let (client, model_id, voice_id) = build_client(config.as_ref());
            let body = TextToSpeechBody::new(&text).with_model_id(model_id);
            let request = TextToSpeechStream::new(voice_id, body).with_query(TextToSpeechQuery::default().with_output_format(OutputFormat::MuLaw8000Hz));
            tracing::debug!("Sending request to elevenlabs");
            let mut audio = client.hit(request).await?;
            let mut complete_audio: Vec<u8> = vec![];
            while let Some(data) = audio.next().await {
                let data = data?.to_vec();
                complete_audio.extend(&data);
                yield data;
            }

            cache_speech(db.as_ref(), config.as_ref(), &complete_audio, &text).await?;
        }
    })
}

fn text_stream_to_speech_stream(
    text_stream: impl Stream<Item = String> + Send + 'static,
    config: Arc<TTSConfig>,
) -> Pin<Box<dyn Stream<Item = Result<Vec<u8>, TTSError>> + Send>> {
    Box::pin(try_stream! {
        let (client, model_id, voice_id) = build_client(config.as_ref());
        let current_time = std::time::SystemTime::now();
        tracing::debug!("Send to elevenlabs: {:?}", current_time.duration_since(std::time::UNIX_EPOCH));
        let body = WebSocketTTSBody::new(BOSMessage::default().with_generation_config([50, 70, 120, 160]), text_stream);
        let request = WebSocketTTS::new(voice_id, body).with_query(TTSWebSocketQuery::default().with_output_format(OutputFormat::MuLaw8000Hz).with_model_id(model_id));
        let mut stream = client.hit_ws(request).await?;

        while let Some(data) = stream.next().await {
            let bytes = data?.audio_as_bytes()?.to_vec();
            tracing::debug!(bytes_len = ?bytes.len(), "Receive audio bytes");
            yield bytes;
        }
        tracing::debug!("Elevenlabs WS closed");
    })
}

fn message_stream_to_combined_stream(message_stream: MessageStream, config: Arc<TTSConfig>) -> CombinedStream {
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<CombinedStreamItem, CombinedError>>(10);
    let (mut message_stream, text_stream) = attach_text_stream(message_stream);
    let text_sender = sender.clone();
    tokio::spawn(async move {
        while let Some(message) = message_stream.next().await {
            match message {
                Ok(message) => {
                    tracing::debug!("Send message to combined stream");
                    send_to_stream(&text_sender, Ok(CombinedStreamItem::Message(message))).await;
                }
                Err(err) => {
                    tracing::error!(?err, "Failed to get message from message stream");
                    send_to_stream(&text_sender, Err(err.into())).await;
                }
            }
        }
        tracing::debug!("Text stream finished");
    });

    tokio::spawn(async move {
        let mut audio_stream = text_stream_to_speech_stream(text_stream, config);
        while let Some(audio) = audio_stream.next().await {
            match audio {
                Ok(audio) => {
                    tracing::debug!("Send audio to combined stream");
                    send_to_stream(&sender, Ok(CombinedStreamItem::Audio(audio))).await;
                }
                Err(err) => {
                    tracing::error!(?err, "Failed to get audio from audio stream");
                    send_to_stream(&sender, Err(err.into())).await;
                }
            }
        }
        tracing::debug!("Audio stream finished");
    });

    Box::pin(ReceiverStream::new(receiver))
}

async fn send_to_stream<T>(stream: &Sender<T>, item: T)
where
    T: std::fmt::Debug,
{
    match stream.send(item).await {
        Ok(()) => {}
        Err(err) => {
            tracing::error!(?err, "Failed to send item to stream");
        }
    }
}

#[must_use]
pub fn message_stream_to_combined_stream_cached(
    message_stream: MessageStream,
    db: Arc<DatabaseConnection>,
    config: Arc<TTSConfig>,
) -> CombinedStream {
    let mut complete_audio: Vec<u8> = vec![];
    let mut complete_text = String::new();
    let mut combined = message_stream_to_combined_stream(message_stream, Arc::clone(&config));
    Box::pin(stream! {
         while let Some(item) = combined.next().await {
             match item.as_ref() {
                 Ok(CombinedStreamItem::Message(message)) => {
                     match &message.content {
                         Content::Text(text) => {
                             complete_text.push_str(text);
                         }
                         Content::Tool(_) => {
                             tracing::warn!("Received unexpected tool response");
                         }
                     }

                 }
                 Ok(CombinedStreamItem::Audio(audio)) => {
                     complete_audio.extend(audio);
                 }
                _ => {}
            }
            yield item;
        }
        cache_speech(db.as_ref(), config.as_ref(), &complete_audio, &complete_text).await?;
    })
}

fn demoji(string: &str) -> String {
    let regex = Regex::new(concat!(
        "[",
        "\u{01F600}-\u{01F64F}",
        "\u{01F300}-\u{01F5FF}",
        "\u{01F680}-\u{01F6FF}",
        "\u{01F1E0}-\u{01F1FF}",
        "\u{002702}-\u{0027B0}",
        "\u{0024C2}-\u{01F251}",
        "]+",
    ))
    .unwrap();

    let string = regex.replace_all(string, "").to_string();

    html_escape::decode_html_entities(&string).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::{Message, streaming::BoxedStream};
    use futures::lock::Mutex;
    use tokio::time::{Duration, sleep};
    use tokio_stream::StreamExt;

    fn simulate_websocket_stream() -> BoxedStream {
        let stream = stream! {
            yield Ok(Message::new(Content::Text("Hellö, world! ".to_string()), None));
            sleep(Duration::from_secs(2)).await;
            yield Ok(Message::new(Content::Text("Goodbye, world! ".to_string()), None));
            sleep(Duration::from_secs(2)).await;
            yield Ok(Message::new(Content::Text("How are you?".to_string()), None));
        };
        Box::pin(stream)
    }

    #[tokio::test]
    async fn test_attach_text_stream() {
        let (mut message_stream, mut text_stream) = attach_text_stream(MessageStream::new(simulate_websocket_stream()));

        let combined_response = Arc::new(Mutex::new(String::new()));

        let message_writer = Arc::clone(&combined_response);
        let message_task = tokio::spawn(async move {
            while let Some(message) = message_stream.next().await {
                let message = message.unwrap();
                let text = match message.content {
                    Content::Text(text) => text,
                    _ => "".to_string(),
                };
                message_writer
                    .lock()
                    .await
                    .push_str(format!("Message: {}", text).as_str());
            }
        });

        let text_writer = Arc::clone(&combined_response);
        let text_task = tokio::spawn(async move {
            while let Some(text) = text_stream.next().await {
                text_writer.lock().await.push_str(format!("Text: {text}").as_str());
            }
        });

        let (res_1, res_2) = tokio::join!(message_task, text_task);
        res_1.unwrap();
        res_2.unwrap();
        let response = combined_response.lock().await.to_owned();

        assert_eq!(
            response,
            "Message: Hellö, world! Text: Hellö, Message: Goodbye, world! Text: world! Text: Goodbye, Message: How are you?Text: world! Text: How Text: are Text: you? "
        );
    }
}
