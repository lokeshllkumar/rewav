use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use log::{info, debug};
use claxon::FlacReader;
use crate::errors::TranscoderError;
use crate::transcoders::TranscodeOptions;
use crate::audio_processor::{self, resampler::AudioResampler};
use hound;

/// transcoding a FLAC file to a WAV file, applying teh sepcified options for sample rate and number of channels using native Rust processing
/// the bitrate option is ignored for lossless FLAC and WAV
pub fn transcode_flac_to_wav_with_options(
    input_path: &Path,
    output_path: &Path,
    options: &TranscodeOptions,
) -> Result<(), TranscoderError> {
    info!("Native FLAC to WAV transcoder: Reading from {:?}", input_path);

    let file = File::open(input_path)?;
    let mut reader = FlacReader::new (file)
        .map_err(|e| TranscoderError::Flac(format!("Failed to create FLAC decoder: {:?}", e)))?;

    let stream_info = reader.streaminfo();
    info!("Input FLAC stream info: {:?}", stream_info);

    // converting from f32 to i32 for processing
    let input_sample_rate = stream_info.sample_rate;
    let input_channels = stream_info.channels as u8;
    let input_bits_per_sample = stream_info.bits_per_sample;

    let output_sample_rate = options.sample_rate.unwrap_or(input_sample_rate);
    let output_channels = options.channels.unwrap_or(input_channels);
    let output_bits_per_sample = 16; // for WAV output

    let wav_spec = hound::WavSpec {
        channels: output_channels as u16,
        sample_rate: output_sample_rate,
        bits_per_sample: output_bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };

    info!("Output WAV specifications: {:?}", wav_spec);

    let mut writer = hound::WavWriter::create(output_path, wav_spec)?;

    // initializing resampler
    let mut audio_resampler: Option<AudioResampler> = None;
    if input_sample_rate != output_sample_rate {
        audio_resampler = Some(AudioResampler::new(
            input_sample_rate,
            output_sample_rate,
            input_channels,
            1024,
        )?);
    }

    // decoding FLAC frames, process, and write WAV samples
    let mut samples = reader.samples();
    let mut buffer: Vec<i32> = Vec::new();
    
    while let Some(sample_result) = samples.next() {
        let sample = sample_result
            .map_err(|e| TranscoderError::Flac(format!("Error decoding FLAC sample: {:?}", e)))?;
        buffer.push(sample);

        // processing in chunks
        if buffer.len() >= 1024 * input_channels as usize {
            let mut current_samples_f32 = audio_processor::i32_to_f32(&buffer);

            // resampling
            if let Some(resampler) = &mut audio_resampler {
                current_samples_f32 = resampler.process_interleaved(&current_samples_f32)?;
            }

            // mixing channels
            if input_channels != output_channels {
                current_samples_f32 = audio_processor::mix_channels(
                    &current_samples_f32,
                    input_channels,
                    output_channels,
                );
            }

            let processed_samples_i16 = audio_processor::f32_to_i16(&current_samples_f32);

            for &sample in &processed_samples_i16 {
                writer.write_sample(sample)?;
            }
            buffer.clear();
        }
    }

    if !buffer.is_empty() {
        let mut current_samples_f32 = audio_processor::i32_to_f32(&buffer);

        if let Some(resampler) = &mut audio_resampler {
            current_samples_f32 = resampler.process_interleaved(&current_samples_f32)?;
        }

        if input_channels != output_channels {
            current_samples_f32 = audio_processor::mix_channels(
                &current_samples_f32,
                input_channels,
                output_channels,
            );
        }

        let processed_samples_i16 = audio_processor::f32_to_i16(&current_samples_f32);

        for &sample in &processed_samples_i16 {
            writer.write_sample(sample)?;
        }
    }

    if let Some(resampler) = &mut audio_resampler {
        let flushed_samples_f32 = resampler.flush()?;
        if !flushed_samples_f32.is_empty() {
            let processed_samples_i16 = audio_processor::f32_to_i16(&flushed_samples_f32);
            for &sample in &processed_samples_i16 {
                writer.write_sample(sample)?;
            }
        }
    }

    // finalizing writer
    info!("Native FLAC to WAV transcoder; successfully wrote to {:?}", output_path);
    Ok(())
}