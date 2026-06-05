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

Write-Host "Creating local virtual environment (.venv)..."
& $UvExe venv --python 3.11

Write-Host "Installing PyTorch, DeepFilterNet, and soundfile..."
& $UvExe pip install torch==2.5.1+cu121 torchaudio==2.5.1+cu121 --index-url https://download.pytorch.org/whl/cu121
& $UvExe pip install deepfilternet soundfile

Write-Host "Environment setup complete!"
