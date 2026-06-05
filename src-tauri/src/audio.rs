use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub duration_sec: f64,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub media_type: String, // "audio" | "video"
    pub has_audio: bool,
    pub has_video: bool,
}

// Internal types for parsing ffprobe JSON output
#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Option<Vec<FfprobeStream>>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u16>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

// ---------------------------------------------------------------------------
// FFmpeg/FFprobe path resolution
// ---------------------------------------------------------------------------

/// Locate the ffmpeg binary inside the app's local `ffmpeg_bin/` directory.
pub fn ffmpeg_exe() -> Result<PathBuf, String> {
    find_tool("ffmpeg")
}

/// Locate the ffprobe binary inside the app's local `ffmpeg_bin/` directory.
pub fn ffprobe_exe() -> Result<PathBuf, String> {
    find_tool("ffprobe")
}

fn find_tool(name: &str) -> Result<PathBuf, String> {
    let exe_name = if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    // Try relative to current exe (production)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let candidate = parent.join("ffmpeg_bin").join(&exe_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    // Fallback for dev mode: check cwd and cwd/..
    let cwd = std::env::current_dir().unwrap_or_default();
    for base in [cwd.as_path(), cwd.join("..").as_path()] {
        let candidate = base.join("ffmpeg_bin").join(&exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "{} not found in ffmpeg_bin/. Please run the setup script first.",
        name
    ))
}

// ---------------------------------------------------------------------------
// Probe functions
// ---------------------------------------------------------------------------

/// Fast WAV-only probe using the hound crate (no external dependencies).
pub fn probe_wav(path: &str) -> Result<MediaInfo, String> {
    let reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels;
    let duration_samples = reader.duration() as f64;
    let duration_sec = duration_samples / sample_rate as f64 / channels as f64;

    Ok(MediaInfo {
        duration_sec,
        sample_rate: Some(sample_rate),
        channels: Some(channels),
        media_type: "audio".into(),
        has_audio: true,
        has_video: false,
    })
}

/// Generic media probe using ffprobe. Works with any format (WAV, MP3, MP4, etc.).
pub fn probe_media(path: &str) -> Result<MediaInfo, String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // For WAV files, try the fast path first
    if ext == "wav" {
        if let Ok(info) = probe_wav(path) {
            return Ok(info);
        }
    }

    // Use ffprobe for everything else (or as WAV fallback)
    let ffprobe = ffprobe_exe()?;

    let output = Command::new(&ffprobe)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_streams",
            "-show_format",
            path,
        ])
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe failed: {}", stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let probe: FfprobeOutput =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse ffprobe output: {}", e))?;

    let streams = probe.streams.unwrap_or_default();
    let has_audio = streams
        .iter()
        .any(|s| s.codec_type.as_deref() == Some("audio"));
    let has_video = streams
        .iter()
        .any(|s| s.codec_type.as_deref() == Some("video"));

    let audio_stream = streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"));

    let sample_rate = audio_stream
        .and_then(|s| s.sample_rate.as_ref())
        .and_then(|sr| sr.parse::<u32>().ok());

    let channels = audio_stream.and_then(|s| s.channels);

    let duration_sec = probe
        .format
        .and_then(|f| f.duration)
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    let media_type = if has_video { "video" } else { "audio" };

    Ok(MediaInfo {
        duration_sec,
        sample_rate,
        channels,
        media_type: media_type.into(),
        has_audio,
        has_video,
    })
}
