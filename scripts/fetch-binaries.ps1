$ErrorActionPreference = "Stop"

$ReleaseTag = if ($env:DEEPFILTER_RELEASE_TAG) { $env:DEEPFILTER_RELEASE_TAG } else { "v0.5.6" }
$BaseUrl = "https://github.com/Rikorose/DeepFilterNet/releases/download/$ReleaseTag"
$TargetDir = Join-Path $PSScriptRoot "..\src-tauri\binaries"
New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null

function Download-Binary {
  param(
    [string]$AssetName,
    [string]$OutputName
  )
  Invoke-WebRequest -Uri "$BaseUrl/$AssetName" -OutFile (Join-Path $TargetDir $OutputName)
}

Download-Binary -AssetName "deep-filter-0.5.6-x86_64-unknown-linux-musl" -OutputName "deep-filter-x86_64-unknown-linux-gnu"
Download-Binary -AssetName "deep-filter-0.5.6-x86_64-pc-windows-msvc.exe" -OutputName "deep-filter-x86_64-pc-windows-msvc.exe"

Write-Host "DeepFilterNet binaries installed in $TargetDir"
