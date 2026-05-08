param(
  [Parameter(Mandatory = $true)]
  [string]$Path
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Path)) {
  throw "Windows signing target was not found: $Path"
}

$resolvedPath = (Resolve-Path $Path).Path

if ([string]::IsNullOrWhiteSpace($env:WINDOWS_SIGN_COMMAND)) {
  Write-Host "Windows signing skipped: WINDOWS_SIGN_COMMAND is not configured for $resolvedPath"
  exit 0
}

$env:WINDOWS_SIGN_FILE = $resolvedPath
$command = $env:WINDOWS_SIGN_COMMAND

if ($command.Contains("{file}")) {
  $escapedPath = $resolvedPath.Replace("'", "''")
  $command = $command.Replace("{file}", "'$escapedPath'")
} elseif ($command.Contains("%1")) {
  $escapedPath = $resolvedPath.Replace("'", "''")
  $command = $command.Replace("%1", "'$escapedPath'")
}

Write-Host "Signing Windows artifact: $resolvedPath"
$process = Start-Process -FilePath "powershell" -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $command) -NoNewWindow -Wait -PassThru
if ($process.ExitCode -ne 0) {
  throw "Windows signing command failed with exit code $($process.ExitCode)"
}
