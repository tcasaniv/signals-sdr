# Signals SDR: Roadmap y Plan de Desarrollo

> **Documento de referencia para desarrolladores, colaboradores y agentes de Inteligencia Artificial.**
> Define la arquitectura, estado actual y próximas fases de desarrollo de **Signals SDR**.

---

## 1. Misión y Visión Educativa

**Signals SDR** nace con el objetivo de servir como una plataforma de entrenamiento, laboratorio y docencia para universidades, institutos técnicos y comunidades de radioaficionados.

### El Problema
Tradicionalmente, quienes aprenden radio definida por software (SDR), telecomunicaciones aeroespaciales o procesamiento digital de señales (DSP) dependen de:
1. El paso impredecible de satélites meteorológicos sobre su horizonte (con pocas pasadas al día y duración de 10 a 15 minutos).
2. Equipos de recepción al aire libre expuestos al clima y problemas de antena.
3. Dificultad para reproducir casos de estudio con condiciones controladas.

### La Solución de Signals SDR
Signals SDR invierte el flujo: permite **generar transmisiones idénticas a las señales reales de satélites y emisoras**, exportarlas en archivos I/Q o transmitirlas por cable/antena a baja potencia con un dispositivo SDR (HackRF, bladeRF, USRP). De este modo, los estudiantes y desarrolladores pueden:
- Validar sus algoritmos de demodulación en tiempo real.
- Entrenar software decodificador (SatDump, SDR++, Gqrx, QSSTV, GNU Radio).
- Realizar pruebas repetibles y medibles en el aula de clases o en casa.

---

## 2. Estado de Implementación (Fase 1 - Actual)

### 2.1. Arquitectura de UI/UX
- **Framework:** `egui / eframe` en Rust puro.
- **Diseño Mobile-First y Responsive:**
  - Breakpoint dinámico en 720 px de ancho.
  - Modo móvil: Barra inferior de 2 secciones con navegación *Master-Detail* y botón de retorno.
  - Modo escritorio: Barra lateral izquierda, buscador de pipelines y panel de trabajo multitarea.
