# Signals SDR v0.4.4
Esta versión incluye una **[versión web](https://tcasaniv.github.io/signals-sdr/)** alojada en GitHub Pages, **ejecutables precompilados** para **Windows**, **Linux** y **macOS (Apple Silicon)**, la migración de dependencias a **crates.io** y compatibilidad nativa con **Wayland y X11**.

## Demo en Línea (WebAssembly)
Puedes probar la app directamente desde tu navegador sin instalar dependencias: **[Abrir Signals SDR Web](https://tcasaniv.github.io/signals-sdr/)**

> [!NOTE]
> (Permite generar tramas, modelar canal y usar el Instrumento Virtual con FFT, cascada y constelación. Para control de hardware SDR por USB o emisión por RTL-TCP nativo, utiliza los binarios de escritorio).

## Instalación y Ejecución

### Windows 10 / 11 (x64)
1. Descarga `signals-sdr-v0.4.4-windows-amd64.zip`.
2. Descomprime el archivo y ejecuta:
   ```powershell
   .\signals-sdr.exe
   ```

### Linux / Ubuntu (x64)
1. Descarga `signals-sdr-v0.4.4-linux-amd64.tar.gz`.
2. Descomprime y ejecuta:
   ```bash
   tar -xzvf signals-sdr-v0.4.4-linux-amd64.tar.gz
   ./signals-sdr
   ```

> [!NOTE]
> En instalaciones limpias de Ubuntu/Debian, si el sistema requiere librerías multimedia o de audio, puedes instalarlas con:
> ```bash
> sudo apt update && sudo apt install -y libasound2 libudev1 libx11-6 libwayland-client0
> ```

---

### macOS (Apple Silicon M1/M2/M3/M4)
1. Descarga `signals-sdr-v0.4.4-macos-arm64.tar.gz`.
2. Descomprime y ejecuta:
   ```bash
   tar -xzvf signals-sdr-v0.4.4-macos-arm64.tar.gz
   ./signals-sdr
   ```

> [!TIP]
> Si macOS Gatekeeper advierte que la aplicación proviene de un desarrollador no identificado:
> - Puedes hacer clic derecho sobre el archivo en Finder y seleccionar **Abrir**.
> - O ejecutar en la Terminal: `xattr -cr signals-sdr`

---

### WSL2 / Ubuntu (Windows Subsystem for Linux)

En WSL necesitas instalar los portales de interfaz para soporte gráfico y explorador de archivos:

```bash
sudo apt update && sudo apt install -y zenity xdg-desktop-portal-gtk
```

Luego descomprime y ejecuta:
```bash
tar -xzvf signals-sdr-v0.4.4-linux-amd64.tar.gz
./signals-sdr
```

> [!TIP]
> Si vas a usar un dispositivo SDR físico por USB (ej. HackRF) desde WSL, recuerda que WSL2 no monta dispositivos USB automáticamente. Para compartir el hardware USB de Windows a WSL, utiliza la utilidad oficial [usbipd-win](https://github.com/dorssel/usbipd-win):
> ```cmd
> usbipd list
> usbipd attach --wsl --busid <BUSID>
> ```

---

## Novedades y Mejoras en v0.4.4

- **Binarios Nativos Oficiales**: Compilación y empaquetado automático multiplataforma (Windows `.zip`, Linux `.tar.gz`, macOS `.tar.gz`) adjuntos directamente a la release.
- **Instrumento Virtual en WebAssembly**: Migración a `web-time` para la medición temporal. Elimina el error de pánico en navegadores y permite usar el analizador FFT, la cascada espectral y el diagrama de constelación en la web a 60 FPS.
- **Corrección de Rutas en GitHub Pages**: Enrutamiento canónico de módulos `.js` y binarios `.wasm` bajo `/signals-sdr/` sin errores 404.
- **Migración a Crates.io**: Todas las dependencias de codificación (`apt-encoder`, `lrpt-encoder`, `sstv-encoder`, `fm-modulator`, `am-modulator`) y abstracción SDR (`seify`) ahora apuntan a su versión pública.
- **Soporte de Pantalla Wayland y X11**: Compatibilidad gráfica directa para entornos de escritorio Linux modernos con Wayland y servidores tradicionales X11.
- **Optimización de Red RTL-TCP**: Desactivación del algoritmo de Nagle (`TCP_NODELAY`) y sondeo sincronizado para una recepción fluida en SDR#, SDR++ y GQRX.