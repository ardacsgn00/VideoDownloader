@echo off
chcp 65001 >nul

:basla
cls
echo ============================================
echo               VIDEO INDIRICI
echo ============================================
echo.
echo [1] En Yuksek Kalite Video
echo [2] Sadece Ses (MP3)
echo [3] 1080p Video (Boyut Tasarrufu)
echo [4] Sistemi Guncelle (Çalışmıyorsa)
echo.
set /p secim="Secimini yaz (1, 2, 3 veya 4): "

if "%secim%"=="4" goto guncelle

echo.
set /p url="Indirmek istedigin linki yapistir: "
if "%url%"=="" goto basla

echo.
echo Indirme baslatiliyor...
echo.

set YTPATH="C:\yt-dlp\yt-dlp.exe"
set HEDEF="%USERPROFILE%\Desktop"

if "%secim%"=="1" (
    %YTPATH% -P %HEDEF% -S "vcodec:h264,res,acodec:aac" --merge-output-format mp4 -o "%%(title)s.%%(ext)s" "%url%"
) else if "%secim%"=="2" (
    %YTPATH% -P %HEDEF% -x --audio-format mp3 -o "%%(title)s.%%(ext)s" "%url%"
) else if "%secim%"=="3" (
    %YTPATH% -P %HEDEF% -S "vcodec:h264,res:1080,acodec:aac" --merge-output-format mp4 -o "%%(title)s.%%(ext)s" "%url%"
) else (
    %YTPATH% -P %HEDEF% -S "vcodec:h264,res,acodec:aac" --merge-output-format mp4 -o "%%(title)s.%%(ext)s" "%url%"
)

echo.
echo ============================================
echo Indirme bitti! Dosya Masaustune kaydedildi.
echo ============================================
pause
goto basla

:guncelle
echo.
echo yt-dlp guncelleniyor...
echo.
"C:\yt-dlp\yt-dlp.exe" -U
echo.
pause
goto basla