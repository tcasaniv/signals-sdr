# Changelog

Todos los cambios notables en este proyecto serán documentados en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/).

---

## [0.4.5] - 2026-11-05

### Añadido (Features)
- **Suite VSA para Diagrama de Constelación I/Q en el Instrumento Virtual**:
  - **Modo Símbolos VSA**: Decimación a la tasa de símbolos con selección flexible de **SPS** (*Samples Per Symbol*: 1, 2, 4, 8, 16) para visualizar los puntos de decisión limpios en lugar de transiciones intermedias sobremuestreadas.
  - **Alineación OQPSK (De-offset Q)**: Retardo automático de $T_{\text{sym}} / 2 = \text{SPS} / 2$ muestras en el canal $Q$, colapsando instantáneamente la nube de OQPSK (ej. Meteor LRPT 72k) en 4 atractores nítidos de decisión.
  - **Sintonización de Fase de Ojo y Botón Auto-Ojo**: Ajuste micrométrico de la fase de muestreo ($0 \dots \text{SPS}-1$) con botón para centrar automáticamente la muestra en la máxima apertura del ojo.
  - **Desrotación Fina de Portadora**: Control angular continuo de $0^\circ \dots 360^\circ$ para alinear la orientación de constelaciones digitales (BPSK, QPSK, 8PSK, QAM).
  - **Normas de Escala RF vs Grilla Simbólica**:
    - *Norma de Potencia Unitaria RF ($E_s = 1.0$)*: Estándar de ingeniería de radiofrecuencia (CCSDS/SDR) con símbolos en $\pm 1/\sqrt{2}$ ($\pm 0.707$) sobre el círculo unitario $r=1.0$, aprovechando el 100% del rango dinámico del DAC sin saturación.
    - *Norma de Cuadrícula Simbólica ($\pm 1.0$)*: Normalizada por $\sqrt{2}$ para que los centros de decisión coincidan con las marcas cartesianas teóricas.
  - **Mediciones y Coordenadas Interactivas al Pasar el Cursor**: Cursor en cruz de alta precisión con cálculo matemático inverso del plano complejo: $I$, $Q$, magnitud de vector de símbolo $|s| = \sqrt{I^2 + Q^2}$, ángulo de fase $\theta = \operatorname{atan2}(Q, I)$ en grados, y nivel de señal relativo en decibelios ($20 \log_{10}|s|$ dBFS).
- **Exportación de Informes y Capturas de Laboratorio en Alta Resolución (PNG)**:
  - Exportación directa desde el Instrumento Virtual con renderizado anotado en resolución nativa 1920x1080.
  - Encabezado técnico institucional con metadatos del transmisor, frecuencia portadora central, tasa de muestreo y fecha/hora.
  - Parámetros de RF en vivo (SNR espectral, potencia de canal en dBFS, piso de ruido estimado) y configuración de constelación (Modo VSA, SPS, norma de potencia, rotación angular $\theta$).
- **Herramientas Avanzadas de Acondicionamiento y Limpieza I/Q (Paso 2 - Generic IQ)**:
  - **Filtro FIR Pasa-Bajos Simétrico (Denoising)**: Filtro de fase lineal con ventana de Hann y frecuencia de corte ajustable para suprimir ruido fuera de banda sin distorsionar la fase de los símbolos.
  - **Corrección Automática de Desbalance de Ganancia y Ortogonalidad I/Q**: Compensación de relación de amplitudes y ortogonalización Gram-Schmidt para eliminar imágenes espectrales y elipticidad.
  - **Inversión Espectral (Invert I/Q / Conjugate)**: Inversión de signo en el canal $Q$ para corregir grabaciones SDR con banda lateral invertida.
  - **Control Automático de Ganancia (AGC)**: Normalización inteligente de envolvente ante variaciones dinámicas de potencia.
  - **Supresor de Ruido / Squelch**: Umbral de corte configurable en dBFS.
  - **Bypass Automático y UI Didáctica**: Transparencia para señales sintéticas pre-generadas que no requieren acondicionamiento forzado.
