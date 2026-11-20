@echo off
rem Runs the dist build with stdout/stderr captured to out\audit\ (AUDIT-2026-11-20).
cd /d "%~dp0\..\.."
if not exist out\audit mkdir out\audit
"dist\Constellations\constellations.exe" %* 1> out\audit\stdout.log 2> out\audit\stderr.log
echo Exit code %ERRORLEVEL%. Logs in out\audit\
