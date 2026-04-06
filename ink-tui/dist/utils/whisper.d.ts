/**
 * whisper — OpenAI Whisper API transcription client.
 *
 * Sends a WAV file to the OpenAI audio transcription endpoint and
 * returns the transcript text.
 *
 * Uses Node.js built-in `fetch` (available since Node 18) and
 * built-in `FormData` / `Blob` (available since Node 18).
 */
export interface TranscriptionResult {
    /** The transcribed text. */
    text: string;
    /** Detected language code (e.g. "en", "es"). */
    language: string;
    /** Duration of the audio in seconds. */
    duration: number;
}
export declare class WhisperError extends Error {
    readonly statusCode?: number | undefined;
    readonly cause?: unknown | undefined;
    constructor(message: string, statusCode?: number | undefined, cause?: unknown | undefined);
}
/**
 * Send a WAV file to OpenAI Whisper API for transcription.
 *
 * @param wavPath - Absolute path to the WAV file.
 * @param apiKey  - OpenAI API key (sk-...).
 * @returns Transcription result with text, language, and duration.
 * @throws {WhisperError} On API errors, network failures, or invalid responses.
 */
export declare function transcribe(wavPath: string, apiKey: string): Promise<TranscriptionResult>;
