@echo off
chcp 65001 >nul
title Asistente de Controladores SDR - Signals SDR

echo =====================================================================
echo           Signals SDR - Asistente de Controladores y Módulos
echo =====================================================================
echo.
echo NOTA: Signals SDR incluye soporte NATIVO directo para HackRF One,
echo simulador Dummy y servidor RTL-TCP sin necesidad de instalar nada.
echo.
echo Este asistente te permite instalar o configurar SoapySDR si planeas
echo transmitir con transceptores como PlutoSDR, LimeSDR, bladeRF o USRP.
echo.

:menu
echo Selecciona una opción:
echo [1] Descargar e instalar Radioconda (Recomendado para GNU Radio y SoapySDR)
echo [2] Descargar e instalar PothosSDR (Paquete liviano con SoapySDR y drivers)
echo [3] Comprobar si SoapySDR ya está instalado en este equipo
echo [4] Salir
echo.

set /p opcion="Ingresa tu opción (1-4): "

if "%opcion%"=="1" goto radioconda
if "%opcion%"=="2" goto pothos
if "%opcion%"=="3" goto check
if "%opcion%"=="4" goto fin

echo Opción inválida. Intenta nuevamente.
goto menu

:radioconda
echo.
echo Abriendo la página oficial de Radioconda en tu navegador...
start https://github.com/ryanvolz/radioconda/releases/latest
echo Una vez instalado Radioconda, Signals SDR detectará SoapySDR automáticamente.
goto fin

:pothos
echo.
echo Descargando instalador de PothosSDR...
powershell -Command "Start-BitsTransfer -Source 'https://downloads.myriadrf.org/builds/PothosSDR/PothosSDR-2021.07.25-vc16-x64.exe' -Destination 'PothosSDR-Installer.exe'"
if exist PothosSDR-Installer.exe (
    echo Iniciando instalador de PothosSDR...
    start PothosSDR-Installer.exe
) else (
    echo No se pudo descargar automáticamente. Abriendo sitio web...
    start https://downloads.myriadrf.org/builds/PothosSDR/
)
goto fin

:check
echo.
echo Buscando SoapySDR en rutas conocidas...
powershell -NoProfile -Command "& { $paths = @($env:USERPROFILE + '\radioconda\Library\bin', 'C:\Program Files\PothosSDR\bin', 'C:\radioconda\Library\bin'); $found = $false; foreach ($p in $paths) { if (Test-Path ($p + '\SoapySDR.dll')) { Write-Host 'Encontrado SoapySDR.dll en:' $p -ForegroundColor Green; $found = $true } }; if (-not $found) { Write-Host 'No se encontró SoapySDR en las rutas estándar.' -ForegroundColor Yellow } }"
echo.
pause
goto menu

:fin
echo.
echo ¡Listo! Puedes ejecutar signals-sdr.exe en cualquier momento.
pause
