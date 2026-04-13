CREATE TABLE tts_cache
(
    message_hash TEXT PRIMARY KEY,
    audio_path   TEXT
);

CREATE INDEX tts_cache_message_hash ON tts_cache (message_hash);--
