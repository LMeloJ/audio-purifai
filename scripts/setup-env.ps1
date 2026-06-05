$ErrorActionPreference = "Stop"

$AppDir = $PWD.Path
$env:UV_INSTALL_DIR = Join-Path $AppDir "uv_bin"
$env:UV_PYTHON_INSTALL_DIR = Join-Path $AppDir "uv_python"
$env:UV_CACHE_DIR = Join-Path $AppDir "uv_cache"

$UvExe = "uv"
if (-not (Get-Command uv -ErrorAction Ignore)) {
    Write-Host "Installing uv package manager..."
    & powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
    $UvExe = Join-Path $env:UV_INSTALL_DIR "uv.exe"
}

# ── FFmpeg ──────────────────────────────────────────────
$FfmpegDir = Join-Path $AppDir "ffmpeg_bin"
$FfmpegExe = Join-Path $FfmpegDir "ffmpeg.exe"
if (-not (Test-Path $FfmpegExe)) {
    Write-Host "Downloading FFmpeg..."
    $FfmpegZip = Join-Path $AppDir "ffmpeg.zip"
    $FfmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
    Invoke-WebRequest -Uri $FfmpegUrl -OutFile $FfmpegZip
    Expand-Archive -Path $FfmpegZip -DestinationPath $FfmpegDir -Force
    # Move binaries from nested folder to ffmpeg_bin root
    $Nested = Get-ChildItem $FfmpegDir -Directory | Select-Object -First 1
    if ($Nested) {
        Move-Item (Join-Path $Nested.FullName "bin\*") $FfmpegDir -Force
        Remove-Item $Nested.FullName -Recurse -Force
    }
    Remove-Item $FfmpegZip -Force
    Write-Host "FFmpeg installed to $FfmpegDir"
} else {
    Write-Host "FFmpeg already present."
}

Write-Host "Creating local virtual environment (.venv)..."
& $UvExe venv --python 3.11

Write-Host "Installing PyTorch, DeepFilterNet, and soundfile..."
& $UvExe pip install torch==2.5.1+cu121 torchaudio==2.5.1+cu121 --index-url https://download.pytorch.org/whl/cu121
& $UvExe pip install deepfilternet soundfile

Write-Host "Environment setup complete!"
