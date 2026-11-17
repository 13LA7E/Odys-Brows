@echo off
title Odyssey - push to GitHub
cd /d "%~dp0"
echo Pushing Odyssey (branch main) to https://github.com/13LA7E/Odys-Brows.git ...
echo A browser window may open asking you to sign in to GitHub - that's normal.
echo.
git push -u origin main
echo.
echo Done. If you saw an error above, read it and let Claude know.
pause
