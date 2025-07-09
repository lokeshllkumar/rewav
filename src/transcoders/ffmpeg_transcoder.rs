use std::path::Path;
use std::process::Command;
use log::{info, debug, warn, error};
use crate::errors::TranscoderError;
use crate::transcoders::TranscodeOptions;

/// transcodes an audio file from any ffmpeg-supported audio format to any other ffmpeg-supported audio format using `ffmpeg-next` library
pub fn transcode_with_ffmpeg(
    input_path: &Path,
    output_path: &Path,
    options: &TranscodeOptions,
) -> Result<(), TranscoderError> {
    info!("FFmpeg transcoder: Converting {:?} to {:?} with options: {:?}", input_path, output_path, options);

    let mut command = Command::new("ffmpeg");

    command.arg("-i").arg(input_path);

    if let Some(codec) = &options.output_codec {
        command.arg("-c:a").arg(codec);
    }

    if let Some(bitrate_kbps) = options.bitrate_kbps {
        command.arg("-b:a").arg(format!("{}k", bitrate_kbps));
    }

    if let Some(sample_rate) = options.sample_rate {
        command.arg("-ar").arg(sample_rate.to_string());
    }

    if let Some(channels) = options.channels {
        command.arg("-ac").arg(channels.to_string());
    }

    if let Some(threads) = options.threads {
        command.arg("-threads").arg(threads.to_string());
    }

    if let Some(quality_preset) = &options.quality_preset {
        warn!("'quality-preset' is a highly codec-specific option and may not directly apply too all audio codecs via generic flags for the FFmpeg CLI");
        command.arg("-preset").arg(quality_preset);
    }

    command.arg("-y");

    command.arg(output_path);

    debug!("Executing FFmpeg: {:?}", command);

    let output = command.output().map_err(|e| {
        TranscoderError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to execute ffmpeg command. Please check if ffmpeg is installed and in your PATH. Error: {}", e),
        ))
    })?;

    if output.status.success() {
        info!("FFmpeg successfully transcoded {:?} to {:?}", input_path, output_path);
        debug!("FFmpeg stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    } else {
        error!("FFmpeg CLI failed to transcode {:?} to {:?}", input_path, output_path);
        error!("FFmpeg stderr:\n{}", String::from_utf8_lossy(&output.stderr));
        return Err(TranscoderError::FfmpegCli(format!(
            "FFmpeg exited with non-zero status: {:?}\nStderr:{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}
