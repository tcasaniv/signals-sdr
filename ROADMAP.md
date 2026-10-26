# Signals SDR: Roadmap y Plan de Desarrollo

> **Documento de referencia para desarrolladores, colaboradores y agentes de Inteligencia Artificial.**
> Define la arquitectura, el estado de implementación actual (**v0.4.3**) y las próximas fases de desarrollo hacia nuevos protocolos satelitales e integraciones.

---

## 1. Misión y Visión Educativa

**Signals SDR** es una suite de generación, modulación y transmisión de señales de radiofrecuencia (RF) y satelitales, diseñada para servir como plataforma de entrenamiento, laboratorio interactivo y docencia en universidades, institutos técnicos y comunidades de radioaficionados.

### El Problema
Tradicionalmente, quienes aprenden radio definida por software (SDR), telecomunicaciones aeroespaciales o procesamiento digital de señales (DSP) dependen de:
1. El paso impredecible de satélites meteorológicos sobre su horizonte (con pocas pasadas al día y duración limitada de 10 a 15 minutos).
2. Equipos de recepción al aire libre expuestos al clima, interferencias locales y problemas de antena.
3. Dificultad para reproducir casos de estudio con condiciones controladas de ruido, desvanecimiento o Doppler.

### La Solución de Signals SDR
Signals SDR invierte el flujo: permite **generar transmisiones idénticas a las señales reales emitidas por satélites y transmisores analógicos**, exportarlas en archivos I/Q o transmitirlas por cable/antena a baja potencia con un dispositivo SDR (HackRF, bladeRF, USRP) o a través de la red local mediante RTL-TCP o streaming UDP. De este modo, estudiantes e investigadores pueden:
- Validar sus algoritmos de demodulación en tiempo real.
- Entrenar y probar software decodificador existente (SatDump, SDR++, GQRX, SDR#, WXtoImg, QSSTV, GNU Radio).
- Realizar prácticas de laboratorio repetibles, medibles y reproducibles en el aula de clases o en casa.

---

## 2. Estado de Implementación (Versión v0.4.3 - Actual)

### 2.1. Arquitectura de UI/UX Unificada (Material 3 + Apple HIG)
- **Framework:** `egui / eframe` en Rust puro a 60 FPS estables.
- **Diseño Adaptativo y Responsivo:**
  - Breakpoint dinámico en **720 px de ancho**.
  - **Modo Escritorio ($\ge$ 720 px):**
    - Navigation Rail lateral estrecho (72 px) con píldoras de selección M3 e interruptor de tema visual.
    - Panel de catálogo deslizable y redimensionable libremente por el usuario (220 px a 450 px) con sincronización activa de arrastre.
    - Barra de búsqueda instantánea Material 3 con botón de borrado `✕` dinámico delimitado.
    - Chips de filtrado rápido con desplazamiento horizontal (`Todos`, `Satélites`, `SSTV`, `Analógicas`, `I/Q DSP`).
    - Panel secundario de Ajustes con tarjetas interactivas a ancho completo (`ui.available_width()`), bordes redondeados M3 (10 px) e iconos vectoriales de chevron derecho.
  - **Modo Móvil ($<$ 720 px):**
    - Barra superior táctil con botón de retorno asistido en flujo *Master-Detail*.
    - Barra inferior de navegación con indicadores de píldora M3 e iconos vectoriales.
- **Iconografía Vectorial:** Ausencia total de emojis en la interfaz gráfica. Uso exclusivo de **Google Material Symbols & Icons** (`egui_icons` y [`crate::icons`]).
- **Temas:** Modo Claro y Modo Oscuro persistentes con paleta tonal Material 3.

### 2.2. Flujo Didáctico de 4 Subpestañas por Pipeline
Cada pipeline se divide transparentemente en 4 etapas pedagógicas:
1. **Subpestaña 1 (Elegir fuente):** Carga de imágenes locales, descarga por URL, audios WAV/MP3 o archivos I/Q pregrabados.
2. **Subpestaña 2 (Codificar / Modular):** Explicación visual paso a paso de cada fase interna de procesamiento, preprocesamiento de imagen y síntesis de modulación.
3. **Subpestaña 3 (Efectos de canal):** Simulación de degradaciones de RF antes de la emisión.
4. **Subpestaña 4 (Transmisión):** Transmisión con hardware SDR, servidor RTL-TCP, streaming UDP GNU Radio o guardado en archivo I/Q.

### 2.3. Simulación de Canal y Ruido en Segundo Plano (Sin Truncamiento)
- **AWGN (Ruido Blanco Gaussiano Aditivo):** Control continuo de relación señal a ruido (SNR desde -10 dB hasta +40 dB) mediante generador pseudoaleatorio ultrarrápido y determinista (`FastRng`).
- **Desplazamiento Doppler:** Simulación del paso orbital del satélite con desviación temporal de portadora (modelo hiperbólico LEO $\pm 3\text{ kHz}$, deriva lineal y offset estático).
- **Fading y Atenuación:** Presets didácticos para simular desvanecimiento por multitrayecto (Rician / Rayleigh) y pérdidas de espacio libre (FSPL).
- **Cálculo en Hilo Secundario sin Truncamiento:** Procesamiento asíncrono sobre un hilo de fondo del sistema operativo (`std::thread::spawn`), garantizando la preservación 1:1 del 100% de las muestras de la señal (ej. 17.5M+ muestras) para la transmisión continua.

### 2.4. Servidor RTL-TCP Didáctico Integrado
- Emulación del dongle RTL2832U sobre TCP (`127.0.0.1:1234`) con cabecera `RTL0` y sintonizador R820T.
- Soporte para **SDR#**, **SDR++**, **GQRX**, **WebSDR** y **GNU Radio**.
- Decodificación robusta con framing acumulativo de los comandos de ganancia por índice `0x0D` (29 pasos de ganancia de 0.0 a 49.6 dB) y ganancia directa `0x04`.
- Modelo físico de dither continuo del ADC ($\sigma_{\text{adc}} = 0.014$) con piso de ruido constante a **~ -75 dBFS** (eliminando el espectro plano a -200 dBFS).
- 15 tasas de muestreo estándar (0.25 MSps a 3.2 MSps) y adaptación dinámica del reloj de pacing.

### 2.5. Instrumento Virtual de Laboratorio (Panel Flexible Integrado)
- Integrado directamente en el panel flexible derecho (`CentralPanel`) con botón de cierre `[X]` sin modales flotantes invasivos.
- **Espectro RF (FFT):** Visualización de magnitud en dB con ventana configurable y retención de picos máximos (amarillo) y mínimos (magenta).
- **Cascada Espectral (Waterfall):** Representación bidimensional tiempo-frecuencia con gradientes de color dinámicos.
- **Dominio del Tiempo:** Osciloscopio en cuadratura para señales $I(t)$, $Q(t)$ y envolvente $|IQ|$ con controles de zoom y pan.
- **Constelación I/Q:** Diagrama de plano complejo para analizar constelaciones digitales.
- **Métricas de Señal:** Telemetría en vivo con cálculo de potencia pico (dBFS), RMS, factor de cresta (PAPR), desbalance DC y aislamiento de portadora.

### 2.6. Transmisores Satelitales y Analógicos Disponibles
1. **NOAA APT (Automatic Picture Transmission):** 137.100 / 137.9125 MHz, trama APT analógica (2080 px/línea), subportadora AM 2400 Hz, modulación FM 17 kHz.
2. **METEOR LRPT (Low Rate Picture Transmission):** 137.100 / 137.900 MHz, JPEG DCT $8\times 8$, paquetes CCSDS Space Packet y VCDU, Reed-Solomon (255,223), Scrambler, Convolucional FEC ($k=7, r=1/2$), modulación digital OQPSK a 72 ksps.
3. **ISS SSTV (Estación Espacial Internacional):** 145.800 MHz FM estrecha, modos Robot 36, PD 120 y Martin 1.
4. **Generic SSTV (Multiprotocolo):** 11 modos de radioaficionados (Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 120/180/290, BW 8/12).
5. **Generic FM (Frecuencia Modulada):** Presets WFM (75 kHz), NFM (5 kHz), APT (17 kHz), pre-énfasis seleccionable (50 µs / 75 µs / plano) y offset IF.
6. **Generic AM (Amplitud Modulada):** DSB-FC (con índice $m$), DSB-SC, SSB USB y SSB LSB mediante transformada analítica de Hilbert.
7. **Generic I/Q (Gestor y Retransmisor I/Q):** Carga de archivos pregrabados, análisis espectral FFT, corrección de offset DC y rotación Doppler.
8. **NOAA HRPT Didáctico:** Banda L (1.7 GHz) con modulación digital BPSK Split-Phase-L (Manchester) a 665.4 kbps.

### 2.7. Destinos de Emisión y Formatos I/Q
- **Hardware SDR:** Selector jerárquico de 2 etapas con Seify (HackRF, bladeRF, USRP, SoapySDR, Dummy).
- **Streaming GNU Radio UDP:** Envío de paquetes I/Q en formatos `Float32`, `Complex32` e `Int16`.
- **Formatos de Archivo I/Q:** WAV I/Q PCM 16 bits estéreo, RAW SC16_Q11 (bladeRF-cli), RAW SC16 completo, RAW int8 (hackrf_transfer), RAW float32 (GNU Radio / inspectrum) y WAV Audio Mono.
- **Suite de Pruebas:** 32 pruebas automatizadas unitarias y de integración pasando al 100%.

---

## 3. Próximas Fases de Desarrollo

### Fase 4: Protocolos Satelitales y de Telemetría Avanzados (v0.5.0 - Q1-Q2 2027)
1. **NOAA HRPT Completo:** Flujo de tramas Minor Frame / Major Frame con telemetría AVHRR de 5 canales espectrales en tiempo real.
2. **GOES LRIT / HRIT:** Transmisión digital de satélites meteorológicos geoestacionarios con tramas CCSDS y modulación QPSK.
3. **APRS (Automatic Packet Reporting System):** Generador de paquetes AX.25 modulados en Bell 202 AFSK (1200 baudios) en 144.390 / 144.800 MHz.
4. **Inmarsat AERO / STD-C:** Señales de comunicaciones marítimas y aeronáuticas en banda L (~1.5 GHz).
5. **LoRa CSS (Chirp Spread Spectrum):** Modulación de espectro ensanchado para telemetría de picosatélites (PocketQube / CubeSat).

### Fase 5: Integración con GNU Radio y Herramientas Externas (v0.6.0 - Q3-Q4 2027)
- Generación y exportación directa de grafos de flujo GNU Radio Companion (`.grc`).
- Generación de scripts en Python / C para reproducir transmisiones de forma headless en servidores remotos.
- Módulos Out-Of-Tree (OOT) en Rust para GNU Radio 3.10+.
- Servidor WebRTC para audio y streaming de cascada de bajísima latencia en navegadores web.

---

## 4. Guía para Nuevos Desarrolladores y Agentes de IA

1. **Documentación con Rustdoc:** Ejecute `cargo doc --no-deps --open` para explorar la referencia completa de la arquitectura y la API del crate.
2. **Convenciones de Código:** Mantener cero emojis en la interfaz de usuario; emplear exclusivamente [`crate::icons`] y `egui_icons`.
3. **Pruebas Continuas:** Cada nuevo modulador o destino de transmisión debe incluir pruebas unitarias en `tests/` y mantener limpias las 32 pruebas existentes mediante `cargo test`.
