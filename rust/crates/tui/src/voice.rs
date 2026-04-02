//! Voice input — microphone capture with audio level visualization and Whisper transcription.
//!
//! Activated via Ctrl+Space. Records audio from the default input device, displays
//! a blue-bordered VU meter in the input area, and transcribes via Whisper API on stop.

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Voice recording state shared between the audio thread and TUI.
#[derive(Clone)]
pub struct VoiceState {
    /// Whether recording is currently active.
    pub is_recording: Arc<AtomicBool>,
    /// Current audio level (0-100 scale) for VU meter display.
    pub audio_level: Arc<AtomicU16>,
    /// Peak audio level (decays over time).
    pub peak_level: Arc<AtomicU16>,
    /// Recorded audio samples (i16 PCM, mono).
    samples: Arc<std::sync::Mutex<Vec<i16>>>,
    /// Sample rate of the recording.
    sample_rate: Arc<AtomicU16>,
    /// When recording started.
    pub start_time: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_level: Arc::new(AtomicU16::new(0)),
            peak_level: Arc::new(AtomicU16::new(0)),
            samples: Arc::new(std::sync::Mutex::new(Vec::new())),
            sample_rate: Arc::new(AtomicU16::new(16000)),
            start_time: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl VoiceState {
    /// Start recording from the default input device.
    /// Returns Err if no input device is available.
    pub fn start_recording(&self) -> Result<(), String> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Ok(()); // Already recording
        }

        // Clear previous samples
        if let Ok(mut samples) = self.samples.lock() {
            samples.clear();
        }
        *self.start_time.lock().unwrap() = Some(Instant::now());
        self.audio_level.store(0, Ordering::SeqCst);
        self.peak_level.store(0, Ordering::SeqCst);

        let is_recording = self.is_recording.clone();
        let audio_level = self.audio_level.clone();
        let peak_level = self.peak_level.clone();
        let samples = self.samples.clone();
        let sample_rate_atomic = self.sample_rate.clone();

        // Spawn audio capture on a dedicated thread (cpal requires its own thread)
        std::thread::Builder::new()
            .name("voice-capture".to_string())
            .spawn(move || {
                if let Err(e) = run_capture(is_recording, audio_level, peak_level, samples, sample_rate_atomic) {
                    eprintln!("[voice] Capture error: {e}");
                }
            })
            .map_err(|e| format!("Failed to spawn audio thread: {e}"))?;

        // Mark as recording after thread spawns
        self.is_recording.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Stop recording and return the WAV data as bytes.
    pub fn stop_recording(&self) -> Option<Vec<u8>> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return None;
        }

        self.is_recording.store(false, Ordering::SeqCst);
        *self.start_time.lock().unwrap() = None;

        // Give the capture thread a moment to flush
        std::thread::sleep(std::time::Duration::from_millis(100));

        let samples = self.samples.lock().ok()?;
        if samples.is_empty() {
            return None;
        }

        let rate = self.sample_rate.load(Ordering::SeqCst) as u32;
        encode_wav(&samples, rate)
    }

    /// Get recording duration in seconds.
    pub fn recording_duration(&self) -> f64 {
        self.start_time
            .lock()
            .ok()
            .and_then(|t| t.map(|start| start.elapsed().as_secs_f64()))
            .unwrap_or(0.0)
    }

    /// Get the current audio level (0-100).
    pub fn level(&self) -> u16 {
        self.audio_level.load(Ordering::Relaxed)
    }

    /// Get the peak level (0-100).
    pub fn peak(&self) -> u16 {
        self.peak_level.load(Ordering::Relaxed)
    }
}

/// Run audio capture from the default input device.
fn run_capture(
    is_recording: Arc<AtomicBool>,
    audio_level: Arc<AtomicU16>,
    peak_level: Arc<AtomicU16>,
    samples: Arc<std::sync::Mutex<Vec<i16>>>,
    sample_rate_atomic: Arc<AtomicU16>,
) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("No input config: {e}"))?;

    let rate = config.sample_rate().0;
    // Store sample rate (capped to u16 max — 48000 fits)
    sample_rate_atomic.store(rate.min(u32::from(u16::MAX)) as u16, Ordering::SeqCst);

    let channels = config.channels() as usize;
    let is_rec = is_recording.clone();
    let level = audio_level.clone();
    let peak = peak_level.clone();
    let samps = samples.clone();

    let stream = match config.sample_format() {
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !is_rec.load(Ordering::SeqCst) {
                        return;
                    }
                    process_samples(data, channels, &level, &peak, &samps);
                },
                |err| eprintln!("[voice] Stream error: {err}"),
                None,
            )
            .map_err(|e| format!("Build stream: {e}"))?,
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_rec.load(Ordering::SeqCst) {
                        return;
                    }
                    // Convert f32 to i16
                    let i16_data: Vec<i16> = data
                        .iter()
                        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                        .collect();
                    process_samples(&i16_data, channels, &level, &peak, &samps);
                },
                |err| eprintln!("[voice] Stream error: {err}"),
                None,
            )
            .map_err(|e| format!("Build stream: {e}"))?,
        fmt => return Err(format!("Unsupported sample format: {fmt:?}")),
    };

    stream.play().map_err(|e| format!("Play stream: {e}"))?;

    // Keep the stream alive until recording stops
    while is_recording.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    drop(stream);
    Ok(())
}

