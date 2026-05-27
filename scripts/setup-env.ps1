$ErrorActionPreference = "Stop"

if (-not (Get-Command uv -ErrorAction Ignore)) {
    Write-Host "Installing uv package manager..."
    irm https://astral.sh/uv/install.ps1 | iex
    $env:Path += ";$HOME\.cargo\bin"
}

Write-Host "Creating local virtual environment (.venv)..."
uv venv --python 3.11

Write-Host "Installing PyTorch, DeepFilterNet, and soundfile..."
uv pip install torch==2.5.1+cu121 torchaudio==2.5.1+cu121 --index-url https://download.pytorch.org/whl/cu121
uv pip install deepfilternet soundfile

Write-Host "Environment setup complete!"
