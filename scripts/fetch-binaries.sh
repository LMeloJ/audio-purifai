#!/usr/bin/env bash
set -euo pipefail

RELEASE_TAG="${DEEPFILTER_RELEASE_TAG:-v0.5.6}"
BASE_URL="https://github.com/Rikorose/DeepFilterNet/releases/download/${RELEASE_TAG}"
TARGET_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../src-tauri/binaries" && pwd)"

mkdir -p "${TARGET_DIR}"

download_binary() {
  local asset_name="$1"
  local output_name="$2"
  curl -fL "${BASE_URL}/${asset_name}" -o "${TARGET_DIR}/${output_name}"
  chmod +x "${TARGET_DIR}/${output_name}"
}

download_binary "deep-filter-0.5.6-x86_64-unknown-linux-musl" "deep-filter-x86_64-unknown-linux-gnu"
download_binary "deep-filter-0.5.6-x86_64-pc-windows-msvc.exe" "deep-filter-x86_64-pc-windows-msvc.exe"

echo "DeepFilterNet binaries installed in ${TARGET_DIR}"
