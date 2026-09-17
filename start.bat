@echo off
title Image Upscaler
cd /d "%~dp0"

echo ===================================================
echo               Image Upscaler AI Studio
echo ===================================================
echo.
echo [1/2] Checking for latest updates...
git rev-parse --is-inside-work-tree >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    git pull --quiet 2>nul
    echo       Latest updates applied successfully!
) else (
    echo       Offline mode / Standalone startup.
)

echo.
echo [2/2] Starting AI Upscaling Server...
start "" "http://127.0.0.1:8080"
python server.py

pause
