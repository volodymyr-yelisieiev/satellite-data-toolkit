$ErrorActionPreference = "Stop"

function Require-Command($Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "$Name is required"
  }
}

Require-Command node
Require-Command npm
Require-Command cargo

node --version
npm --version
cargo --version

npm ci
npm run build
npm run tauri:build -- --bundles msi,nsis

$msi = Get-ChildItem -Path "target\release\bundle\msi" -Filter "*.msi" -ErrorAction SilentlyContinue
$nsis = Get-ChildItem -Path "target\release\bundle\nsis" -Filter "*.exe" -ErrorAction SilentlyContinue
if (-not $msi) { throw "MSI artifact was not produced" }
if (-not $nsis) { throw "NSIS artifact was not produced" }

Write-Host "MSI artifacts:"
$msi | ForEach-Object { Write-Host " - $($_.FullName)" }
Write-Host "NSIS artifacts:"
$nsis | ForEach-Object { Write-Host " - $($_.FullName)" }
