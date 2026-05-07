@echo off
cd /d "%~dp0"
if not exist "target\release\satellite_data_toolkit.exe" (
    cargo build --release
)
mkdir dist 2>nul
copy /Y "target\release\satellite_data_toolkit.exe" "dist\SatelliteDataToolkitPro.exe"
echo Done: dist\SatelliteDataToolkitPro.exe
pause
