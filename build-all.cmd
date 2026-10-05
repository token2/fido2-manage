@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-all.ps1" %*
