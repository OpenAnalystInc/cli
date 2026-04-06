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
import { type ApiKeyResult } from '../utils/api-key.js';
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
export declare function useVoice(): UseVoiceReturn;