- **Iconografía Vectorial:** Ausencia total de caracteres emoji, empleando trazos geométricos nítidos y/o Material Design Icons [`egui_icons`](https://crates.io/crates/egui_icons).
- **Temas:** Modo Claro y Modo Oscuro persistentes.

### 2.2. Pipelines Implementados
1. **NOAA APT (Automatic Picture Transmission):**
   - Entrada: 1 o 2 imágenes (Canal A y Canal B).
   - Subpestaña 2:
     - Preprocesamiento (escala de grises, ancho 909 px, alineación de menor altura).
     - Codificación de trama APT (2080 px ancho con sincro A/B, espacios y telemetría) exportable a PNG.
     - Modulación AM sobre subportadora de 2400 Hz (audio WAV a 11025 Hz).
   - Subpestaña 3: Modulación FM I/Q (desviación 17 kHz, 288 kHz / 1 MHz) para transmisión SDR o exportación I/Q.
2. **METEOR LRPT (Low Rate Picture Transmission):**
   - Entrada: 1 a 3 imágenes (Ch1, Ch2, Ch3) o descomposición RGB.
   - Subpestaña 2: Compresión JPEG DCT, multiplexado CCSDS VCDU, Reed-Solomon (255,223), Scrambler CCSDS, Convolucional FEC ($r=1/2, K=7$).
   - Subpestaña 3: Modulación digital OQPSK (72 ksps, filtro RRC $\alpha=0.5$) a 288 kHz.
3. **ISS SSTV (Slow Scan Television):**
   - Entrada: 1 imagen.
   - Preset específico de la Estación Espacial Internacional (Robot 36, PD 120, Martin 1).
4. **Generic SSTV:**
   - Selector libre de modos SSTV: Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 120/180/290, BW 8/12.
   - Síntesis analógica de tonos VIS y modulación de barrido de líneas.
5. **Generic FM:**
   - Entrada: Audio WAV o MP3.
   - Presets WFM (75 kHz), NFM (5 kHz), APT (17 kHz), pre-énfasis analógico (50 µs / 75 µs / plano), y desplazamiento IF.
6. **Generic AM:**
   - Entrada: Audio WAV o MP3.
   - Modos: DSB-FC (portadora completa con índice $m$), DSB-SC (portadora suprimida), SSB (USB/LSB por cuadratura Hilbert).
7. **Generic IQ:**
   - Entrada: Archivos I/Q pregrabados (WAV estéreo, RAW SC16, Float32, Int8).
   - Inspección espectral FFT, osciloscopio y retransmisión con SDR o conversión de formato.

### 2.3. Subpestaña "3. Transmisión"
- Modo mutuamente excluyente:
  - **Hardware SDR:** Selección de dispositivo con `seify`, frecuencia central RF, tasa de muestreo, ganancia TX, interruptor de amplificador de potencia (PA para HackRF) y botón de transmisión.
  - **Archivo IQ:** Selector de formato (SC16_Q11 para bladeRF, WAV para SDR#, RAW int8 para HackRF, Float32), selector de ruta y botón de guardado (sin controles de RF).

### 2.4. Gestión de Archivos en Ajustes
- Selector de carpeta raíz por defecto.
- Subcarpetas automáticas: `noaa_apt/`, `meteor_lrpt/`, `iss_sstv/`, `generic_sstv/`, `generic_fm/`, `generic_am/`, `generic_iq/`.

---

## 3. Próximas Fases de Desarrollo

### Fase 2: Simulación de Canal de Radio y Ruido (Q1 2027)
Para enriquecer la experiencia docente, permitir añadir degradaciones de canal controladas a las señales antes de exportarlas o transmitirlas:
- **Ruido Blanco Gaussiano Aditivo (AWGN):** Control de relación señal a ruido (SNR en dB).
- **Desplazamiento Doppler:** Simulación del paso orbital del satélite (variación de frecuencia $\pm 3\text{ kHz}$ a lo largo del tiempo).
- **Fading y Atenuación Rayleigh/Rician:** Simulación de multitrayecto o pérdidas por obstáculos.

### Fase 3: Nuevos Protocolos Satelitales y Digitales (Q2 2027)
1. **NOAA HRPT (High Resolution Picture Transmission):**
   - Frecuencia: 1698.0 / 1707.0 MHz (Banda L).
   - Tasa de bits: 665.4 kbps con modulación digital BPSK.
   - Telemetría AVHRR de 5 canales espectrales en tiempo real.
2. **GOES LRIT / HRIT:**
   - Transmisión digital de satélites geoestacionarios meteorológicos con tramas CCSDS y modulación QPSK.
3. **Inmarsat AERO / STD-C:**
   - Señales de comunicaciones marítimas y aeronáuticas en banda L (~1.5 GHz).
4. **APRS (Automatic Packet Reporting System):**
   - Paquetes AX.25 modulados en Bell 202 AFSK (1200 baudios) en 144.390 / 144.800 MHz.
5. **LoRa (Long Range):**
   - Modulación CSS (Chirp Spread Spectrum) para telemetría educativa de picosatélites (PocketQube / CubeSat).

### Fase 4: Despliegue WebAssembly y Soporte Móvil WebUSB (Q3 2027)
- **Web App PWA:** Ejecutable en cualquier navegador sin instalar software.
- **WebUSB en Smartphones:**
  - Utilizar el soporte WebUSB del crate `seify` para conectar dispositivos HackRF o bladeRF vía cable USB-OTG directamente a teléfonos Android.
  - Transmisión de señales de satélite desde la palma de la mano en actividades de campo de radioafición y talleres educativos.

### Fase 5: Integración con Bloques GNU Radio y GNU Radio Companion (Q4 2027)
- Exportación directa de grafos de flujo (`.grc`) que reproduzcan la señal generada.
- Bloques Rust para integración en pipelines de investigación universitaria.

---

## 4. Guía para Nuevos Desarrolladores y Agentes de IA

### Cómo Añadir un Nuevo Pipeline
Para incorporar un nuevo protocolo a **Signals SDR**:

1. **Crear el módulo en `src/pipelines/<nombre>.rs`:**
   Implementar la estructura con los tres estados:
   ```rust
   pub struct MiPipeline {
       // Datos de fuente (Paso 1)
       // Parámetros de modulación (Paso 2)
       // Muestras I/Q generadas
   }
   ```
2. **Implementar las vistas de las tres subpestañas:**
   - `render_source(&mut self, ui: &mut egui::Ui)`
   - `render_process(&mut self, ui: &mut egui::Ui)`
   - La subpestaña 3 utiliza el componente común `ui::transmission::render_transmission_tab`.
3. **Registrar el pipeline en `src/pipelines/mod.rs`:**
   Añadir la variante a la enumeración `PipelineKind` y asignarle nombre legible, descripción y categoría.
4. **Añadir pruebas unitarias en `tests/pipeline_tests.rs`:**
   Verificar que la modulación genera muestras válidas sin entrar en pánico.

### Reglas de Estilo del Proyecto
- **Sin emojis:** Todo componente visual debe usar texto limpio o trazos vectoriales en `src/icons.rs` o usar Material Design Icons [`egui_icons`](https://crates.io/crates/egui_icons).
- **Lineamientos de UI**: Material Design 3 principalmente, con algunos conceptos de Apple Human Interface Guidelines, Fluent 2 + Windows 11 design principles y diseño mobile first con soporte desktop.
- **Rendimiento UI:** Operaciones pesadas de DSP o I/O deben ejecutarse en hilos secundarios (`std::thread::spawn` o canales `mpsc`) para no congelar los 60 FPS de `egui`.
- **Multiplataforma:** Evitar llamadas a APIs específicas de Windows/Linux/macos/android/ios/web sin condicionales `#[cfg(...)]`.
