# Signals SDR v0.4.5

Esta versión incorpora capacidades avanzadas de demodulación y análisis en el **Instrumento Virtual de Laboratorio** (Modo Símbolos, alineación OQPSK, sincronismo Auto-Ojo y normalización precisa), **exportación de capturas PNG de alta resolución con contexto de laboratorio**, suite de **Acondicionamiento y Limpieza I/Q en Paso 2** (Generic IQ), **portabilidad out-of-the-box con delay-loading y detección de múltiples entornos SoapySDR** (Radioconda y PothosSDR) y un **rediseño responsivo integral Mobile-First** con márgenes simétricos y tipografía fluida bajo directrices Material 3 y Apple HIG.

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
> [!TIP]
> **Portabilidad Inmediata:** Gracias al soporte de *Delay-Loading*, `signals-sdr.exe` inicia al instante en cualquier equipo con Windows sin requerir instalaciones previas ni arrojar ventanas de error por librerías DLL faltantes. Si deseas transmitir con hardware físico por USB, puedes instalar los controladores con el script asistente incluido `instalar-controladores-sdr.bat` o mediante Radioconda / PothosSDR.

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

### 1. Instrumento Virtual de Laboratorio: Constelación Normalizada y Modo Símbolos
- **Modo Símbolos con Decimación Exacta**: Decimación de la señal continua a 1 muestra por símbolo con selector flexible de SPS (1, 2, 4, 8, 16) para evaluar la constelación digital y los puntos de decisión limpios (BPSK, QPSK, OQPSK, QAM).
- **Normalización Automática en Constelación**: Los puntos de decisión se escalan automáticamente por su magnitud media de símbolo ($|s|=1.000$), situándose con exactitud sobre el círculo unitario de referencia ($r=1.0$, $I,Q=\pm 0.707$) en modo RF o sobre las marcas enteras cartesianas ($\pm 1.0$) en modo Grilla.
- **Compensación de Retardo OQPSK ($Q = T_s/2$)**: Eliminación precisa del desplazamiento de medio símbolo en señales Meteor LRPT, colapsando trayectorias sobremuestreadas en los cuatro cuadrantes nítidos del estándar.
- **Sincronismo de Fase con Búsqueda Automática (`Auto Ojo`)**: Algoritmo integrado para encontrar el instante de máxima apertura del diagrama de ojo con un solo clic.
- **Desrotación Continua de Portadora**: Control angular de $0^\circ \dots 360^\circ$ para alinear la orientación de la constelación con los ejes I/Q.
- **Modos de Escala de Potencia Clarificados**: Selector de escala abreviado en la interfaz (`RF` / `Grilla`) acompañado de tooltips didácticos detallados que explican la diferencia entre la norma física unitaria ($E_s=1.0$) y la representación académica convencional.
- **Inspección Interactiva con Cursor en Cruz**: Al posar el cursor sobre el gráfico, se despliegan retículas de alineación y una lectura en tiempo real de $I$, $Q$, magnitud de símbolo $|s| = \sqrt{I^2 + Q^2}$, ángulo de fase $\theta = \operatorname{atan2}(Q, I)$ y decibelios relativos ($20 \log_{10}|s|$ dBFS).
- **Alternancia Independiente de Círculos y Cuadrícula**: Casillas de verificación para activar u ocultar los círculos de referencia ($r=0.5, 1.0$) y la retícula ortogonal por separado.

### 2. Exportación de Capturas PNG con Contexto de Laboratorio
- **Encabezado Institucional y Metadatos**: Las imágenes guardadas desde cualquier pestaña del instrumento virtual (Constelación, Espectro FFT, Cascada, Osciloscopio temporal o Métricas) incluyen un banner superior calibrado con:
  - Título oficial del instrumento.
  - Frecuencia central ($f_c$) y frecuencia de muestreo ($f_s$).
  - Configuración activa (norma de potencia, SPS, bins FFT, tamaño de ventana temporal).
  - Estampa de tiempo UTC.
