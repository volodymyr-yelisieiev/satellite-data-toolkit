@echo off
setlocal EnableExtensions EnableDelayedExpansion
cd /d "%~dp0"
title Satellite Data Toolkit Pro - Rust Builder

echo ======================================================
echo Satellite Data Toolkit Pro - Windows Loader
echo Rust app with professional API slots, cache, logs
echo ======================================================
echo.

where cargo >nul 2>&1
if errorlevel 1 (
    echo [1/4] Rust not found. Downloading rustup-init.exe...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile 'rustup-init.exe'"
    if errorlevel 1 (
        echo ERROR: Could not download Rust installer.
        pause
        exit /b 1
    )
    echo [2/4] Installing Rust toolchain locally for current user...
    rustup-init.exe -y --default-toolchain stable
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
) else (
    echo [1/4] Rust already installed.
)

echo [3/4] Building release executable...
cargo build --release
if errorlevel 1 (
    echo.
    echo ERROR: Build failed. On Windows you may need Microsoft C++ Build Tools.
    echo Install: https://visualstudio.microsoft.com/visual-cpp-build-tools/
    pause
    exit /b 1
)

echo [4/4] Starting application...
"target\release\satellite_data_toolkit.exe"
pause
