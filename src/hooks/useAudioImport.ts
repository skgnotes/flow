import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface UseAudioImportOptions {
  onTranscription: (text: string) => void;
  isModelReady: boolean;
}

export function useAudioImport({
  onTranscription,
  isModelReady,
}: UseAudioImportOptions) {
  const [isImporting, setIsImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const importAudioFile = useCallback(async () => {
    if (!isModelReady) {
      setError("Whisper model not ready");
      return;
    }

    try {
      setError(null);

      // Open file dialog
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Audio Files",
            extensions: ["mp3", "m4a", "wav", "ogg", "flac", "aac"],
          },
        ],
      });

      if (!selected) {
        // User cancelled
        return;
      }

      setIsImporting(true);

      // Transcribe the file
      const transcript = await invoke<string>("transcribe_audio_file", {
        path: selected,
      });

      onTranscription(transcript);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsImporting(false);
    }
  }, [isModelReady, onTranscription]);

  return {
    importAudioFile,
    isImporting,
    error,
  };
}
