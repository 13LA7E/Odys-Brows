@echo off
title Odyssey - build
cd /d "%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
taskkill /IM odyssey.exe /F >nul 2>&1
echo Building Odyssey...
echo Build started %date% %time% > build-log.txt
cargo build --release >> build-log.txt 2>&1
set CARGO_EXIT=%errorlevel%
echo cargo exit: %CARGO_EXIT% >> build-log.txt
if "%CARGO_EXIT%"=="0" (
  echo BUILD_OK >> build-log.txt
  start "" "target\release\odyssey.exe"
) else (
  echo BUILD_FAILED >> build-log.txt
  echo Build failed - see build-log.txt
  timeout /t 8
)
