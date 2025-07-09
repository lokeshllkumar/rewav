pub mod native_wav;
pub mod native_flac_to_wav;
pub mod ffmpeg_transcoder;

use std::path::Path;
use log::{info, error, debug};
use crate::errors::TranscoderError;
use crate::utils::{infer_file_type, get_file_extension};

/// options for audio transcoding
#[derive(Debug, Default, Clone)]
pub struct TranscodeOptions {
    /// output file format extension
    pub output_format_extension: String,
    /// output audio codec; if None, ffmpeg behaves a fallback and chooses a default for the output format
    pub output_codec: Option<String>,
    /// desired output bitrate in kbps; if None, ffmpeg chooses a default
    pub bitrate_kbps: Option<u32>,
    /// desired output sample rate in Hz; if None, ffmpeg will choose the input audio's sample rate or a codec default
    pub sample_rate: Option<u32>,
    /// desired number of output audio channels; if None, ffmpeg will use the input audio's channel count or a codec default
    pub channels: Option<u8>, 
    /// defienes the quality preset for ffmpeg; codec-specific and may influence encoding speed vs compression efficiency
    pub quality_preset: Option<String>,
    /// number fo threads to used for encoding; if None, ffmpeg will default to all available cores
    pub threads: Option<usize>,
}

/// selects between the native Rust implementations and ffmpeg as the fallback based on the detected file type
pub fn transcode_audio(
    input_path: &Path,
    output_path: &Path,
    options: &TranscodeOptions,
) -> Result<(), TranscoderError> {
    info!("Attempting to transcode audio from {:?} to {:?} with options: {:?}", input_path, output_path, options);

    // inferring input file type
    let input_file_type = infer_file_type(input_path)?;
    let input_extension = get_file_extension(input_path)?;

    info!("Detected input file type {:?} extension: '{}'", input_file_type, input_extension);
    info!("Requested output format extension: '{}'", options.output_format_extension);

    // dispatching processing to the appropriate transcoder
    // prioritizing native transcoding and relying on ffmpeg if either of the input or output extensions are not supported
    let use_native_wav = input_file_type.as_ref().map_or(false, |t| t.extension() == "wav") 
        && options.output_format_extension == "wav"
        && options.output_codec.is_none();

    let use_native_flac_to_wav = input_file_type.as_ref().map_or(false, |t| t.extension() == "flac") 
        && options.output_format_extension == "wav"
        && options.output_codec.is_none();

    if use_native_wav {
        info!("Dispatching to native WAV transcoder...");
        native_wav::transcode_wav_with_options(input_path, output_path, options)
    }
    else if use_native_flac_to_wav {
        info!("Dispatching to native FLAC to WAV transcoder...");
        native_flac_to_wav::transcode_flac_to_wav_with_options(input_path, output_path, options)
    }
    else {
        info!("Dispatching to FFmpeg's transcoder (fallback)...");
        ffmpeg_transcoder::transcode_with_ffmpeg(input_path, output_path, options)
    }
}