- **Portabilidad Out-of-the-Box y Desacoplamiento de SoapySDR en Windows**:
  - **Delay-Loading (`/DELAYLOAD:SoapySDR.dll`)**: Eliminada la dependencia estática al inicio de Windows. La aplicación abre instantáneamente en cualquier equipo con Windows sin requerir instalaciones previas de Radioconda o PothosSDR ni arrojar la ventana *"No se encontró SoapySDR.dll"*.
  - **Auto-detección Dinámica y Registro en Tiempo de Ejecución**: Escaneo silencioso al arrancar en ubicaciones estándar (Radioconda, PothosSDR, Conda) y registro automático en el cargador dinámico de Windows con `SetDllDirectoryW` y `PATH`.
  - **Selector Manual de Entorno SoapySDR**: Banner intuitivo en la pestaña de Ajustes para alternar entre diferentes entornos instalados o forzar redetección con un clic.
  - **Scripts Asistentes de Soporte Multiplataforma**: Inclusión de `instalar-controladores-sdr.bat` en Windows y `instalar-dependencias.sh` en Linux/macOS dentro de los paquetes de distribución.

### Corregido (Fixes)
- **Diseño Responsivo Integral y Erradicación de Cortes Visuales en el Instrumento Virtual**:
  - **Sub-barras Atómicas Contextuales**: Eliminado el quiebre vertical de caracteres en `dB Máx:` (`d\nB\nM\ná\nx\n:`) en las pestañas de Espectro RF y Cascada mediante una distribución en 2 filas semánticas `ui.horizontal` explícitas para pantallas móviles (< 620 px).
  - **Prevención de Colisiones de Texto en Constelación I/Q**: En gráficos compactos (< 340 px), el distintivo superior izquierdo se sintetiza a `"RF"` (36 px) o `"Grilla"` (52 px) y la etiqueta del eje vertical se abrevia a `+Q`, manteniendo más de 65 px de margen libre y eliminando cualquier solapamiento.
  - **Sub-barra de Constelación Cohesiva**: Sliders y etiquetas (`Fase θ`, `Puntos`) agrupados de manera atómica sin saltos de línea desalineados.
  - **Barra de Reproducción en 3 Niveles Adaptativos**:
    - `< 480 px` (Móvil Ultra-compacto): Fila 1 de transporte y tiempo compacto (`mm:ss / mm:ss`, eliminando truncamiento); Fila 2 con barra de progreso a ancho completo; Fila 3 con velocidad, FFT y botón PNG.
    - `480 .. 820 px` (Tablet/Intermedio): 2 filas limpias con slider elástico.
    - `≥ 820 px` (Escritorio): Fila única balanceada.
  - **Altura Contenida del Gráfico (`plot_h`)**: Relación de aspecto acotada en móvil (máximo 260 px en espectro y 280 px en constelación) y reserva de margen vertical en escritorio, impidiendo que la barra de reproducción se sumerja o se oculte detrás de la barra de navegación.
- **Supresión de Spam en la Consola**: Supresión de advertencias espurias `sdrplay_api_Open() failed` cuando el servicio de Windows de SDRplay no se encuentra activo.

---

## [0.4.4] - 2026-10-31

### Añadido (Features)
- **Flujo Automatizado de Releases Multiplataforma**: Incorporado el workflow [`.github/workflows/release.yml`](.github/workflows/release.yml) para compilar, empaquetar y publicar automáticamente ejecutables nativos unificados para **Windows 10/11 x64** (`.zip`), **Linux x64** (`.tar.gz`) y **macOS Apple Silicon arm64** (`.tar.gz`) en cada nueva release de GitHub.
- **Aprovisionamiento Completo en Matriz de CI**: Soporte y validación continua en runners limpios de GitHub Actions para los 3 sistemas operativos (`ubuntu-latest`, `windows-latest`, `macos-latest`), aprovisionando bibliotecas del sistema de audio (ALSA `libasound2-dev`), transceptores SDR (`soapysdr-tools`, `libsoapysdr-dev` y `PothosSDR`) y subsistemas gráficos (X11, Wayland y OpenGL).

### Corregido (Fixes)
- **Congelamiento de WebAssembly en el Instrumento Virtual**: Migración del cálculo temporal de `std::time::Instant` a `web-time` en `VirtualSinkModal`, `TransmissionTabState` y `channel_effects`. Elimina el pánico de Rust (`time not implemented on this platform`) en navegadores web, permitiendo la renderización fluida del osciloscopio en cuadratura, espectrograma FFT con Peak Hold, cascada 2D y diagrama de constelación I/Q en WebAssembly.
- **Enrutamiento de Assets en GitHub Pages (Error 404)**: Configurado `public_url = "/signals-sdr/"` en `Trunk.toml` e inyectada la etiqueta canónica `<base data-trunk-public-url/>` en `web/index.html`. Los módulos `.js` y binarios `.wasm` ahora se resuelven correctamente bajo la subruta del repositorio sin buscar en la raíz del dominio.
- **Compilación Limpia de Trunk en CI**: Eliminada la directiva redundante de canonización `[watch].ignore = ["target"]` en `Trunk.toml` que causaba detenciones tempranas en runners donde la carpeta `target/` no existía al iniciar el build.
- **Estabilidad de Pruebas de Red RTL-TCP**: Añadida la opción `set_nodelay(true)` y sondeo reactivo con reintentos (`wait_gain`) en `test_rtl_tcp_gain_commands_by_index_and_direct`, previniendo fallos espurios causados por la latencia del programador de hilos del sistema operativo en runners bajo alta carga.