- **Ejes Calibrados y Graduación Numérica**: Marcas de escala visibles en dBFS, Hz/kHz/MHz, amplitud normalizada y coordenadas cartesianas I/Q.
- **Tarjetas Integradas de Diagnóstico**: Inclusión de recuadros con cálculo de potencia de pico, potencia RMS, relación de cresta (PAPR), desbalance I/Q, estimación de SNR y relación de error de modulación (MER).

### 3. Generic I/Q: Acondicionamiento DSP y Bypass Didáctico (Paso 2)
- **Selector Segmentado de Modo**: Opciones simétricas `[ Sin procesamiento ]` y `[ Con procesamiento ]` análogas a las de Efectos de Canal.
- **Transmisión Directa (Bypass Educativo)**: Permite inspeccionar y transmitir archivos I/Q crudos directamente al SDR o analizador sin pasar por bloques de ecualización o filtrado, ofreciendo botones directos para saltar a Efectos de Canal o a Transmisión.
- **Cadena de Acondicionamiento Digital**:
  - Filtro FIR pasa-bajos simétrico con ventana de Hann para eliminación de ruido fuera de banda.
  - Corrección automática de desbalance de ganancia y ortogonalidad I/Q mediante ortogonalización Gram-Schmidt.
  - Inversión espectral (conjugada compleja) para corregir grabaciones con banda lateral invertida.
  - Control Automático de Ganancia (AGC) y supresor de ruido (squelch en dBFS).

### 4. Portabilidad Out-of-the-Box y Gestor Dinámico SoapySDR
- **Delay-Loading en Windows (`/DELAYLOAD:SoapySDR.dll`)**: Eliminación del enlace estático temprano. La aplicación abre instantáneamente en cualquier versión de Windows sin mostrar ventanas de error de librerías DLL ausentes.
- **Detección Automática con un Clic**: Escaneo no bloqueante de bibliotecas SoapySDR en rutas conocidas de **Radioconda**, **PothosSDR** y entornos Conda.
- **Selección Flexible**: Muestra una lista de todas las versiones y rutas detectadas en el sistema, permitiendo al usuario conmutar entre ellas inmediatamente mediante el botón `Usar esta versión`.
- **Persistencia Segura**: Guardado automático de la selección en la configuración local de la aplicación.
- **Supresión de Spam en Consola**: Supresión de advertencias y logs espurios de SDRplay cuando el servicio de Windows no se encuentra en ejecución.

### 5. Arquitectura Visual Responsiva Material 3 y Apple HIG
- **Sub-barras Atómicas Contextuales**: Reestructuración de las opciones contextuales en Espectro RF, Cascada y Dominio del Tiempo en filas semánticas explícitas para pantallas estrechas (< 620 px), eliminando por completo el quiebre vertical de caracteres en `dB Máx:` (`d\nB\nM\ná\nx\n:`).
- **Prevención de Colisiones de Texto en Constelación**: Síntesis automática del distintivo superior izquierdo a `"RF"` (36 px) o `"Grilla"` (52 px) y de la etiqueta vertical a `+Q` en gráficos compactos (< 340 px), manteniendo más de 65 px de margen libre respecto al semieje.
- **Barra de Reproducción en 3 Niveles Adaptativos**:
  - `< 480 px` (Móvil Ultra-compacto): Fila 1 de transporte y tiempo compacto (`mm:ss / mm:ss`, sin truncamientos); Fila 2 con slider de progreso a ancho completo; Fila 3 con velocidad, FFT y botón PNG.
  - `480 .. 820 px` (Tablet/Intermedio): 2 filas estructuradas con slider elástico.
  - `≥ 820 px` (Escritorio): Fila única balanceada.
- **Altura Contenida del Gráfico (`plot_h`)**: Relación de aspecto acotada en móvil y reserva vertical en escritorio, impidiendo que la barra de reproducción se sumerja detrás de la barra de navegación.
- **Simetría de Márgenes en Ajustes**: Márgenes uniformes de 16 px a izquierda y derecha en cualquier resolución de pantalla.
- **Iconografía Unificada Vectorial**: Uso exclusivo de símbolos del crate `egui_icons` (Google Material Symbols), sin emojis.
