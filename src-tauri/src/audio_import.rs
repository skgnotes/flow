use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::audio_recorder::resample;

/// Convert any supported audio file to 16kHz mono f32 samples for Whisper
pub fn convert_to_whisper_format(path: &Path) -> Result<Vec<f32>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    // Add extension hint if available
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();
    let decoder_opts = DecoderOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| format!("Unsupported audio format: {}", e))?;

    let mut format = probed.format;

    let track = format
        .default_track()
        .ok_or("No audio track found in file")?;

    let track_id = track.id;
    let source_sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let source_channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let mut all_samples: Vec<f32> = Vec::new();

    // Decode all packets
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let spec = *decoded.spec();
        let num_channels = spec.channels.count();

        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let samples = sample_buf.samples();

        // Convert to mono if needed
        if num_channels > 1 {
            for chunk in samples.chunks(num_channels) {
                let sum: f32 = chunk.iter().sum();
                all_samples.push(sum / num_channels as f32);
            }
        } else {
            all_samples.extend_from_slice(samples);
        }
    }

    if all_samples.is_empty() {
        return Err("No audio data found in file".to_string());
    }

    // Resample to 16kHz if needed
    let resampled = if source_sample_rate != 16000 {
        resample(&all_samples, source_sample_rate, 16000)
    } else {
        all_samples
    };

    Ok(resampled)
}

/// Get duration of audio in seconds
pub fn get_audio_duration(samples: &[f32], sample_rate: u32) -> f32 {
    samples.len() as f32 / sample_rate as f32
}
