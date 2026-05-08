param(
  [string]$MsiPath = "",
  [string]$LogDirectory = "target\release\bundle\msi-smoke"
)

$ErrorActionPreference = "Stop"

function Resolve-MsiPath {
  param([string]$Path)

  if (-not [string]::IsNullOrWhiteSpace($Path)) {
    if (-not (Test-Path $Path)) {
      throw "MSI artifact was not found: $Path"
    }
    return (Resolve-Path $Path).Path
  }

  $msi = Get-ChildItem -Path "target\release\bundle\msi" -Filter "*.msi" -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $msi) {
    throw "No MSI artifact was found under target\release\bundle\msi"
  }

  return $msi.FullName
}

function Get-MsiProperty {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  $installer = New-Object -ComObject WindowsInstaller.Installer
  $database = $installer.OpenDatabase($Path, 0)
  $query = "SELECT ``Value`` FROM ``Property`` WHERE ``Property`` = '$Name'"
  $view = $database.OpenView($query)
  $null = $view.Execute()
  $record = $view.Fetch()

  if ($null -eq $record) {
    $null = $view.Close()
    return $null
  }

  $value = $record.StringData(1)
  $null = $view.Close()
  if ($value -is [array]) {
    $value = $value[0]
  }

  return ([string]$value).Trim()
}

function Get-UninstallEntry {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ProductCode,
    [Parameter(Mandatory = $true)]
    [string]$ProductName
  )

  $paths = @(
    "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
  )

  Get-ItemProperty -Path $paths -ErrorAction SilentlyContinue |
    Where-Object { ($_.PSChildName -eq $ProductCode) -or ($_.DisplayName -eq $ProductName) }
}

function Invoke-MsiExecChecked {
  param(
    [Parameter(Mandatory = $true)]
    [string[]]$Arguments,
    [Parameter(Mandatory = $true)]
    [string]$Description
  )

  Write-Host "> msiexec.exe $($Arguments -join ' ')"
  $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = "msiexec.exe"
  $startInfo.UseShellExecute = $false
  foreach ($argument in $Arguments) {
    $startInfo.ArgumentList.Add($argument)
  }

  $process = [System.Diagnostics.Process]::Start($startInfo)
  $process.WaitForExit()
  $exitCode = $process.ExitCode

  if (($exitCode -ne 0) -and ($exitCode -ne 3010)) {
    throw "$Description failed with exit code $exitCode"
  }
  if ($exitCode -eq 3010) {
    Write-Host "$Description completed and requested reboot (3010)."
  }
}

if ($PSVersionTable.PSEdition -eq "Core" -and -not $IsWindows) {
  throw "scripts\smoke-windows-msi.ps1 must run on Windows."
}

$resolvedMsi = Resolve-MsiPath -Path $MsiPath
New-Item -ItemType Directory -Path $LogDirectory -Force | Out-Null
$resolvedLogDirectory = (Resolve-Path $LogDirectory).Path
$installLog = Join-Path $resolvedLogDirectory "msi-install.log"
$uninstallLog = Join-Path $resolvedLogDirectory "msi-uninstall.log"

$productCode = Get-MsiProperty -Path $resolvedMsi -Name "ProductCode"
$productName = Get-MsiProperty -Path $resolvedMsi -Name "ProductName"

if ([string]::IsNullOrWhiteSpace($productCode)) {
  throw "MSI ProductCode is missing in $resolvedMsi"
}
if ([string]::IsNullOrWhiteSpace($productName)) {
  throw "MSI ProductName is missing in $resolvedMsi"
}

Write-Host "MSI smoke target: $resolvedMsi"
Write-Host "MSI ProductCode: $productCode"
Write-Host "MSI ProductName: $productName"

$existing = Get-UninstallEntry -ProductCode $productCode -ProductName $productName
if ($existing) {
  Write-Host "Existing installation found; uninstalling before smoke install."
  Invoke-MsiExecChecked -Description "Pre-smoke uninstall" -Arguments @("/x", $productCode, "/qn", "/norestart", "/L*v", $uninstallLog)
}

try {
  Invoke-MsiExecChecked -Description "MSI silent install" -Arguments @("/i", $resolvedMsi, "/qn", "/norestart", "/L*v", $installLog)

  $installed = Get-UninstallEntry -ProductCode $productCode -ProductName $productName
  if (-not $installed) {
    throw "MSI silent install completed but no uninstall registry entry was found for $productName ($productCode)."
  }

  Write-Host "MSI install registry entry found:"
  $installed | Select-Object -First 1 DisplayName, DisplayVersion, Publisher, InstallLocation | Format-List
} finally {
  if (Get-UninstallEntry -ProductCode $productCode -ProductName $productName) {
    Invoke-MsiExecChecked -Description "MSI silent uninstall" -Arguments @("/x", $productCode, "/qn", "/norestart", "/L*v", $uninstallLog)
  }
}

$remaining = Get-UninstallEntry -ProductCode $productCode -ProductName $productName
if ($remaining) {
  throw "MSI silent uninstall completed but uninstall registry entry remains for $productName ($productCode)."
}

Write-Host "MSI silent install/uninstall smoke passed."
