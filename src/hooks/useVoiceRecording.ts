import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";

export type RecordingState = "idle" | "recording" | "transcribing";

interface UseVoiceRecordingOptions {
  onTranscription: (text: string) => void;
  isModelReady: boolean;
}

export function useVoiceRecording({
  onTranscription,
  isModelReady,
}: UseVoiceRecordingOptions) {
  const [state, setState] = useState<RecordingState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [recordingDuration, setRecordingDuration] = useState(0);
  const timerRef = useRef<number | null>(null);

  // Start recording
  const startRecording = useCallback(async () => {
    if (!isModelReady) {
      setError("Whisper model not ready");
      return;
    }

    if (state !== "idle") return;

    try {
      setError(null);
      await invoke("start_recording");
      setState("recording");
      setRecordingDuration(0);

      // Start duration timer
      timerRef.current = window.setInterval(() => {
        setRecordingDuration((d) => d + 1);
      }, 1000);
    } catch (e) {
      setError(String(e));
    }
  }, [isModelReady, state]);

  // Stop recording and transcribe
  const stopRecording = useCallback(async () => {
    if (state !== "recording") return;

    // Stop timer
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    try {
      setState("transcribing");
      const transcript = await invoke<string>("stop_recording_and_transcribe");
      onTranscription(transcript);
      setState("idle");
      setRecordingDuration(0);
    } catch (e) {
      setError(String(e));
      setState("idle");
      setRecordingDuration(0);
    }
  }, [state, onTranscription]);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
    };
  }, []);

  // Register global shortcut (Cmd+Shift+R)
  useEffect(() => {
    if (!isModelReady) return;

    const shortcut = "CommandOrControl+Shift+R";
    let isRegistered = false;

    const registerShortcut = async () => {
      try {
        await register(shortcut, async (event) => {
          if (event.state === "Pressed") {
            // Start recording on key down
            if (state === "idle") {
              try {
                setError(null);
                await invoke("start_recording");
                setState("recording");
                setRecordingDuration(0);
                timerRef.current = window.setInterval(() => {
                  setRecordingDuration((d) => d + 1);
                }, 1000);
              } catch (e) {
                setError(String(e));
              }
            }
          } else if (event.state === "Released") {
            // Stop recording on key up
            if (timerRef.current) {
              clearInterval(timerRef.current);
              timerRef.current = null;
            }
            try {
              setState("transcribing");
              const transcript = await invoke<string>("stop_recording_and_transcribe");
              onTranscription(transcript);
              setState("idle");
              setRecordingDuration(0);
            } catch (e) {
              setError(String(e));
              setState("idle");
              setRecordingDuration(0);
            }
          }
        });
        isRegistered = true;
      } catch (e) {
        console.warn("Failed to register global shortcut:", e);
      }
    };

    registerShortcut();

    return () => {
      if (isRegistered) {
        unregister(shortcut).catch(console.warn);
      }
    };
  }, [isModelReady, onTranscription]);

  // Format duration as mm:ss
  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  return {
    state,
    error,
    recordingDuration,
    formattedDuration: formatDuration(recordingDuration),
    startRecording,
    stopRecording,
    isRecording: state === "recording",
    isTranscribing: state === "transcribing",
  };
}
