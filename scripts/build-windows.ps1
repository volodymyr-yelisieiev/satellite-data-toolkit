$ErrorActionPreference = "Stop"

function Require-Command($Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "$Name is required"
  }
}

Require-Command node
Require-Command npm
Require-Command cargo

function Assert-WindowsGuiSubsystem($Path) {
  if (-not (Test-Path $Path)) {
    throw "Windows executable was not produced: $Path"
  }

  $bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $Path))
  $peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
  $optionalHeaderOffset = $peOffset + 24
  $magic = [BitConverter]::ToUInt16($bytes, $optionalHeaderOffset)
  if (($magic -ne 0x10b) -and ($magic -ne 0x20b)) {
    throw "Unknown PE optional header magic 0x$($magic.ToString('x')) in $Path"
  }

  $subsystemOffset = $optionalHeaderOffset + 68
  $subsystem = [BitConverter]::ToUInt16($bytes, $subsystemOffset)
  if ($subsystem -ne 2) {
    throw "Expected Windows GUI subsystem (2), got $subsystem for $Path"
  }

  Write-Host "Windows subsystem: GUI (2)"
}

node --version
npm --version
cargo --version

npm ci
npm run verify
npm run tauri:build -- --bundles msi,nsis
Assert-WindowsGuiSubsystem "target\release\satellite-data-toolkit.exe"

$msi = Get-ChildItem -Path "target\release\bundle\msi" -Filter "*.msi" -ErrorAction SilentlyContinue
$nsis = Get-ChildItem -Path "target\release\bundle\nsis" -Filter "*.exe" -ErrorAction SilentlyContinue
if (-not $msi) { throw "MSI artifact was not produced" }
if (-not $nsis) { throw "NSIS artifact was not produced" }

Write-Host "MSI artifacts:"
$msi | ForEach-Object { Write-Host " - $($_.FullName)" }
Write-Host "NSIS artifacts:"
$nsis | ForEach-Object { Write-Host " - $($_.FullName)" }

$artifacts = @($msi) + @($nsis)
$checksums = $artifacts | ForEach-Object {
  $hash = Get-FileHash -Algorithm SHA256 -Path $_.FullName
  "$($hash.Hash.ToLowerInvariant())  $($_.Name)"
}
$checksumPath = "target\release\bundle\SHA256SUMS.txt"
$checksums | Set-Content -Path $checksumPath -Encoding ascii
Write-Host "SHA256 sums: $((Resolve-Path $checksumPath).Path)"
