use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};
use log::debug;
use crate::errors::TranscoderError;

pub struct AudioResampler {
    resampler: SincFixedIn<f32>,
    input_buffer: Vec<Vec<f32>>,
    output_buffer: Vec<Vec<f32>>,
    input_frame_size: usize,
}

impl AudioResampler {
    /// `input_rate` and `output_rate` are in Hz
    /// `channels` is the number of audio channels
    /// `chunk_size` is the number of samples per channel to process at a time
    pub fn new(
        input_rate: u32,
        output_rate: u32,
        channels: u8,
        chunk_size: usize,
    ) -> Result<Self, TranscoderError> {
        debug!("Initialzing audio resampler: input rate = {} Hz, output rate = {} Hz, channels = {}, chunk size = {}",
            input_rate, output_rate, channels, chunk_size);

        let parameters = SincInterpolationParameters {
            sinc_len: 256, // length of the sinc filter, higher implies a better quality
            f_cutoff: 0.95, // cutoff frequency
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2, // window function for the filter
        };

        let resampler = SincFixedIn::<f32>::new(
            output_rate as f64 / input_rate as f64,
            chunk_size as f64,
            parameters,
            channels as usize,
            0, // resample delay in frames
        ).map_err(
            |e| TranscoderError::Resampler(format!("Failed to intialize Rubato sampler: {:?}", e))
        )?;

        // initializing channel interleaved buffers for `rubato`
        let input_buffer = vec![vec![0.0f32; chunk_size]; channels as usize];
        let output_buffer = vec![vec![0.0f32; resampler.output_frames_next()]; channels as usize];

        Ok(Self {
            resampler,
            input_buffer,
            output_buffer,
            input_frame_size: channels as usize,
        })
    }

    /// resamples a chunk of interleaved audio samples
    pub fn process_interleaved(&mut self, input_interleaved: &[f32]) -> Result<Vec<f32>, TranscoderError> {
        if input_interleaved.is_empty() {
            return Ok(Vec::new());
        }

        // de-interleaving input samples into the channel-separated format offered by `rubato`
        let num_input_frames = input_interleaved.len() / self.input_frame_size;
        for c in 0..self.input_buffer.len() {
            self.input_buffer[c].resize(num_input_frames, 0.0);
            for i in 0..num_input_frames {
                self.input_buffer[c][i] = input_interleaved[i * self.input_frame_size + c];
            }
        }

        // processing block
        let (resampled_frames_per_channel, _channels) = self.resampler.process_into_buffer(
            &self.input_buffer,
            &mut self.output_buffer,
            None,
        ).map_err(
            |e| TranscoderError::Resampler(format!("Failed to process samples with rubato {:?}", e))
        )?;

        // re-interleaving output sampled from the special format in `rubato`
        let mut output_interleaved = Vec::with_capacity(resampled_frames_per_channel * self.input_buffer.len());
        for i in 0..resampled_frames_per_channel {
            for c in 0..self.input_buffer.len() {
                output_interleaved.push(self.output_buffer[c][i]);
            }
        }
        Ok(output_interleaved)
    }

    /// flushes any remaining buffered samples from the resampler
    pub fn flush(&mut self) -> Result<Vec<f32>, TranscoderError> {
        debug!("Flushing resampler");
        let empty_input: Vec<Vec<f32>> = vec![Vec::new(); self.input_buffer.len()];
        let (resampled_frames_per_channel, _channels) = self.resampler.process_into_buffer(
            &empty_input,
            &mut self.output_buffer,
            None,
        ).map_err(
            |e| TranscoderError::Resampler(format!("Failed to flush rubato resampler: {:?}", e))
        )?;

        let mut output_interleaved = Vec::with_capacity(resampled_frames_per_channel * self.input_buffer.len());
        for i in 0..resampled_frames_per_channel {
            for c in 0..self.input_buffer.len() {
                output_interleaved.push(self.output_buffer[c][i]);
            }
        }
        Ok(output_interleaved)
    }
}