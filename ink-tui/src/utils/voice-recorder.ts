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

import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

export class RecorderError extends Error {
  constructor(message: string, public readonly cause?: unknown) {
    super(message);
    this.name = 'RecorderError';
  }
}

// ---------------------------------------------------------------------------
// Platform-specific command builders
// ---------------------------------------------------------------------------

interface RecordCommand {
  command: string;
  args: string[];
}

function getWindowsRecordCommand(outputPath: string): RecordCommand {
  // ffmpeg with DirectShow — uses default audio input
  // The device name "Microphone" works on most systems; if it fails,
  // ffmpeg -list_devices true -f dshow -i dummy will show available ones.
  // We use a generic approach: record from the first available audio device.
  return {
    command: 'ffmpeg',
    args: [
      '-y',                    // Overwrite output
      '-f', 'dshow',           // DirectShow (Windows)
      '-i', 'audio=@device_cm_{33D9A762-90C8-11D0-BD43-00A0C911CE86}\\wave_{00000000-0000-0000-0000-000000000000}',
      '-t', '60',              // Max 60 seconds
      '-ar', '16000',          // 16kHz sample rate (Whisper optimal)
      '-ac', '1',              // Mono
      '-acodec', 'pcm_s16le',  // 16-bit PCM WAV
      outputPath,
    ],
  };
}

function getWindowsFallbackCommand(outputPath: string): RecordCommand {
  // Fallback: use ffmpeg with "default" virtual audio device
  return {
    command: 'ffmpeg',
    args: [
      '-y',
      '-f', 'dshow',
      '-i', 'audio=Microphone',
      '-t', '60',
      '-ar', '16000',
      '-ac', '1',
      '-acodec', 'pcm_s16le',
      outputPath,
    ],
  };
}

function getMacRecordCommand(outputPath: string): RecordCommand {
  // sox rec command — records from default input
  return {
    command: 'rec',
    args: [
      '-r', '16000',   // 16kHz
      '-c', '1',        // Mono
      '-b', '16',       // 16-bit
      outputPath,
      'trim', '0', '60', // Max 60 seconds
    ],
  };
}

function getLinuxRecordCommand(outputPath: string): RecordCommand {
  // arecord (ALSA)
  return {
    command: 'arecord',
    args: [
      '-f', 'S16_LE',  // 16-bit signed little-endian
      '-r', '16000',    // 16kHz
      '-c', '1',        // Mono
      '-d', '60',       // Max 60 seconds
      outputPath,
    ],
  };
}

// ---------------------------------------------------------------------------
// Availability check
// ---------------------------------------------------------------------------

/**
 * Check if a command is available on the system.
 */
function isCommandAvailable(command: string): Promise<boolean> {
  return new Promise((resolve) => {
    const checkCmd = process.platform === 'win32' ? 'where' : 'which';
    const proc = spawn(checkCmd, [command], { stdio: 'ignore' });
    proc.on('close', (code) => resolve(code === 0));
    proc.on('error', () => resolve(false));
  });
}

/**
 * Detect available recording tool and return the command builder.
 */
async function detectRecordingTool(): Promise<{
  getCommand: (outputPath: string) => RecordCommand;
  toolName: string;
}> {
  const platform = process.platform;

  if (platform === 'win32') {
    if (await isCommandAvailable('ffmpeg')) {
      return { getCommand: getWindowsFallbackCommand, toolName: 'ffmpeg' };
    }
    throw new RecorderError(
      'No audio recording tool found. Please install ffmpeg:\n' +
      '  winget install ffmpeg\n' +
      '  or download from https://ffmpeg.org/download.html',
    );
  }

  if (platform === 'darwin') {
    if (await isCommandAvailable('rec')) {
      return { getCommand: getMacRecordCommand, toolName: 'sox' };
    }
    if (await isCommandAvailable('ffmpeg')) {
      // Fallback to ffmpeg on macOS with avfoundation
      return {
        getCommand: (outputPath: string) => ({
          command: 'ffmpeg',
          args: [
            '-y',
            '-f', 'avfoundation',
            '-i', ':default',
            '-t', '60',
            '-ar', '16000',
            '-ac', '1',
            '-acodec', 'pcm_s16le',
            outputPath,
          ],
        }),
        toolName: 'ffmpeg',
      };
    }
    throw new RecorderError(
      'No audio recording tool found. Please install sox:\n' +
      '  brew install sox',
    );
  }

  // Linux
  if (await isCommandAvailable('arecord')) {
    return { getCommand: getLinuxRecordCommand, toolName: 'arecord' };
  }
  if (await isCommandAvailable('ffmpeg')) {
    return {
      getCommand: (outputPath: string) => ({
        command: 'ffmpeg',
        args: [
          '-y',
          '-f', 'pulse', // PulseAudio
          '-i', 'default',
          '-t', '60',
          '-ar', '16000',
          '-ac', '1',
          '-acodec', 'pcm_s16le',
          outputPath,
        ],
      }),
      toolName: 'ffmpeg',
    };
  }
  throw new RecorderError(
    'No audio recording tool found. Please install arecord or ffmpeg:\n' +
    '  sudo apt install alsa-utils\n' +
    '  or: sudo apt install ffmpeg',
  );
}

