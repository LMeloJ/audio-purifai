#!/usr/bin/env bash
set -e

if ! command -v uv &> /dev/null; then
    echo "Installing uv package manager..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# ── FFmpeg ──────────────────────────────────────────────
FFMPEG_DIR="$PWD/ffmpeg_bin"
if [ ! -f "$FFMPEG_DIR/ffmpeg" ]; then
    echo "Downloading FFmpeg..."
    mkdir -p "$FFMPEG_DIR"
    curl -L "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz" -o /tmp/ffmpeg.tar.xz
    tar -xf /tmp/ffmpeg.tar.xz -C "$FFMPEG_DIR" --strip-components=1
    rm /tmp/ffmpeg.tar.xz
    echo "FFmpeg installed to $FFMPEG_DIR"
else
    echo "FFmpeg already present."
fi

echo "Creating local virtual environment (.venv)..."
uv venv --python 3.11

echo "Installing PyTorch, DeepFilterNet, and soundfile..."
uv pip install torch==2.5.1+cu121 torchaudio==2.5.1+cu121 --index-url https://download.pytorch.org/whl/cu121
uv pip install deepfilternet soundfile

echo "Environment setup complete!"
