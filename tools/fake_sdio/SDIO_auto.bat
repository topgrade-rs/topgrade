@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0fake_sdio.ps1" %*
set "ERR=%ERRORLEVEL%"
endlocal & exit /b %ERR%
