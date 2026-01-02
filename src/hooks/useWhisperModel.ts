import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function useWhisperModel() {
  const [isModelReady, setIsModelReady] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  // Check if model is downloaded on mount
  useEffect(() => {
    invoke<boolean>("check_whisper_model")
      .then(setIsModelReady)
      .catch((e) => setError(String(e)));
  }, []);

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<number>("whisper-download-progress", (event) => {
      setDownloadProgress(event.payload);
      if (event.payload >= 100) {
        setIsModelReady(true);
        setIsDownloading(false);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Download the model
  const downloadModel = useCallback(async () => {
    try {
      setError(null);
      setIsDownloading(true);
      setDownloadProgress(0);

      await invoke("download_whisper_model");

      setIsModelReady(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsDownloading(false);
    }
  }, []);

  return {
    isModelReady,
    isDownloading,
    downloadProgress,
    downloadModel,
    error,
  };
}
