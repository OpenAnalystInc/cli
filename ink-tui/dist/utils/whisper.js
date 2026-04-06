/**
 * whisper — OpenAI Whisper API transcription client.
 *
 * Sends a WAV file to the OpenAI audio transcription endpoint and
 * returns the transcript text.
 *
 * Uses Node.js built-in `fetch` (available since Node 18) and
 * built-in `FormData` / `Blob` (available since Node 18).
 */
import fs from 'node:fs';
import path from 'node:path';
export class WhisperError extends Error {
    statusCode;
    cause;
    constructor(message, statusCode, cause) {
        super(message);
        this.statusCode = statusCode;
        this.cause = cause;
        this.name = 'WhisperError';
    }
}
// ---------------------------------------------------------------------------
// API endpoint
// ---------------------------------------------------------------------------
const WHISPER_API_URL = 'https://api.openai.com/v1/audio/transcriptions';
const WHISPER_MODEL = 'whisper-1';
// ---------------------------------------------------------------------------
// Transcription
// ---------------------------------------------------------------------------
/**
 * Send a WAV file to OpenAI Whisper API for transcription.
 *
 * @param wavPath - Absolute path to the WAV file.
 * @param apiKey  - OpenAI API key (sk-...).
 * @returns Transcription result with text, language, and duration.
 * @throws {WhisperError} On API errors, network failures, or invalid responses.
 */
export async function transcribe(wavPath, apiKey) {
    // Validate file exists
    let fileBuffer;
    try {
        fileBuffer = fs.readFileSync(wavPath);
    }
    catch (err) {
        throw new WhisperError(`Cannot read audio file: ${wavPath}`, undefined, err);
    }
    if (fileBuffer.length < 100) {
        throw new WhisperError('Audio file is too small — recording may have failed.');
    }
    // Build multipart form data
    const fileName = path.basename(wavPath);
    // Use ArrayBuffer to avoid TS strict Blob/Buffer incompatibility
    const arrayBuffer = fileBuffer.buffer.slice(fileBuffer.byteOffset, fileBuffer.byteOffset + fileBuffer.byteLength);
    const fileBlob = new Blob([arrayBuffer], { type: 'audio/wav' });
    const formData = new FormData();
    formData.append('file', fileBlob, fileName);
    formData.append('model', WHISPER_MODEL);
    formData.append('response_format', 'verbose_json');
    // Send request
    let response;
    try {
        response = await fetch(WHISPER_API_URL, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${apiKey}`,
            },
            body: formData,
        });
    }
    catch (err) {
        throw new WhisperError('Network error connecting to OpenAI API. Check your internet connection.', undefined, err);
    }
    // Handle HTTP errors
    if (!response.ok) {
        let errorMessage = `OpenAI API error (${response.status})`;
        try {
            const errorBody = await response.json();
            if (errorBody?.error?.message) {
                errorMessage = errorBody.error.message;
            }
        }
        catch {
            // Couldn't parse error body — use generic message
        }
        if (response.status === 401) {
            throw new WhisperError('Invalid OpenAI API key. Check your OPENAI_API_KEY in .env or ~/.openanalyst/.env', 401);
        }
        if (response.status === 429) {
            throw new WhisperError('OpenAI rate limit exceeded. Wait a moment and try again.', 429);
        }
        if (response.status === 413) {
            throw new WhisperError('Audio file too large. Try a shorter recording (max ~25MB).', 413);
        }
        throw new WhisperError(errorMessage, response.status);
    }
    // Parse successful response
    let body;
    try {
        body = await response.json();
    }
    catch (err) {
        throw new WhisperError('Failed to parse OpenAI API response.', undefined, err);
    }
    const result = body;
    if (!result.text && result.text !== '') {
        throw new WhisperError('OpenAI API returned an unexpected response format.');
    }
    return {
        text: result.text?.trim() ?? '',
        language: result.language ?? 'unknown',
        duration: result.duration ?? 0,
    };
}
//# sourceMappingURL=whisper.js.map