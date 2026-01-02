use std::fs;
use std::io::Write;
use std::path::PathBuf;
use futures_util::StreamExt;
use tauri::{Emitter, Window};

const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";
const MODEL_FILENAME: &str = "ggml-base.en.bin";

pub fn get_models_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join("Documents")
        .join("Project Data Files")
        .join("Journal")
        .join("models")
}

pub fn get_model_path() -> PathBuf {
    get_models_dir().join(MODEL_FILENAME)
}

pub fn is_model_downloaded() -> bool {
    let path = get_model_path();
    if !path.exists() {
        return false;
    }
    // Check file size is reasonable (base.en is ~142MB)
    if let Ok(metadata) = fs::metadata(&path) {
        return metadata.len() > 100_000_000; // At least 100MB
    }
    false
}

#[tauri::command]
pub fn check_whisper_model() -> Result<bool, String> {
    Ok(is_model_downloaded())
}

#[tauri::command]
pub async fn download_whisper_model(window: Window) -> Result<(), String> {
    let models_dir = get_models_dir();
    fs::create_dir_all(&models_dir).map_err(|e| format!("Failed to create models directory: {}", e))?;

    let model_path = get_model_path();

    // If already downloaded, skip
    if is_model_downloaded() {
        let _ = window.emit("whisper-download-progress", 100u8);
        return Ok(());
    }

    // Download the model
    let client = reqwest::Client::new();
    let response = client
        .get(MODEL_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(142_000_000);
    let mut downloaded: u64 = 0;

    let mut file = fs::File::create(&model_path)
        .map_err(|e| format!("Failed to create model file: {}", e))?;

    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Download error: {}", e))?;

        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;
        let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u8;

        // Emit progress every ~1%
        let _ = window.emit("whisper-download-progress", progress);
    }

    file.flush().map_err(|e| format!("Failed to flush file: {}", e))?;

    // Verify download
    if !is_model_downloaded() {
        fs::remove_file(&model_path).ok();
        return Err("Download verification failed - file may be incomplete".to_string());
    }

    let _ = window.emit("whisper-download-progress", 100u8);
    Ok(())
}
