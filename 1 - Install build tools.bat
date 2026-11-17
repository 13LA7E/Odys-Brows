@echo off
title Odyssey - install build tools
cd /d "%~dp0"
set LOG=%~dp0install-log.txt
echo Odyssey build tools install > "%LOG%"
echo Started %date% %time% >> "%LOG%"

echo [1/3] Installing Rust (rustup)...
winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements --silent >> "%LOG%" 2>&1
echo rustup winget exit: %errorlevel% >> "%LOG%"

echo [2/3] Setting up the Rust toolchain...
"%USERPROFILE%\.cargo\bin\rustup.exe" default stable-x86_64-pc-windows-msvc >> "%LOG%" 2>&1
echo rustup default exit: %errorlevel% >> "%LOG%"

echo [3/3] Installing Visual Studio C++ Build Tools (about 3-5 GB, takes a while).
echo      Click YES if Windows asks for permission.
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-source-agreements --accept-package-agreements --override "--quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" >> "%LOG%" 2>&1
echo buildtools winget exit: %errorlevel% >> "%LOG%"

"%USERPROFILE%\.cargo\bin\cargo.exe" --version >> "%LOG%" 2>&1
"%USERPROFILE%\.cargo\bin\rustc.exe" --version >> "%LOG%" 2>&1
if exist "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC" (echo MSVC: found >> "%LOG%") else (echo MSVC: NOT found >> "%LOG%")
echo Finished %date% %time% >> "%LOG%"
echo INSTALL_DONE >> "%LOG%"
echo.
echo All done. You can close this window.
pause
