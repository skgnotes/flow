use once_cell::sync::Lazy;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::whisper_model::{get_model_path, is_model_downloaded};

// Global Whisper context - expensive to create, so we reuse it
static WHISPER_CTX: Lazy<Mutex<Option<WhisperContext>>> = Lazy::new(|| Mutex::new(None));

/// Initialize or get the Whisper context
fn ensure_context_initialized() -> Result<(), String> {
    let mut ctx_guard = WHISPER_CTX.lock().map_err(|e| format!("Lock error: {}", e))?;

    if ctx_guard.is_none() {
        if !is_model_downloaded() {
            return Err("Whisper model not downloaded. Please download it first.".to_string());
        }

        let model_path = get_model_path();
        let model_path_str = model_path
            .to_str()
            .ok_or("Invalid model path encoding")?;

        let ctx = WhisperContext::new_with_params(model_path_str, WhisperContextParameters::default())
            .map_err(|e| format!("Failed to load Whisper model: {}", e))?;

        *ctx_guard = Some(ctx);
    }

    Ok(())
}

/// Transcribe audio samples (must be 16kHz mono f32)
pub fn transcribe_audio(samples: &[f32]) -> Result<String, String> {
    if samples.is_empty() {
        return Err("No audio samples provided".to_string());
    }

    // Ensure context is initialized
    ensure_context_initialized()?;

    let ctx_guard = WHISPER_CTX.lock().map_err(|e| format!("Lock error: {}", e))?;
    let ctx = ctx_guard
        .as_ref()
        .ok_or("Whisper context not initialized")?;

    // Create state for this transcription
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("Failed to create Whisper state: {}", e))?;

    // Configure transcription parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // Optimize for speed and English
    params.set_n_threads(4);
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_no_context(true);
    params.set_single_segment(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Run transcription
    state
        .full(params, samples)
        .map_err(|e| format!("Transcription failed: {}", e))?;

    // Collect all segments
    let num_segments = state
        .full_n_segments()
        .map_err(|e| format!("Failed to get segment count: {}", e))?;

    let mut transcript = String::new();

    for i in 0..num_segments {
        if let Ok(segment_text) = state.full_get_segment_text(i) {
            if !transcript.is_empty() && !segment_text.starts_with(' ') {
                transcript.push(' ');
            }
            transcript.push_str(&segment_text);
        }
    }

    let result = transcript.trim().to_string();

    if result.is_empty() {
        return Err("No speech detected in the audio".to_string());
    }

    Ok(result)
}

/// Unload the Whisper model to free memory
#[allow(dead_code)]
pub fn unload_model() {
    if let Ok(mut ctx_guard) = WHISPER_CTX.lock() {
        *ctx_guard = None;
    }
}
