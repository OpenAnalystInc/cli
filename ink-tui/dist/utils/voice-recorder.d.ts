/**
 * voice-recorder — Cross-platform audio recording via child process.
 *
 * Records audio from the default microphone to a temporary WAV file.
 *
 * Platform detection:
 *   - Windows: ffmpeg (dshow) with PowerShell fallback for device detection
 *   - macOS:   sox (rec command)
 *   - Linux:   arecord (ALSA)
 *
 * The recorder spawns the recording tool as a child process, and stop()
 * terminates it gracefully. The WAV file is written to os.tmpdir().
 *
 * Max recording duration: 60 seconds (auto-stop safety).
 */
export interface RecordingSession {
    /** Start recording. Resolves when the recording process is spawned. */
    start(): Promise<void>;
    /** Stop recording. Returns the path to the recorded WAV file. */
    stop(): Promise<string>;
    /** Get current estimated audio level (0-100) for VU meter display. */
    getLevel(): number;
    /** Whether currently recording. */
    readonly isRecording: boolean;
    /** Elapsed recording time in milliseconds. */
    readonly elapsedMs: number;
}
export declare class RecorderError extends Error {
    readonly cause?: unknown | undefined;
    constructor(message: string, cause?: unknown | undefined);
}
/**
 * Create a new recording session.
 *
 * Each call creates a fresh recorder with a new temp file path.
 * Call start() to begin recording, stop() to end and get the WAV path.
 */
export declare function createRecorder(): RecordingSession;
/**
 * Clean up a temporary WAV file after transcription.
 */
export declare function cleanupRecording(wavPath: string): void;
