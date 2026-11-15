@echo off
cd /d "%~dp0"
cargo build --release
if errorlevel 1 (
    echo Build failed.
    pause
    exit /b 1
)
rem Launch detached so this console closes and the game window keeps the focus.
start "" "target\release\rustblox.exe"
exit
