/**
 * useVoice — React hook orchestrating voice recording + Whisper transcription.
 *
 * Lifecycle:
 *   1. On mount: detect API key availability
 *   2. startRecording(): spawn recorder, enter voice_recording mode
 *   3. stopRecording(): kill recorder, send WAV to Whisper, return transcript
 *   4. Cleanup: remove temp WAV, exit voice_recording mode
 *
 * If no API key is found, startRecording() pushes a system message
 * explaining how to set one up — it does NOT throw.
 *
 * Priority 7 in the keypress system (between scroll mode and autocomplete).
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { findOpenAIKey, type ApiKeyResult } from '../utils/api-key.js';
import {
  createRecorder,
  cleanupRecording,
  type RecordingSession,
  RecorderError,
} from '../utils/voice-recorder.js';
import { transcribe, WhisperError } from '../utils/whisper.js';
import { useUIActions } from '../contexts/ui-state-context.js';
import { useChatActions } from '../contexts/chat-context.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface UseVoiceReturn {
  /** Begin recording audio from the microphone. */
  startRecording(): void;
  /** Stop recording and transcribe. Returns transcript text or null on error. */
  stopRecording(): Promise<string | null>;
  /** Cancel recording without transcribing. */
  cancelRecording(): void;
  /** Whether a recording is currently in progress. */
  isRecording: boolean;
  /** Elapsed recording time in milliseconds. */
  elapsedMs: number;
  /** Current audio level (0-100) for VU meter. */
  level: number;
  /** Last error message, or null. */
  error: string | null;
  /** Whether an OpenAI API key was found. */
  apiKeyAvailable: boolean;
  /** Where the API key was found. */
  apiKeySource: ApiKeyResult['source'];
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NO_API_KEY_MESSAGE =
  'Voice input requires an OpenAI API key for Whisper transcription.\n\n' +
  'Set OPENAI_API_KEY in one of these locations:\n' +
  '  1. Project .env file: OPENAI_API_KEY=sk-...\n' +
  '  2. Global config:     ~/.openanalyst/.env\n' +
  '  3. Environment:       export OPENAI_API_KEY=sk-...';

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useVoice(): UseVoiceReturn {
  const uiActions = useUIActions();
  const chatActions = useChatActions();

  // API key state (checked once on mount)
  const apiKeyRef = useRef<ApiKeyResult>({ key: null, source: null });
  const [apiKeyAvailable, setApiKeyAvailable] = useState(false);
  const [apiKeySource, setApiKeySource] = useState<ApiKeyResult['source']>(null);

  // Recording state
  const recorderRef = useRef<RecordingSession | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [elapsedMs, setElapsedMs] = useState(0);
  const [level, setLevel] = useState(0);
  const [error, setError] = useState<string | null>(null);

  // Elapsed time ticker
  const tickerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Check for API key on mount
  useEffect(() => {
    const result = findOpenAIKey();
    apiKeyRef.current = result;
    setApiKeyAvailable(result.key !== null);
    setApiKeySource(result.source);
  }, []);

  // Elapsed time + level polling while recording
  useEffect(() => {
    if (isRecording) {
      tickerRef.current = setInterval(() => {
        const recorder = recorderRef.current;
        if (recorder && recorder.isRecording) {
          setElapsedMs(recorder.elapsedMs);
          setLevel(recorder.getLevel());
        }
      }, 100);
    } else {
      if (tickerRef.current) {
        clearInterval(tickerRef.current);
        tickerRef.current = null;
      }
      setElapsedMs(0);
      setLevel(0);
    }

    return () => {
      if (tickerRef.current) {
        clearInterval(tickerRef.current);
        tickerRef.current = null;
      }
    };
  }, [isRecording]);

  // ── Start recording ──
  const startRecording = useCallback(() => {
    setError(null);

    // Re-check API key (user might have added it since mount)
    const keyResult = findOpenAIKey();
    apiKeyRef.current = keyResult;
    setApiKeyAvailable(keyResult.key !== null);
    setApiKeySource(keyResult.source);

    if (!keyResult.key) {
      chatActions.pushSystem(NO_API_KEY_MESSAGE, 'warning');
      return;
    }

    // Create recorder and start
    const recorder = createRecorder();
    recorderRef.current = recorder;

    recorder.start()
      .then(() => {
        setIsRecording(true);
        uiActions.setVoiceRecording(true);
      })
      .catch((err: unknown) => {
        recorderRef.current = null;
        const message = err instanceof RecorderError
          ? err.message
          : `Failed to start recording: ${err instanceof Error ? err.message : String(err)}`;
        setError(message);
        chatActions.pushSystem(message, 'error');
      });
  }, [chatActions, uiActions]);

  // ── Stop recording + transcribe ──
  const stopRecording = useCallback(async (): Promise<string | null> => {
    const recorder = recorderRef.current;
    const apiKey = apiKeyRef.current.key;

    if (!recorder || !recorder.isRecording) {
      setIsRecording(false);
      uiActions.setVoiceRecording(false);
      return null;
    }

    setError(null);

    let wavPath: string;
    try {
      wavPath = await recorder.stop();
    } catch (err) {
      recorderRef.current = null;
      setIsRecording(false);
      uiActions.setVoiceRecording(false);

      const message = err instanceof RecorderError
        ? err.message
        : `Recording failed: ${err instanceof Error ? err.message : String(err)}`;
      setError(message);
      chatActions.pushSystem(message, 'error');
      return null;
    }

    recorderRef.current = null;
    setIsRecording(false);
    uiActions.setVoiceRecording(false);

    if (!apiKey) {
      cleanupRecording(wavPath);
      chatActions.pushSystem(NO_API_KEY_MESSAGE, 'warning');
      return null;
    }

    // Transcribe
    chatActions.pushSystem('Transcribing audio...', 'info');

    try {
      const result = await transcribe(wavPath, apiKey);
      cleanupRecording(wavPath);

      if (!result.text) {
        chatActions.pushSystem('No speech detected in the recording.', 'warning');
        return null;
      }

      return result.text;
    } catch (err) {
      cleanupRecording(wavPath);

      const message = err instanceof WhisperError
        ? err.message
        : `Transcription failed: ${err instanceof Error ? err.message : String(err)}`;
      setError(message);
      chatActions.pushSystem(message, 'error');
      return null;
    }
  }, [chatActions, uiActions]);

  // ── Cancel recording (no transcription) ──
  const cancelRecording = useCallback(() => {
    const recorder = recorderRef.current;
    if (recorder && recorder.isRecording) {
      recorder.stop()
        .then((wavPath) => cleanupRecording(wavPath))
        .catch(() => { /* ignore cancel errors */ });
    }
    recorderRef.current = null;
    setIsRecording(false);
    setError(null);
    uiActions.setVoiceRecording(false);
  }, [uiActions]);

  return {
    startRecording,
    stopRecording,
    cancelRecording,
    isRecording,
    elapsedMs,
    level,
    error,
    apiKeyAvailable,
    apiKeySource,
  };
}
