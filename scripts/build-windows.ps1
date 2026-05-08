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

function Get-FirstFile {
  param(
    [Parameter(Mandatory = $true)]
    [string[]]$Paths,
    [Parameter(Mandatory = $true)]
    [string]$Description
  )

  foreach ($path in $Paths) {
    if (Test-Path $path -PathType Leaf) {
      return (Resolve-Path $path).Path
    }
  }

  throw "$Description was not found. Checked: $($Paths -join ', ')"
}

function Get-PackagedEumdacFile {
  $sidecar = Get-ChildItem -Path "target\release" -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -in @("eumdac.exe", "eumdac-x86_64-pc-windows-msvc.exe") } |
    Select-Object -First 1

  if (-not $sidecar) {
    throw "Packaged EUMDAC sidecar was not found under target\release"
  }

  return $sidecar.FullName
}

function Get-PackagedEumdacManifest {
  $manifest = Get-ChildItem -Path "target\release" -Recurse -File -Filter "eumdac-sidecar-manifest.json" -ErrorAction SilentlyContinue |
    Select-Object -First 1

  if (-not $manifest) {
    throw "Packaged EUMDAC sidecar manifest was not found under target\release"
  }

  return $manifest.FullName
}

function Update-EumdacManifestHash {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath,
    [Parameter(Mandatory = $true)]
    [string]$SidecarPath
  )

  $sidecarName = Split-Path -Leaf $SidecarPath
  $sidecarHash = (Get-FileHash -Algorithm SHA256 -Path $SidecarPath).Hash.ToLowerInvariant()
  $manifest = Get-Content -Raw -Path $ManifestPath | ConvertFrom-Json
  foreach ($entry in $manifest.binaries) {
    if (($entry.name -eq $sidecarName) -or ($entry.name -eq "eumdac.exe")) {
      $entry.sha256 = $sidecarHash
    }
  }
  $manifest | ConvertTo-Json -Depth 8 | Set-Content -Path $ManifestPath -Encoding ascii
  Write-Host "Updated EUMDAC sidecar manifest hash for $sidecarName"
}

function Assert-EumdacManifestHash {
  param(
    [Parameter(Mandatory = $true)]
    [string]$SidecarPath,
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath
  )

  $sidecarName = Split-Path -Leaf $SidecarPath
  $sidecarHash = (Get-FileHash -Algorithm SHA256 -Path $SidecarPath).Hash.ToLowerInvariant()
  $manifest = Get-Content -Raw -Path $ManifestPath | ConvertFrom-Json
  $entry = $manifest.binaries |
    Where-Object { (($_.name -eq $sidecarName) -or ($_.name -eq "eumdac.exe")) -and ($_.sha256 -eq $sidecarHash) } |
    Select-Object -First 1

  if (-not $entry) {
    throw "EUMDAC sidecar hash $sidecarHash for $sidecarName does not match $ManifestPath"
  }

  Write-Host "EUMDAC sidecar manifest hash matches: $sidecarName"
}

function Invoke-WindowsSigningScript($Path) {
  Invoke-CheckedCommand -Command "powershell" -Arguments @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "scripts\sign-windows.ps1", $Path)
}

node --version
npm --version
cargo --version

$staleArtifactPaths = @(
  "target\release\bundle\msi",
  "target\release\bundle\nsis",
  "target\release\bundle\SHA256SUMS.txt",
  "target\windows-signing.tauri.conf.json"
)
foreach ($stalePath in $staleArtifactPaths) {
  if (Test-Path $stalePath) {
    Remove-Item -Path $stalePath -Recurse -Force
  }
}

Invoke-CheckedCommand -Command "npm" -Arguments @("ci")
Invoke-CheckedCommand -Command "npm" -Arguments @("run", "verify")
Invoke-CheckedCommand -Command "npm" -Arguments @("run", "eumdac:prepare")

$stagedEumdacSidecar = Get-FirstFile -Paths @(
  "src-tauri\binaries\eumdac-x86_64-pc-windows-msvc.exe"
) -Description "Staged EUMDAC sidecar"
$stagedEumdacManifest = Get-FirstFile -Paths @(
  "src-tauri\resources\eumdac-sidecar-manifest.json"
) -Description "Generated EUMDAC sidecar manifest"

if (-not [string]::IsNullOrWhiteSpace($env:WINDOWS_SIGN_COMMAND)) {
  Invoke-WindowsSigningScript $stagedEumdacSidecar
  Update-EumdacManifestHash -ManifestPath $stagedEumdacManifest -SidecarPath $stagedEumdacSidecar
}
Assert-EumdacManifestHash -SidecarPath $stagedEumdacSidecar -ManifestPath $stagedEumdacManifest

$tauriBuildArgs = @("run", "tauri:build", "--", "--config", "src-tauri\tauri.eumdac.generated.conf.json", "--bundles", "msi,nsis")
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
$packagedEumdacSidecar = Get-PackagedEumdacFile
$packagedEumdacManifest = Get-PackagedEumdacManifest
Assert-EumdacManifestHash -SidecarPath $packagedEumdacSidecar -ManifestPath $packagedEumdacManifest

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
  Assert-AuthenticodeSignature $packagedEumdacSidecar
  $artifacts | ForEach-Object { Assert-AuthenticodeSignature $_.FullName }
} else {
  Write-Host "Authenticode verification skipped: WINDOWS_SIGN_COMMAND is not configured"
}
