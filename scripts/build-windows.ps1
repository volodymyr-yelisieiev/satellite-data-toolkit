$ErrorActionPreference = "Stop"

function Require-Command($Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "$Name is required"
  }
}

function Invoke-CheckedCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Command,
    [Parameter(Mandatory = $true)]
    [string[]]$Arguments
  )

  & $Command @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$Command failed with exit code $LASTEXITCODE"
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

function Assert-AuthenticodeSignature($Path) {
  $signature = Get-AuthenticodeSignature -FilePath $Path
  if ($signature.Status -ne "Valid") {
    throw "Authenticode signature is not valid for $Path`: $($signature.Status)"
  }

  Write-Host "Authenticode signature valid: $Path"
}

node --version
npm --version
cargo --version

Invoke-CheckedCommand -Command "npm" -Arguments @("ci")
Invoke-CheckedCommand -Command "npm" -Arguments @("run", "verify")

$tauriBuildArgs = @("run", "tauri:build", "--", "--bundles", "msi,nsis")
if (-not [string]::IsNullOrWhiteSpace($env:WINDOWS_SIGN_COMMAND)) {
  $signScript = (Resolve-Path "scripts\sign-windows.ps1").Path.Replace("\", "\\")
  $signConfigPath = "target\windows-signing.tauri.conf.json"
  $signCommand = "powershell -NoProfile -ExecutionPolicy Bypass -File `"$signScript`" `"%1`""
  $signConfig = @{
    bundle = @{
      windows = @{
        signCommand = $signCommand
      }
    }
  } | ConvertTo-Json -Depth 5

  New-Item -ItemType Directory -Path "target" -Force | Out-Null
  $signConfig | Set-Content -Path $signConfigPath -Encoding ascii
  $tauriBuildArgs += @("--config", $signConfigPath)
  Write-Host "Windows Authenticode signing is enabled through scripts\sign-windows.ps1"
} else {
  Write-Host "Windows Authenticode signing is disabled: WINDOWS_SIGN_COMMAND is not configured"
}

Invoke-CheckedCommand -Command "npm" -Arguments $tauriBuildArgs
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

if (-not [string]::IsNullOrWhiteSpace($env:WINDOWS_SIGN_COMMAND)) {
  Assert-AuthenticodeSignature "target\release\satellite-data-toolkit.exe"
  $artifacts | ForEach-Object { Assert-AuthenticodeSignature $_.FullName }
} else {
  Write-Host "Authenticode verification skipped: WINDOWS_SIGN_COMMAND is not configured"
}
