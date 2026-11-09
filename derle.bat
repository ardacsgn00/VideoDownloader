@echo off
chcp 65001 >nul
echo ============================================
echo   VIDEO INDIRICI - DERLEME ARACI (RUST)
echo ============================================
echo.
echo MSVC ve Rust araclari hazirlaniyor...
call "E:\devtools\msvc\setup_x64.bat"
set "RUSTUP_HOME=E:\devtools\rust\rustup"
set "CARGO_HOME=E:\devtools\rust\cargo"
set "PATH=E:\devtools\rust\cargo\bin;%PATH%"

echo.
echo Derleme baslatiliyor (Release modu)...
cd /d "%~dp0gui"
cargo build --release

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [HATA] Derleme basarisiz oldu!
    pause
    exit /b %ERRORLEVEL%
)

echo.
echo Derleme tamamlandi! Dosya kopyalaniyor...
copy /y "%~dp0gui\target\release\video_indirici.exe" "%~dp0VideoIndirici.exe" >nul

echo.
echo ============================================
echo   Basarili! VideoIndirici.exe olusturuldu.
echo ============================================
pause

