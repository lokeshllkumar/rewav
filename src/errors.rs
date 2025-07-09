use std::io;
use thiserror::Error;

/// error type for the transcoder
#[derive(Error, Debug)]
pub enum TranscoderError {
    /// error: I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// error: input audio format not supported by the transcoder
    #[error("Input audio format not supported: {0}")]
    UnsupportedInputFormat(String),

    /// error: output audio format not supported by the transcoder
    #[error("Output audio format not supported: {0}")]
    UnsupportedOutputFormat(String),

    /// error: error from the `hound` crate for WAV
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),

    /// error: error from the `flac` crate for FLAC
    #[error("FLAC error: {0}")]
    Flac(String),

    /// error: error from the `rubato` crate for resampling
    #[error("Resampler error: {0}")]
    Resampler(String),

    /// error: error from the `ffmpeg-next` create for FFmpeg operations
    #[error("FFmpeg CLI error: {0}")]
    FfmpegCli(String),

    /// error: error with respect to file paths
    #[error("Path error: {0}")]
    Path(String),

    /// error: error during argument parsing or validation
    #[error("Argument error: {0}")]
    Argument(String),

    // catch all for other errors
    #[error("An unexpected error occurred: {0}")]
    Other(String),
}