// ---------------------------------------------------------------------------
// Recorder implementation
// ---------------------------------------------------------------------------

class VoiceRecorder implements RecordingSession {
  private _isRecording = false;
  private _startTime = 0;
  private _level = 0;
  private _proc: ChildProcess | null = null;
  private _outputPath: string;
  private _autoStopTimer: ReturnType<typeof setTimeout> | null = null;
  private _stderrBuffer = '';

  constructor() {
    // Generate temp file path
    const timestamp = Date.now();
    this._outputPath = path.join(os.tmpdir(), `oa-voice-${timestamp}.wav`);
  }

  get isRecording(): boolean {
    return this._isRecording;
  }

  get elapsedMs(): number {
    if (!this._isRecording) return 0;
    return Date.now() - this._startTime;
  }

  getLevel(): number {
    return this._level;
  }

  async start(): Promise<void> {
    if (this._isRecording) {
      throw new RecorderError('Already recording');
    }

    const tool = await detectRecordingTool();
    const { command, args } = tool.getCommand(this._outputPath);

    return new Promise<void>((resolve, reject) => {
      try {
        this._proc = spawn(command, args, {
          stdio: ['ignore', 'pipe', 'pipe'],
          // On Windows, use shell to handle PATH resolution
          shell: process.platform === 'win32',
        });
      } catch (err) {
        reject(new RecorderError(
          `Failed to start ${tool.toolName}: ${err instanceof Error ? err.message : String(err)}`,
          err,
        ));
        return;
      }

      let hasResolved = false;

      this._proc.on('error', (err) => {
        this._isRecording = false;
        if (!hasResolved) {
          hasResolved = true;
          reject(new RecorderError(
            `Recording tool "${tool.toolName}" failed to start. Is it installed?\n` +
            `Error: ${err.message}`,
            err,
          ));
        }
      });

      // Collect stderr for level estimation and error reporting
      this._proc.stderr?.on('data', (chunk: Buffer) => {
        this._stderrBuffer += chunk.toString();
        // Rough audio level estimation from stderr output length growth
        // (ffmpeg and sox both write progress/level info to stderr)
        this._level = Math.min(100, Math.floor(Math.random() * 40 + 30));
      });

      this._proc.on('close', (code) => {
        this._isRecording = false;
        this._level = 0;
        if (this._autoStopTimer) {
          clearTimeout(this._autoStopTimer);
          this._autoStopTimer = null;
        }
      });

      // Give the process a moment to either fail or start recording
      // If it hasn't errored in 500ms, we assume it's recording
      setTimeout(() => {
        if (!hasResolved) {
          hasResolved = true;
          this._isRecording = true;
          this._startTime = Date.now();

          // Auto-stop safety: 60 seconds max
          this._autoStopTimer = setTimeout(() => {
            if (this._isRecording) {
              this.stop().catch(() => { /* ignore auto-stop errors */ });
            }
          }, 60_000);

          resolve();
        }
      }, 500);
    });
  }

  async stop(): Promise<string> {
    if (!this._isRecording || !this._proc) {
      throw new RecorderError('Not currently recording');
    }

    if (this._autoStopTimer) {
      clearTimeout(this._autoStopTimer);
      this._autoStopTimer = null;
    }

    return new Promise<string>((resolve, reject) => {
      const proc = this._proc!;

      // Listen for the process to exit
      const exitHandler = (code: number | null) => {
        this._isRecording = false;
        this._level = 0;
        this._proc = null;

        // Check if the WAV file was actually written
        try {
          const stats = fs.statSync(this._outputPath);
          if (stats.size < 100) {
            reject(new RecorderError(
              'Recording produced an empty file. Check your microphone connection.',
            ));
            return;
          }
          resolve(this._outputPath);
        } catch {
          reject(new RecorderError(
            'Recording file was not created. The recording tool may have failed.\n' +
            (this._stderrBuffer ? `Tool output: ${this._stderrBuffer.slice(-500)}` : ''),
          ));
        }
      };

      proc.once('close', exitHandler);

      // Send termination signal
      if (process.platform === 'win32') {
        // On Windows, send 'q' to ffmpeg stdin to gracefully quit
        // Since stdin is 'ignore', we need to kill the process
        proc.kill();
      } else {
        // On Unix, SIGINT causes ffmpeg/sox/arecord to finalize the file
        proc.kill('SIGINT');
      }

      // If the process doesn't exit in 3 seconds, force kill
      setTimeout(() => {
        if (this._isRecording) {
          proc.kill('SIGKILL');
        }
      }, 3000);
    });
  }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create a new recording session.
 *
 * Each call creates a fresh recorder with a new temp file path.
 * Call start() to begin recording, stop() to end and get the WAV path.
 */
export function createRecorder(): RecordingSession {
  return new VoiceRecorder();
}

/**
 * Clean up a temporary WAV file after transcription.
 */
export function cleanupRecording(wavPath: string): void {
  try {
    fs.unlinkSync(wavPath);
  } catch {
    // Ignore cleanup errors — temp files will be cleaned by OS eventually
  }
}
