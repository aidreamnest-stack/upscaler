@echo off
title Install Image Upscaler Shortcut
color 0A

echo ===================================================
echo     Image Upscaler - Desktop Shortcut Installer
echo ===================================================
echo.
echo This will create a shortcut named "Image Upscaler" on your Desktop.
echo.

set "SCRIPT_DIR=%~dp0"
set "TARGET_BAT=%SCRIPT_DIR%start.bat"
set "SHORTCUT_PATH=%USERPROFILE%\Desktop\Image Upscaler.lnk"
set "ICON_PATH=%SCRIPT_DIR%icon.ico"

:: Use PowerShell to create the Windows shortcut (.lnk file)
:: The icon is set to the custom icon.ico in the repository
powershell -NoProfile -Command "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('%SHORTCUT_PATH%'); $Shortcut.TargetPath = '%TARGET_BAT%'; $Shortcut.WorkingDirectory = '%SCRIPT_DIR%'; $Shortcut.IconLocation = '%ICON_PATH%'; $Shortcut.Description = 'Launch Image Upscaler AI Tool'; $Shortcut.Save()"

if exist "%SHORTCUT_PATH%" (
    echo [SUCCESS] Shortcut created successfully on your Desktop!
    echo.
    echo You can now close this window and double-click the "Image Upscaler" 
    echo icon on your Desktop anytime to start the tool.
) else (
    echo [ERROR] Failed to create the shortcut.
)

echo.
pause
