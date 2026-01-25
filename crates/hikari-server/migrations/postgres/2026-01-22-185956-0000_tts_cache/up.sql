CREATE TABLE tts_cache
(
    message_hash TEXT PRIMARY KEY,
    audio_path   TEXT
);

CREATE INDEX voice_cache_text_hash ON tts_cache (message_hash);--