### Cambiado (Changed)
- **Migración Integral a Crates.io**: Todas las dependencias de codificadores y moduladores (`apt-encoder`, `lrpt-encoder`, `sstv-encoder`, `fm-modulator`, `am-modulator`) y abstracción de hardware SDR (`seify`) ahora apuntan al registro oficial de [crates.io](https://crates.io) en lugar de rutas de disco relativas locales (`path = "../..."`).
- **Autonomía y Compatibilidad con GitHub Workflows**: Los flujos de trabajo de CI/CD (`CI` multiplataforma en Linux/Windows/macOS, `deploy-pages` WebAssembly y `release`) y cualquier entorno externo de compilación ahora resuelven todas las dependencias sin requerir repositorios clonados en carpetas hermanas.
- **Soporte Nativo de Wayland y X11**: Activadas explícitamente las características `wayland` y `x11` en `eframe` para máxima compatibilidad gráfica en distribuciones Linux modernas y servidores gráficos tradicionales.

---

## [0.4.3] - 2026-10-26

### Corregido (Fixes)
- **Procesamiento Íntegro de Muestras I/Q en Efectos de Canal (Paso 3)**: Eliminado el límite artificial `limit` (300,000 / 600,000 muestras) en `dispatch_compute_channel_iq` que recortaba silenciosamente las señales moduladas a solo una fracción de su duración. En Desktop nativo, la totalidad de las muestras de la señal (ej. 17,596,560 muestras en señales largas de satélites o audio) se procesa íntegramente en un hilo de fondo del sistema operativo (`std::thread::spawn`), manteniendo una interfaz fluida a 60 FPS sin bloqueos.
- **Preservación Total de Señal en Transmisión (Paso 4)**: Al pasar al Paso 4 de Transmisión, la señal con efectos de canal (`processed_iq`) conserva exactamente el 100% de las muestras generadas, garantizando transmisiones y exportaciones completas sin cortes tempranos.
- **Navegación Reactiva Paso 3 -> Paso 4**: Incorporado el estado `pending_continue_to_tx` en `ChannelEffectsState`. Si el usuario pulsa *"Continuar a Transmisión"* mientras se están calculando los efectos en segundo plano, la interfaz muestra el estado en progreso y avanza automáticamente a Transmisión de manera fluida tan pronto el hilo finaliza.

### Añadido (Features)
- **Tasas de Muestreo Completas SDR# y SDR++ en RTL-TCP**: Ampliada la lista `RTL_SDR_SAMPLE_RATES` de 10 a 15 tasas estándar con cobertura total de los selectores de SDR# y SDR++: `0.25 MSps`, `0.90 MSps`, `1.024 MSps`, `1.4 MSps`, `1.536 MSps` (SDR++), `1.792 MSps` (SDR++), `1.8 MSps`, `1.92 MSps`, `2.048 MSps`, `2.16 MSps` (SDR++), `2.4 MSps`, `2.56 MSps` (SDR++), `2.8 MSps` (SDR#), `2.88 MSps` (SDR++) y `3.2 MSps`.
- **Test Unitario de Integridad para Señales de Gran Volumen**: Test `test_large_signal_preserves_length_without_truncation` en `src/channel/effects.rs` que valida que señales de más de un millón de muestras conservan su longitud exacta (1:1) al aplicar la cadena de canal.

---

## [0.4.2] - 2026-10-25

### Corregido (Fixes)
- **Piso de Ruido RTL-TCP a Bajas Ganancias**: Se incorporó el modelo de ruido de cuantización y térmico intrínseco del ADC RTL2832U de 8 bits ($\sigma_{\text{adc}} = 0.014$, equivalente a $\pm 1.5$ a $2$ LSBs continuos). Se elimina la línea muerta plana a -200 dBFS / -135 dBFS en SDR# y SDR++ cuando la ganancia se ajusta a 0.0 dB, manteniendo un piso de césped de ruido realista constante de ~ -75 dBFS a -70 dBFS.
- **Reloj de Transmisión RTL-TCP y Pacing**: La duración de los paquetes y el reloj de ritmo se sincronizan dinámicamente con la tasa de muestreo solicitada por el cliente SDR remoto mediante el comando `0x02` (`SET_SAMPLE_RATE`).
- **Panel Flexible de Ajustes en Escritorio**: Las tarjetas de los submenús de configuración en el panel secundario izquierdo ahora ocupan el 100% del ancho disponible (`ui.available_width()`), incorporando bordes redondeados M3 (10 px), padding simétrico y flecha/chevron derecho (`icons::ICON_CHEVRON_RIGHT`), manteniendo paridad visual con la experiencia móvil.

### Añadido (Features)
- **Tasas de Muestreo Estándar RTL-SDR**: Menú selector en el panel RTL-TCP con las tasas de muestreo estándar de SDR# y SDR++ (`0.25 MSps`, `0.90 MSps`, `1.024 MSps`, `1.4 MSps`, `1.8 MSps`, `1.92 MSps`, `2.048 MSps`, `2.4 MSps`, `2.56 MSps`, `3.2 MSps`) y botón de remuestreo automático con un solo clic.
- **Instrumento Virtual como Panel Flexible Integrado**: Se reemplazó la ventana modal flotante (`egui::Window`) por un panel flexible que se dibuja directamente en el área de trabajo derecha (`CentralPanel`). Ocupa el 100% del ancho y alto disponible sin expandirse fuera de la ventana ni tapar los menús laterales, incorporando una barra superior con selector de pestañas y botón de cierre `[X]` (`icons::ICON_CLOSE`) para regresar instantáneamente a la configuración del transmisor.

---

## [0.4.1] - 2026-10-25

### Añadido (Features)
- **Servidor RTL-TCP Didáctico**: Emulación del dongle RTL-SDR sobre puerto TCP 1234 con cabecera `RTL0` y soporte para SDR#, GQRX, SDR++, WebSDR y GNU Radio.
- **Soporte de Ganancia por Índice `0x0D` (`SET_TUNER_GAIN_BY_INDEX`)**: Implementación de los 29 pasos de ganancia del sintonizador R820T/R820T2 (0.0 dB a 49.6 dB) con framing TCP acumulativo libre de desalineación por fragmentación.
- **Simulación Logarítmica de Ganancia y Saturación**: Atenuación en decibelios reales ($10^{(\text{gain} - 38)/20}$), punto óptimo a 38 dB (0 dBFS nominal) y elevación exponencial del ruido térmico al exceder 38 dB.
- **Control de Bucle y Re-transmisión**: Toggle de repetición continua con emisión de piso de ruido AWGN continuo al terminar la señal y botón `Re-transmitir Señal` en caliente sin desconectar al cliente SDR.
- **Atenuación Docente Dinámica**: Deslizador de potencia TX de 0.0 dB a -60.0 dB con sincronización atómica en tiempo real.
- **Instrumento Virtual**:
  - Trazas independientes de Max Peak Hold (amarillo) y Min Peak Hold (magenta) con botón de reset.
  - Escalas dinámicas de los ejes X e Y con sliders de rango en dB y marcas de graduación adaptativas.
  - Opciones de FFT ampliadas hasta 16384 puntos (512 a 16384).
  - Resolución de cascada (waterfall) configurable desde 64 columnas hasta resolución nativa de la FFT.
  - Constelación I/Q calibrada con círculos concéntricos de referencia ($r=0.5, 1.0$) y marcas en $\pm 1.0$ y $\pm 0.5$ con zoom.
- **Navegación Móvil y Ajustes**: Pestaña independiente de "Apariencia y Tema" (Modo Claro/Oscuro), menú móvil de Ajustes como lista vertical de tarjetas y navegación a pantalla completa con botón flecha atrás.
- **Apertura de Enlaces en Navegador Nativo**: Lanzamiento confiable en Windows con `rundll32 url.dll,FileProtocolHandler`.
- **Visor de Imágenes Responsivo**: Encabezado en dos filas para anchos de pantalla móvil (< 550 px).

---

## [0.4.0] - 2026-10-24

### Añadido (Features)
- **Suite Integral de Transmisores y Moduladores**: NOAA APT, Meteor LRPT, ISS SSTV, Generic SSTV, Generic FM, Generic AM y Generic I/Q.
- **Streaming de Red GNU Radio**: Soporte TCP Server y UDP Sender en formatos Complex Float32 (`fc32`) y Complex Int16 (`sc16`).
- **Remuestreo Polifásico y DSP**: Remuestreador de muestras I/Q con interpolación cúbica Hermite.
- **Exportación Multi-formato**: Formatos WAV IQ 16-bit, SC16 Q11 (BladeRF), SC16 Full, Int8 (HackRF) y Float32.