/// Process incoming audio samples: compute level, store mono samples.
fn process_samples(
    data: &[i16],
    channels: usize,
    level: &Arc<AtomicU16>,
    peak: &Arc<AtomicU16>,
    samples: &Arc<std::sync::Mutex<Vec<i16>>>,
) {
    if data.is_empty() {
        return;
    }

    // Compute RMS level from the chunk
    let sum: f64 = data.iter().map(|&s| (s as f64) * (s as f64)).sum();
    let rms = (sum / data.len() as f64).sqrt();
    // Scale to 0-100 (i16 max is 32767)
    let normalized = ((rms / 32767.0) * 200.0).min(100.0) as u16;
    level.store(normalized, Ordering::Relaxed);

    // Update peak with slow decay
    let current_peak = peak.load(Ordering::Relaxed);
    if normalized > current_peak {
        peak.store(normalized, Ordering::Relaxed);
    } else if current_peak > 0 {
        peak.store(current_peak.saturating_sub(1), Ordering::Relaxed);
    }

    // Store mono samples (take first channel only if stereo)
    if let Ok(mut samps) = samples.lock() {
        if channels <= 1 {
            samps.extend_from_slice(data);
        } else {
            samps.extend(data.chunks(channels).map(|chunk| chunk[0]));
        }
    }
}

/// Encode samples as WAV bytes.
fn encode_wav(samples: &[i16], sample_rate: u32) -> Option<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = std::io::Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec).ok()?;
    for &sample in samples {
        writer.write_sample(sample).ok()?;
    }
    writer.finalize().ok()?;
    Some(cursor.into_inner())
}

/// Transcribe WAV audio via Whisper API.
/// Tries OpenAI Whisper first, falls back to Gemini.
pub fn transcribe_audio(wav_data: &[u8]) -> Result<String, String> {
    // Try OpenAI Whisper
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() {
            return transcribe_openai(wav_data, &api_key);
        }
    }

    // Try Gemini
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        if !api_key.is_empty() {
            return transcribe_gemini(wav_data, &api_key);
        }
    }

    Err("No transcription API key found. Set OPENAI_API_KEY or GEMINI_API_KEY.".to_string())
}

fn transcribe_openai(wav_data: &[u8], api_key: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;

    let part = reqwest::blocking::multipart::Part::bytes(wav_data.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Multipart: {e}"))?;

    let form = reqwest::blocking::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", part);

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .map_err(|e| format!("Whisper request: {e}"))?;

    let status = resp.status();
    let body = resp.text().map_err(|e| format!("Read response: {e}"))?;

    if !status.is_success() {
        return Err(format!("Whisper API {status}: {body}"));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Parse: {e}"))?;
    json.get("text")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No text in Whisper response".to_string())
}

fn transcribe_gemini(wav_data: &[u8], api_key: &str) -> Result<String, String> {
    use base64::Engine;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;

    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(wav_data);
    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                {"text": "Transcribe this audio exactly. Return only the transcription, nothing else."},
                {
                    "inline_data": {
                        "mime_type": "audio/wav",
                        "data": audio_b64
                    }
                }
            ]
        }]
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
    );

    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .map_err(|e| format!("Gemini request: {e}"))?;

    let status = resp.status();
    let body = resp.text().map_err(|e| format!("Read: {e}"))?;

    if !status.is_success() {
        return Err(format!("Gemini API {status}: {body}"));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Parse: {e}"))?;

    json.get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "No text in Gemini response".to_string())
}

/// Render the voice input VU meter as styled lines for the input area.
/// Returns lines to display when voice recording is active.
pub fn render_voice_indicator(state: &VoiceState, width: u16) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    let level = state.level() as usize;
    let peak = state.peak() as usize;
    let duration = state.recording_duration();
    let bar_width = (width as usize).saturating_sub(4);

    // Build VU meter bar
    let filled = (level * bar_width) / 100;
    let peak_pos = (peak * bar_width) / 100;

    let mut bar_chars = Vec::new();
    for i in 0..bar_width {
        if i < filled {
            // Active level — blue gradient
            let color = if i < bar_width / 3 {
                Color::Rgb(50, 130, 255) // Low — blue
            } else if i < bar_width * 2 / 3 {
                Color::Rgb(80, 200, 255) // Mid — cyan
            } else {
                Color::Rgb(255, 140, 50) // High — orange
            };
            bar_chars.push(Span::styled("█", Style::default().fg(color)));
        } else if i == peak_pos && peak > 0 {
            bar_chars.push(Span::styled("│", Style::default().fg(Color::Rgb(100, 180, 255))));
        } else {
            bar_chars.push(Span::styled("░", Style::default().fg(Color::Indexed(237))));
        }
    }

    let header = Line::from(vec![
        Span::styled(" ● ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("Recording", Style::default().fg(Color::Rgb(50, 130, 255)).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {duration:.1}s"), Style::default().fg(Color::Indexed(245))),
    ]);

    let mut meter_line = vec![Span::styled(" ", Style::default())];
    meter_line.extend(bar_chars);

    let hint = Line::from(vec![
        Span::styled(" Space", Style::default().fg(Color::Indexed(245)).add_modifier(Modifier::BOLD)),
        Span::styled(" to stop and transcribe", Style::default().fg(Color::Indexed(240))),
    ]);

    vec![header, Line::from(meter_line), hint]
}
