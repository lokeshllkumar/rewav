use std::path::Path;
use log::{info, debug};
use crate::errors::TranscoderError;
use crate::transcoders::TranscodeOptions;
use crate::audio_processor::{self, resampler::AudioResampler};

/// transcodes a WAV file to another WAV file, applying specified options for sample rate, number of channels, and bit depth
/// lossless WAV ignore bitrate
pub fn transcode_wav_with_options(
    input_path: &Path,
    output_path: &Path,
    options: &TranscodeOptions,
) -> Result<(), TranscoderError> {
    info!("Native WAV transcoder: Reading from {:?}", input_path);

    let mut reader = hound::WavReader::open(input_path)?;
    let input_spec = reader.spec();

    info!("Input WAV specifications: {:?}", input_spec);

    // determining output specifications based on options or input
    let output_sample_rate = options.sample_rate.unwrap_or(input_spec.sample_rate);
    let output_channels = options.channels.unwrap_or(input_spec.channels as u8);

    let output_bits_per_sample = input_spec.bits_per_sample; // a change in bit depth is typically not observed via options in traditional transcoding

    let output_spec = hound::WavSpec {
        channels: output_channels as u16,
        sample_rate: output_sample_rate,
        bits_per_sample: output_bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };

    info!("Output WAV specifications: {:?}", output_spec);

    let mut writer = hound::WavWriter::create(output_path, output_spec)?;

    // initialzing resampler
    let mut audio_resampler:Option<AudioResampler> = None;
    if input_spec.sample_rate != output_sample_rate {
        audio_resampler = Some(AudioResampler::new(
            input_spec.sample_rate,
            output_sample_rate,
            input_spec.channels as u8,
            1024, // chunk size for resampling
        )?);
    }

    let mut buffer: Vec<f32> = Vec::new();
    let input_chunk_size = 1024 * input_spec.channels as usize;

    // reading samples, proessing, and writing to output
    let mut samples_iter = reader.samples::<i16>();
    loop {
        let mut chunk_i16: Vec<i16> = Vec::with_capacity(input_chunk_size);
        for _ in 0..input_chunk_size {
            if let Some(sample_result) = samples_iter.next() {
                chunk_i16.push(sample_result?);
            } else {
                break;
            }
        }

        if chunk_i16.is_empty() { // EOF
            break;
        }

        // converting i16 to f32
        let mut current_samples_f32 = audio_processor::i16_to_f32(&chunk_i16);

        if let Some(resampler) = &mut audio_resampler {
            current_samples_f32 = resampler.process_interleaved(&current_samples_f32)?;
        }

        // mixing channels
        if input_spec.channels as u8 != output_channels {
            current_samples_f32 = audio_processor::mix_channels(
                &current_samples_f32,
                input_spec.channels as u8,
                output_channels,
            );
        }

        // converting f32 to i16 (for WAV writer)
        let processed_samples_i16 = audio_processor::f32_to_i16(&current_samples_f32);

        // writing processed samples
        for &sample in &processed_samples_i16 {
            writer.write_sample(sample)?;
        }
    }

    // flushing resampler
    if let Some(resampler) = &mut audio_resampler {
        let flushed_samples_f32 = resampler.flush()?;
        if !flushed_samples_f32.is_empty() {
            let processed_samples_i16 = audio_processor::f32_to_i16(&flushed_samples_f32);
            for &sample in &processed_samples_i16 {
                writer.write_sample(sample)?;
            }
        }
    }

    writer.finalize()?;

    info!("Native WAV transcoder: Successfully wrote to {:?}", output_path);
    Ok(())
}