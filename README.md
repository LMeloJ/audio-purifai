# Audio PurifAI Desktop App

Cross-platform desktop app for bulk speech enhancement using DeepFilterNet2.

- Standalone installers for Linux and Windows.
- Drag-and-drop WAV input files.
- Per-file progress and status.
- Local processing via bundled `deep-filter` sidecar binary (no Python setup).

## Tech Stack

- Tauri 2 (Rust backend + webview desktop shell)
- React + TypeScript + Vite + Tailwind CSS
- DeepFilterNet `deep-filter` CLI sidecar

## Prerequisites (for developers only)

- Node.js 20+
- pnpm 9+
- Rust stable toolchain
- Linux: webkit/gtk build dependencies for Tauri

End users do not need these prerequisites after installation.

## Development Setup

```bash
pnpm install
bash ./scripts/fetch-binaries.sh
pnpm tauri dev
```

Windows PowerShell:

```powershell
pnpm install
pwsh ./scripts/fetch-binaries.ps1
pnpm tauri dev
```

## Build Installers

Linux (from Linux host):

```bash
pnpm install
bash ./scripts/fetch-binaries.sh
pnpm tauri build
```

Expected artifacts:

- `src-tauri/target/release/bundle/appimage/*.AppImage`
- `src-tauri/target/release/bundle/deb/*.deb`

Windows (from Windows host):

```powershell
pnpm install
pwsh ./scripts/fetch-binaries.ps1
pnpm tauri build
```

Expected artifact:

- `src-tauri/target/release/bundle/nsis/*-setup.exe`

Cross-platform CI builds on version tags via `.github/workflows/release-build.yml`.

## Usage

1. Launch the app.
2. Drag and drop `.wav` files or click **Add WAV files**.
3. Choose output directory.
4. Optionally enable post-filter (`--pf`) and tune concurrency.
5. Click **Start** to process in bulk.

## Notes

- Input is intentionally restricted to WAV files.
- Best results are with 48kHz WAV, matching DeepFilterNet2 expectations.
- The app marks non-48kHz files before running.

## Credits

This project uses [DeepFilterNet / DeepFilterNet2](https://github.com/Rikorose/DeepFilterNet) and its `deep-filter` CLI.

Related repository reference: [yuguochencuc/DeepFilterNet2](https://github.com/yuguochencuc/DeepFilterNet2/tree/main).
