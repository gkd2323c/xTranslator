@echo off
setlocal

echo ============================================
echo   xTranslator - Release Build
echo ============================================
echo.

cd /d "%~dp0"

echo [1/2] Running tests...
cargo test -p xt-core --lib --quiet
if %errorlevel% neq 0 (
    echo TESTS FAILED - aborting build
    exit /b %errorlevel%
)
echo Tests passed.

echo.
echo [2/2] Building release...
cargo tauri build
if %errorlevel% neq 0 (
    echo BUILD FAILED
    exit /b %errorlevel%
)

echo.
echo ============================================
echo   Build complete
echo   target\release\xtranslator-tauri.exe
echo ============================================
