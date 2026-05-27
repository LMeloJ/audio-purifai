# Audio PurifAI Desktop App

Cross-platform desktop app for bulk speech enhancement using DeepFilterNet2.

- Standalone installers for Linux and Windows.
- Drag-and-drop WAV input files.
- Per-file progress and status.
- GPU-accelerated local processing via a bundled, isolated Python `deepfilternet` environment.

## Tech Stack

- Tauri 2 (Rust backend + webview desktop shell)
- React + TypeScript + Vite + Tailwind CSS
- DeepFilterNet Python CLI (`deepFilter`) inside a local `uv` virtual environment

## Automated Environment Setup

This application relies on PyTorch and CUDA to achieve GPU acceleration. To prevent polluting your system Python installation, the app manages its own isolated virtual environment (`.venv`) using `uv`.

**When you launch the app for the first time, you will be prompted to "Initialize Environment".** 
Clicking this button will automatically:
1. Download and install `uv` (if not present).
2. Create an isolated Python 3.11 environment.
3. Install PyTorch with CUDA 12.1 support and DeepFilterNet.

*(Alternatively, you can manually run `./scripts/setup-env.sh` or `./scripts/setup-env.ps1` from the project root).*

## Development Setup

### Prerequisites
- Node.js 20+
- pnpm 9+
- Rust stable toolchain

### Running Locally
```bash
pnpm install
pnpm tauri dev
```

## Build Installers

Linux (from Linux host):

```bash
pnpm install
pnpm tauri build
```

Expected artifacts:
- `src-tauri/target/release/bundle/appimage/*.AppImage`
- `src-tauri/target/release/bundle/deb/*.deb`

Windows (from Windows host):

```powershell
pnpm install
pnpm tauri build
```

Expected artifact:
- `src-tauri/target/release/bundle/nsis/*-setup.exe`

## Usage

1. Launch the app and Initialize the Environment if prompted.
2. Drag and drop `.wav` files or click **Add WAV files**.
3. Choose output directory.
4. Optionally enable post-filter (`--pf`) and tune concurrency.
5. Click **Start** to process in bulk.

## Credits

This project uses [DeepFilterNet / DeepFilterNet2](https://github.com/Rikorose/DeepFilterNet).
