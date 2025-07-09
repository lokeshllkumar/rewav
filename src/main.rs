mod errors;
mod utils;
mod transcoders;
mod audio_processor;

use clap::{Parser, Subcommand};
use core::num;
use std::{path::{Path, PathBuf}, thread::Thread};
use log::{info, error, warn, LevelFilter};
use env_logger::{Builder, Target};
use rayon::ThreadPoolBuilder;
use num_cpus;

#[derive(Parser, Debug)]
#[clap(author, version, about = "An audio transcoder written in Rust", long_about = None)]
struct CliArgs {
    /// input audio file path
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// output audio file path determined by the output file extension
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,

    /// desired output audio codec
    /// if not specified, ffmpeg will choose a default for the format
    /// native transcoders will ignore this option
    #[arg(long)]
    codec: Option<String>,

    /// desired output bitrate in kbps
    /// primarily for lossy codecs; if not specified, ffmpeg will choose a default
    /// lossless codecs will ignore this option
    #[arg(long, value_name = "KBPS")]
    bitrate: Option<u32>,

    /// desired output sample rate in Hz
    #[arg(long, value_name = "HZ")]
    sample_rate: Option<u32>,

    /// desired number of output audio channels
    #[arg(long, value_name = "NUM")]
    channels: Option<u8>,

    /// quality preset for ffmpeg encoders
    /// this is codec-specific and influences the encoding speed vs compression efficiency
    /// this option only applies to the ffmpeg transcoder
    #[arg(long)]
    quality_preset: Option<String>,

    /// number of threads ffmpeg should use for encoding
    /// defaults to the number of logical CPU cores
    /// applicable only to the fallback ffmpeg transcoder
    #[arg(long, value_name = "NUM")]
    threads: Option<usize>,

    /// increasing verbosity of logging
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

fn main() -> Result<(), errors::TranscoderError> {
    // configuring logging based on level of verbosity
    let log_level = match CliArgs::parse().verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    Builder::new()
        .filter_level(log_level)
        .target(Target::Stdout)
        .init();

    info!("Audio transcoder application started");

    // parsing command line arguments
    let cli = CliArgs::parse();

    let num_threads = cli.threads.unwrap_or_else(num_cpus::get);
    if num_threads > 0 {
        match ThreadPoolBuilder::new().num_threads(num_threads).build_global() {
            Ok(_) => info!("Rayon thread pool configured with {} threads", num_threads),
            Err(e) => warn!("Failed to configure Rayon thread pool: {}. Rayan will use default threading", e),
        }
    }
    else {
        warn!("Invalid number of threads specified ({}). Rayon will use default threading", num_threads)
    }

    // validating input and output paths
    if !cli.input.exists() {
        return Err(errors::TranscoderError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file does not exist: {:?}", cli.input.display()),
        )));
    }
    if !cli.input.is_file() {
        return Err(errors::TranscoderError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Input path is not a file: {:?}", cli.input.display()),
        )));
    }

    let output_extension = utils::get_file_extension(&cli.output)?;
    if output_extension.is_empty() {
        return Err(errors::TranscoderError::Path(format!("Output file path must have an extension: {}", cli.output.display())));
    }

    let options = transcoders::TranscodeOptions {
        output_format_extension: output_extension,
        output_codec: cli.codec,
        bitrate_kbps: cli.bitrate,
        sample_rate: cli.sample_rate,
        channels: cli.channels,
        quality_preset: cli.quality_preset,
        threads: cli.threads,
    };

    match transcoders::transcode_audio(&cli.input, &cli.output, &options) {
        Ok(_) => info!("Audio transcoding completed successfully!"),
        Err(e) => error!("Error during transcoding: {}", e),
    }

    info!("Audio transcoder application finished");
    Ok(())
}