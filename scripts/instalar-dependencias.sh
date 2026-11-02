#!/bin/bash
set -e

echo "====================================================================="
echo "       Signals SDR - Asistente de Dependencias del Sistema"
echo "====================================================================="
echo ""

OS_NAME="$(uname -s)"

if [ "$OS_NAME" = "Linux" ]; then
    echo "Detectado sistema Linux."
    echo "Instalando paquetes de audio (ALSA), gráficos (X11/Wayland) y SoapySDR..."
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y libasound2 libudev1 libx11-6 libwayland-client0 soapysdr-tools libsoapysdr-dev
        echo "¡Dependencias instaladas con éxito!"
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y alsa-lib systemd-libs libX11 wayland-client SoapySDR
        echo "¡Dependencias instaladas con éxito!"
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -Sy --noconfirm alsa-lib systemd-libs libx11 wayland soapysdr
        echo "¡Dependencias instaladas con éxito!"
    else
        echo "Gestor de paquetes no reconocido automáticamente."
        echo "Por favor instala manualmente: libasound2, libudev, libx11, wayland y soapysdr."
    fi
elif [ "$OS_NAME" = "Darwin" ]; then
    echo "Detectado macOS."
    echo "Eliminando atributos de cuarentena Gatekeeper en el binario local..."
    if [ -f "./signals-sdr" ]; then
        xattr -cr ./signals-sdr
        echo "Cuarentena removida para ./signals-sdr."
    fi
    if command -v brew >/dev/null 2>&1; then
        echo "Instalando SoapySDR con Homebrew..."
        brew install soapysdr
    else
        echo "Homebrew no detectado. Si deseas usar transceptores SoapySDR, instala Homebrew desde https://brew.sh"
    fi
else
    echo "Sistema no reconocido: $OS_NAME"
fi

echo ""
echo "¡Listo! Ya puedes ejecutar ./signals-sdr"
