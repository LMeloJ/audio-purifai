"""
Persistent DeepFilterNet GPU worker.

Loads the model once at startup, then reads JSON commands from stdin
and writes JSON responses to stdout. This avoids re-loading the ~200MB
model for every audio file.

Protocol (newline-delimited JSON):
  → {"cmd":"enhance","id":"<uuid>","input":"<path>","output_dir":"<path>","post_filter":false,"media_type":"wav"}
  ← {"id":"<uuid>","status":"done","output":"<path>"}
  ← {"id":"<uuid>","status":"error","message":"..."}

  → {"cmd":"shutdown"}
  (process exits cleanly)

Supported media_type values: "wav", "mp3", "mp4"
"""

import json
import os
import sys
import tempfile
import traceback
import subprocess

# Monkey-patch check_output to gracefully handle missing executables (like 'git').
# DeepFilterNet's init_df tries to run 'git' for logging and crashes on systems without Git.
_original_check_output = subprocess.check_output

def _safe_check_output(*args, **kwargs):
    try:
        return _original_check_output(*args, **kwargs)
    except FileNotFoundError:
        # Simulate a non-zero exit code if the executable isn't found
        cmd = args[0] if args else kwargs.get("args", ["unknown"])
        raise subprocess.CalledProcessError(1, cmd)

subprocess.check_output = _safe_check_output

# Resolve FFmpeg/FFprobe paths from environment (set by Rust backend)
FFMPEG_PATH = os.environ.get("FFMPEG_PATH", "ffmpeg")
FFPROBE_PATH = os.environ.get("FFPROBE_PATH", "ffprobe")


def main():
    # Flush stdout line-by-line so Rust can read responses immediately
    # Redirect all warnings/logging to stderr so stdout stays clean JSON
    import warnings
    warnings.filterwarnings("ignore")

    import logging
    logging.disable(logging.CRITICAL)

    try:
        import torch
        import os
        
        # Force DeepFilterNet and PyTorch to use a local cache directory
        # to avoid polluting the user's global AppData folder.
        script_dir = os.path.dirname(os.path.abspath(__file__))
        app_root = os.path.dirname(script_dir)
        local_cache = os.path.join(app_root, ".venv", "df_cache")
        os.makedirs(local_cache, exist_ok=True)
        os.environ["TORCH_HOME"] = local_cache
        
        import df.utils
        df.utils.get_cache_dir = lambda: local_cache

        from df.enhance import init_df, enhance
        from df.io import load_audio, save_audio
    except Exception as e:
        emit({"status": "error", "message": f"Failed to initialize Python environment: {e}\n{traceback.format_exc()}"})
        sys.exit(1)

    # Load model into GPU (or CPU as fallback)
    try:
        device = "cuda" if torch.cuda.is_available() else "cpu"
        model, df_state, suffix = init_df(
            post_filter=False,
            log_level="ERROR",
            log_file=None,
            default_model="DeepFilterNet3",
        )
        model.eval()
        emit({"status": "ready", "device": device})
    except Exception as e:
        emit({"status": "error", "message": f"Failed to load model: {e}\n{traceback.format_exc()}"})
        sys.exit(1)

    # Main command loop
    for raw_line in sys.stdin:
        raw_line = raw_line.strip()
        if not raw_line:
            continue

        try:
            cmd = json.loads(raw_line)
        except json.JSONDecodeError as e:
            emit({"status": "error", "message": f"Invalid JSON: {e}"})
            continue

        if cmd.get("cmd") == "shutdown":
            break

        if cmd.get("cmd") == "enhance":
            process_enhance(cmd, model, df_state, suffix, enhance, load_audio, save_audio)
        else:
            emit({"status": "error", "message": f"Unknown command: {cmd.get('cmd')}"})


def process_enhance(cmd, model, df_state, suffix, enhance_fn, load_audio_fn, save_audio_fn):
    job_id = cmd.get("id", "unknown")
    input_path = cmd.get("input", "")
    output_dir = cmd.get("output_dir", "")
    post_filter = cmd.get("post_filter", False)
    media_type = cmd.get("media_type", "wav")

    temp_wav = None  # Track temp file for cleanup

    try:
        if not os.path.isfile(input_path):
            emit({"id": job_id, "status": "error", "message": f"File not found: {input_path}"})
            return

        # ── Pre-process: convert to WAV if needed ──────────────────────
        if media_type == "wav":
            audio_input = input_path
        elif media_type in ("mp3", "mp4"):
            # Extract/convert audio to temporary 48kHz mono WAV
            temp_wav = tempfile.mktemp(suffix=".wav", prefix="purifai_")
            ffmpeg_args = [
                FFMPEG_PATH, "-y",
                "-i", input_path,
                "-vn",                # strip video
                "-ar", str(df_state.sr()),  # resample to model's rate (48kHz)
                "-ac", "1",           # mono
                temp_wav,
            ]
            result = subprocess.run(
                ffmpeg_args,
                capture_output=True,
                text=True,
            )
            if result.returncode != 0:
                emit({"id": job_id, "status": "error", "message": f"FFmpeg pre-process failed: {result.stderr[:500]}"})
                return
            audio_input = temp_wav
        else:
            emit({"id": job_id, "status": "error", "message": f"Unsupported media type: {media_type}"})
            return

        # ── DeepFilterNet enhance ──────────────────────────────────────
        audio, meta = load_audio_fn(audio_input, sr=df_state.sr(), verbose=False)
        enhanced = enhance_fn(model, df_state, audio, pad=True)

        # ── Resolve output path ────────────────────────────────────────
        if not output_dir:
            output_dir = os.path.join(os.path.dirname(input_path), "enhanced")
        os.makedirs(output_dir, exist_ok=True)

        basename = os.path.splitext(os.path.basename(input_path))[0]
        enhanced_wav = os.path.join(output_dir, f"{basename}_{suffix}.wav")
        save_audio_fn(enhanced_wav, enhanced, sr=df_state.sr())

        # ── Post-process: remux video if MP4 ───────────────────────────
        if media_type == "mp4":
            output_mp4 = os.path.join(output_dir, f"{basename}_{suffix}.mp4")
            remux_args = [
                FFMPEG_PATH, "-y",
                "-i", input_path,       # original video
                "-i", enhanced_wav,     # enhanced audio
                "-c:v", "copy",         # copy video stream (no re-encode)
                "-map", "0:v:0",        # video from first input
                "-map", "1:a:0",        # audio from second input
                "-shortest",
                output_mp4,
            ]
            result = subprocess.run(
                remux_args,
                capture_output=True,
                text=True,
            )
            if result.returncode != 0:
                emit({"id": job_id, "status": "error", "message": f"FFmpeg remux failed: {result.stderr[:500]}"})
                return
            # Clean up intermediate WAV
            try:
                os.remove(enhanced_wav)
            except OSError:
                pass
            final_output = output_mp4
        else:
            final_output = enhanced_wav

        emit({"id": job_id, "status": "done", "output": final_output})

    except Exception as e:
        emit({"id": job_id, "status": "error", "message": str(e)})

    finally:
        # Clean up temp WAV from pre-processing
        if temp_wav and os.path.exists(temp_wav):
            try:
                os.remove(temp_wav)
            except OSError:
                pass


def emit(obj):
    """Write a JSON object to stdout, one per line, and flush immediately."""
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


if __name__ == "__main__":
    main()
