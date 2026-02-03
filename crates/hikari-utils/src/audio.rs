use crate::audio::error::AudioError;
use hound;
use std::io::Cursor;

pub mod error;
pub fn pcm16_to_wave(pcm_audio: &[u8], channels: u16, sample_rate: u32) -> Result<Vec<u8>, AudioError> {
    let mut buffer = Cursor::new(Vec::new());

    let mut writer = hound::WavWriter::new(
        &mut buffer,
        hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;

    for sample in pcm_audio
        .chunks(2)
        .map(|chunk| i16::from_le_bytes([chunk.first().copied().unwrap_or(0), chunk.get(1).copied().unwrap_or(0)]))
    {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    Ok(buffer.into_inner())
}

pub fn wave_to_pcm16(wave_audio: &[u8]) -> Result<Vec<u8>, AudioError> {
    let cursor = Cursor::new(wave_audio);
    let mut reader = hound::WavReader::new(cursor)?;

    let spec = reader.spec();

    // Ensure WAV format is PCM16
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err(AudioError::NotPCM16);
    }

    let pcm_data: Vec<u8> = reader
        .samples::<i16>()
        .filter_map(Result::ok)
        .flat_map(i16::to_le_bytes)
        .collect();
    Ok(pcm_data)
}
