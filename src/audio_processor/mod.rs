pub mod resampler;

use log::debug;
use rayon::prelude::*;

/// converts a slice of i16 samples to f32 samples
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .par_iter()
        .map(|&s| s as f32 / i16::MAX as f32)
        .collect()
}

/// converts a slice of f32 samples to i16 samples
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .par_iter()
        .map(|&s| {
            (s * i16::MAX as f32)
                .round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16
        })
        .collect()
}

/// converts a slice of i32 samples to f32 samples (FLAC decoding)
pub fn i32_to_f32(samples: &[i32]) -> Vec<f32> {
    samples
        .par_iter()
        .map(|&s| s as f32 / i32::MAX as f32)
        .collect()
}

/// converts a slice of f32 samples to i32 samples (FLAC encoding)
pub fn f32_to_i32(samples: &[f32]) -> Vec<i32> {
    samples
        .par_iter()
        .map(|&s| {
            (s * i32::MAX as f32)
                .round()
                .clamp(i32::MIN as f32, i32::MAX as f32) as i32
        })
        .collect()
}

/// a highly simplified channel mixing logic
/// converts input audio sampls to the desired number of audio channels
/// if `target_channels` is 1, mixes down to mono
/// if `target_channels` is 2, mixes down to stereo
///     - if input is mono, duplicates the mono channel to stereo
///     - if input is stereo, keeps both channels as is and simply passes through
///     - in input is multi-channel, averages all channels to stereo
/// if `target_channels` is greater than 2
///     - if input is mono, duplicate the mono channel to all target channels
///     - if input is stereo, duplicate both channels to all target channels
///     - if input is multi-channel, attempts to map directly or averages if there is a mismatch between the input and output channels
pub fn mix_channels(
    input_samples: &[f32],
    input_channels: u8,
    target_channels: u8,
) -> Vec<f32> {
    if input_channels == target_channels {
        return input_samples.to_vec();
    }

    if input_samples.is_empty() || input_channels == 0 || target_channels == 0 {
        return Vec::new();
    }

    let input_frame_size = input_channels as usize;
    let output_frame_size = target_channels as usize;
    
    // parallelizing processing of individual frames
    input_samples.par_chunks_exact(input_frame_size)
        .flat_map(|input_frame| {
            let mut output_frame = vec![0.0; output_frame_size];

            match (input_channels, target_channels) {
                (1, 2) => {
                    // mono to stereo: duplicate the mono channel
                    output_frame[0] = input_frame[0];
                    output_frame[1] = input_frame[0];
                },
                (2, 1) => {
                    // stereo to mono: averaging the channel outputs
                    output_frame[0] = (input_frame[0] + input_frame[1]) / 2.0;
                },
                (n_in, n_out) if n_in < n_out => {
                    for c_out in 0..n_out {
                        output_frame[c_out as usize] = input_frame[(c_out as usize) % n_in as usize];
                    }
                },
                (n_in, n_out) if n_in > n_out => {
                    // averaging channels into groups if input has more channels than output
                    for c_out in 0..n_out {
                        let mut sum = 0.0;
                        let mut count = 0;
                        for c_in_idx in (c_out as usize)..n_in as usize{
                            sum += input_frame[c_in_idx];
                            count += 1;
                        }
                        if count > 0 {
                            output_frame[c_out as usize] = sum / count as f32;
                        }
                    }
                },
                _ => { // caught when input_channels == target_channels
                    debug!("Channel mix: Unhandled case, copying input frame directly to output frame");
                    output_frame = input_frame.to_vec();
                }
            }

            output_frame
        })
        .collect()
}