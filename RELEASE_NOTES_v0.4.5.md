# Signals SDR v0.4.5

Esta versión incorpora el **Analizador Vectorial de Señales (VSA)** con normalización de constelación precisa, **exportación de capturas PNG de alta resolución con contexto de laboratorio**, modo directo **Sin Procesamiento** en Generic I/Q, **detección dinámica de múltiples entornos SoapySDR** (Radioconda y PothosSDR) y un **rediseño responsivo Mobile-First** con márgenes simétricos y tipografía fluida bajo directrices Material 3 y Apple HIG.

## Demo en Línea (WebAssembly)
Puedes probar la app directamente desde tu navegador sin instalar dependencias: **[Abrir Signals SDR Web](https://tcasaniv.github.io/signals-sdr/)**

> [!NOTE]
> La versión WebAssembly permite generar tramas, modelar efectos de canal y utilizar el Instrumento Virtual con analizador FFT, cascada espectral, osciloscopio en el tiempo y diagrama de constelación a 60 FPS. Para la transmisión por radiofrecuencia hacia dispositivos SDR físicos conectados por USB (HackRF, bladeRF, USRP, RTL-SDR) o streaming por red RTL-TCP, utiliza los binarios de escritorio.

---

## Instalación y Ejecución

### Windows 10 / 11 (x64)
1. Descarga `signals-sdr-v0.4.5-windows-amd64.zip`.
2. Descomprime el archivo y ejecuta en PowerShell o CMD:
   ```powershell
   .\signals-sdr.exe
   ```

### Linux / Ubuntu (x64)
1. Descarga `signals-sdr-v0.4.5-linux-amd64.tar.gz`.
2. Descomprime y ejecuta:
   ```bash
   tar -xzvf signals-sdr-v0.4.5-linux-amd64.tar.gz
   ./signals-sdr
   ```

> [!NOTE]
> En instalaciones limpias de Ubuntu/Debian, si el sistema requiere librerías multimedia o de audio, puedes instalarlas con:
> ```bash
> sudo apt update && sudo apt install -y libasound2 libudev1 libx11-6 libwayland-client0
> ```

---

### macOS (Apple Silicon M1/M2/M3/M4)
1. Descarga `signals-sdr-v0.4.5-macos-arm64.tar.gz`.
2. Descomprime y ejecuta:
   ```bash
   tar -xzvf signals-sdr-v0.4.5-macos-arm64.tar.gz
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
tar -xzvf signals-sdr-v0.4.5-linux-amd64.tar.gz
./signals-sdr
```

> [!TIP]
> Si vas a usar un dispositivo SDR físico por USB (ej. HackRF) desde WSL, recuerda que WSL2 no monta dispositivos USB automáticamente. Para compartir el hardware USB de Windows a WSL, utiliza la utilidad oficial [usbipd-win](https://github.com/dorssel/usbipd-win):
> ```cmd
> usbipd list
> usbipd attach --wsl --busid <BUSID>
> ```

---

## Novedades y Mejoras en v0.4.5

### 1. Instrumento Virtual de Laboratorio: VSA y Constelación Normalizada
- **Normalización Automática en Modo Símbolos (VSA)**: Los puntos de decisión ahora se escalan automáticamente por su magnitud media de símbolo ($|s|=1.000$), situándose con exactitud sobre el círculo unitario de referencia ($r=1.0$, $I,Q=\pm 0.707$) en modo RF o sobre las marcas enteras cartesianas ($\pm 1.0$) en modo Grilla.
- **Compensación de Retardo OQPSK ($Q = T_s/2$)**: Eliminación precisa del desplazamiento de medio símbolo en señales Meteor LRPT, colapsando trayectorias sobremuestreadas en los cuatro cuadrantes nítidos del estándar.
- **Sincronismo de Fase con Búsqueda Automática (`Auto Ojo`)**: Algoritmo integrado para encontrar el instante de máxima apertura del diagrama de ojo con un solo clic.
- **Modos de Escala de Potencia Clarificados**: Selector de escala abreviado en la interfaz (`RF` / `Grilla`) acompañado de tooltips didácticos detallados que explican la diferencia entre la norma física unitaria ($E_s=1.0$) y la representación académica convencional.
- **Inspección Interactiva con Mira Telescópica**: Al posar el cursor sobre el gráfico, se despliegan retículas de alineación y una tarjeta flotante con lectura en tiempo real de $I$, $Q$, $|s|$ y fase angular $\theta$.
- **Alternancia Independiente de Círculos y Cuadrícula**: Casillas de verificación para activar u ocultar los círculos de referencia ($r=0.5, 1.0$) y la retícula ortogonal por separado.

### 2. Exportación de Capturas PNG con Contexto de Laboratorio
- **Encabezado Institucional y Metadatos**: Las imágenes guardadas desde cualquier pestaña del instrumento virtual (Constelación, Espectro FFT, Cascada, Osciloscopio temporal o Métricas) incluyen un banner superior calibrado con:
  - Título oficial del instrumento.
  - Frecuencia central ($f_c$) y frecuencia de muestreo ($f_s$).
  - Configuración activa (norma de potencia, SPS, bins FFT, tamaño de ventana temporal).
  - Estampa de tiempo UTC.
- **Ejes Calibrados y Graduación Numérica**: Marcas de escala visibles en dBFS, Hz/kHz/MHz, amplitud normalizada y coordenadas cartesianas I/Q.
- **Tarjetas Integradas de Diagnóstico**: Inclusión de recuadros con cálculo de potencia de pico, potencia RMS, relación de cresta (PAPR), desbalance I/Q, estimación de SNR y relación de error de modulación (MER).

### 3. Generic I/Q: Modos Sin Procesamiento y Con Procesamiento DSP
- **Selector Segmentado de Modo**: Opciones simétricas `[ Sin procesamiento ]` y `[ Con procesamiento ]` análogas a las de Efectos de Canal.
- **Transmisión Directa (Bypass Educativo)**: Permite inspeccionar y transmitir archivos I/Q crudos directamente al SDR o analizador sin pasar por bloques de ecualización o filtrado, ofreciendo botones directos para saltar a Efectos de Canal o a Transmisión.
- **Acondicionamiento Opcional**: Acceso modular a filtros pasabanda biquad Butterworth, corrección de desbalance y escalado dinámico cuando el usuario decide activar el procesamiento.

### 4. Gestor Dinámico de Múltiples Instalaciones SoapySDR
- **Detección Automática con un Clic**: Escaneo no bloqueante de bibliotecas SoapySDR en rutas conocidas de **Radioconda** y **PothosSDR**.
- **Selección Flexible**: Muestra una lista de todas las versiones y rutas detectadas en el sistema, permitiendo al usuario conmutar entre ellas inmediatamente mediante el botón `Usar esta versión`.
- **Persistencia Segura**: Guardado automático de la selección en la configuración local de la aplicación.

### 5. Arquitectura Visual Responsiva Material 3 y Apple HIG
- **Simetría de Márgenes en Ajustes**: Corrección del cálculo de anchura en las tarjetas de configuración para mantener márgenes uniformes de 16 px a izquierda y derecha en cualquier resolución de pantalla.
- **Redimensionamiento Fluido del Panel Lateral**: Reestructuración del empaquetado de texto en el catálogo, eliminando el bloqueo que forzaba al panel a expandirse al ancho máximo al soltar el ratón.
- **Diseño Mobile-First en Barras de Reproducción**: Reorganización responsiva en dos filas con anchos dinámicos cuando la ventana se reduce, impidiendo la compresión vertical de controles desplegables o desbordamientos fuera de la pantalla.
- **Iconografía Unificada Vectorial**: Uso exclusivo de símbolos del crate `egui_icons` (Google Material Symbols), sin emojis.
