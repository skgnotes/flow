use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

/// Thread-safe audio samples storage
pub struct SharedSamples {
    samples: Mutex<Vec<f32>>,
    is_recording: AtomicBool,
}

impl SharedSamples {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            samples: Mutex::new(Vec::new()),
            is_recording: AtomicBool::new(false),
        })
    }

    pub fn start_recording(&self) {
        self.samples.lock().unwrap().clear();
        self.is_recording.store(true, Ordering::SeqCst);
    }

    pub fn stop_recording(&self) {
        self.is_recording.store(false, Ordering::SeqCst);
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn add_samples(&self, new_samples: &[f32]) {
        if self.is_recording() {
            self.samples.lock().unwrap().extend_from_slice(new_samples);
        }
    }

    pub fn get_samples(&self) -> Vec<f32> {
        self.samples.lock().unwrap().clone()
    }
}

/// Start recording audio in a background thread
/// Returns a handle that stops recording when dropped
pub fn start_recording_thread(shared: Arc<SharedSamples>) -> Result<thread::JoinHandle<()>, String> {
    shared.start_recording();

    let handle = thread::spawn(move || {
        let host = cpal::default_host();

        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                eprintln!("No input device available");
                return;
            }
        };

        let supported_configs = match device.supported_input_configs() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get supported configs: {}", e);
                return;
            }
        };

        // Find a suitable config
        let config = match supported_configs
            .filter(|c| c.channels() == 1 || c.channels() == 2)
            .min_by_key(|c| {
                let min = c.min_sample_rate().0;
                let max = c.max_sample_rate().0;
                if 16000 >= min && 16000 <= max {
                    0
                } else if 16000 < min {
                    min - 16000
                } else {
                    16000 - max
                }
            }) {
            Some(c) => c,
            None => {
                eprintln!("No suitable audio config found");
                return;
            }
        };

        let sample_rate = if config.min_sample_rate().0 <= 16000 && config.max_sample_rate().0 >= 16000 {
            cpal::SampleRate(16000)
        } else {
            config.min_sample_rate()
        };

        let config = config.with_sample_rate(sample_rate);
        let channels = config.channels() as usize;
        let source_sample_rate = config.sample_rate().0;

        let shared_clone = shared.clone();

        let stream = match device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !shared_clone.is_recording() {
                    return;
                }

                // Convert to mono if stereo
                let mono_samples: Vec<f32> = if channels == 2 {
                    data.chunks(2)
                        .filter_map(|chunk| {
                            if chunk.len() == 2 {
                                Some((chunk[0] + chunk[1]) / 2.0)
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    data.to_vec()
                };

                shared_clone.add_samples(&mono_samples);
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to build input stream: {}", e);
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("Failed to start stream: {}", e);
            return;
        }

        // Keep the stream alive while recording
        while shared.is_recording() {
            thread::sleep(std::time::Duration::from_millis(50));
        }

        // Stream is dropped here, stopping the recording

        // Resample if needed
        if source_sample_rate != 16000 {
            let samples = shared.get_samples();
            let resampled = resample(&samples, source_sample_rate, 16000);
            *shared.samples.lock().unwrap() = resampled;
        }
    });

    Ok(handle)
}

/// Resample audio from one sample rate to another using linear interpolation
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;

    (0..new_len)
        .map(|i| {
            let src_idx = i as f64 * ratio;
            let idx = src_idx as usize;
            let frac = (src_idx - idx as f64) as f32;

            if idx + 1 < samples.len() {
                samples[idx] * (1.0 - frac) + samples[idx + 1] * frac
            } else if idx < samples.len() {
                samples[idx]
            } else {
                0.0
            }
        })
        .collect()
}
