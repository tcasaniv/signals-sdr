# Signals SDR: Roadmap y Plan de Desarrollo

> **Documento de referencia para desarrolladores, colaboradores y agentes de Inteligencia Artificial.**
> Define la arquitectura, el estado de implementación actual (**v0.2.1**) y las próximas fases de desarrollo hacia el release web (**v0.3.0**) y nuevos protocolos.

---

## 1. Misión y Visión Educativa

**Signals SDR** es una suite de generación, modulación y transmisión de señales de radiofrecuencia (RF) y satelitales, diseñada para servir como plataforma de entrenamiento, laboratorio interactivo y docencia en universidades, institutos técnicos y comunidades de radioaficionados.

### El Problema
Tradicionalmente, quienes aprenden radio definida por software (SDR), telecomunicaciones aeroespaciales o procesamiento digital de señales (DSP) dependen de:
1. El paso impredecible de satélites meteorológicos sobre su horizonte (con pocas pasadas al día y duración limitada de 10 a 15 minutos).
2. Equipos de recepción al aire libre expuestos al clima, interferencias locales y problemas de antena.
3. Dificultad para reproducir casos de estudio con condiciones controladas de ruido, desvanecimiento o Doppler.

### La Solución de Signals SDR
Signals SDR invierte el flujo: permite **generar transmisiones idénticas a las señales reales emitidas por satélites y transmisores analógicos**, exportarlas en archivos I/Q o transmitirlas por cable/antena a baja potencia con un dispositivo SDR (HackRF, bladeRF, USRP). De este modo, estudiantes e investigadores pueden:
- Validar sus algoritmos de demodulación en tiempo real.
- Entrenar y probar software decodificador existente (SatDump, SDR++, Gqrx, SDR#, WXtoImg, QSSTV, GNU Radio).
- Realizar prácticas de laboratorio repetibles, medibles y reproducibles en el aula de clases o en casa.

---

## 2. Estado de Implementación (Versión v0.2.1 - Actual)

### 2.1. Arquitectura de UI/UX Unificada (Material 3 + Apple HIG)
- **Framework:** `egui / eframe` en Rust puro a 60 FPS estables.
- **Diseño Adaptativo y Responsivo:**
  - Breakpoint dinámico en **720 px de ancho**.
  - **Modo Escritorio ($\ge$ 720 px):**
    - Navigation Rail lateral estrecho (72 px) con píldoras de selección M3 e interruptor de tema visual.
    - Panel de catálogo deslizable y redimensionable libremente por el usuario (220 px a 450 px) con sincronización activa de arrastre.
    - Barra de búsqueda instantánea Material 3 con botón de borrado `✕` dinámico delimitado.
    - Chips de filtrado rápido con desplazamiento horizontal (`Todos`, `Satélites`, `SSTV`, `Analógicas`, `I/Q DSP`).
  - **Modo Móvil ($<$ 720 px):**
    - Barra superior táctil con botón de retorno asistido en flujo *Master-Detail*.
    - Barra inferior de navegación con indicadores de píldora M3 e iconos vectoriales.
- **Iconografía Vectorial:** Ausencia total de emojis en la interfaz gráfica. Uso exclusivo de **Google Material Symbols & Icons** (`egui_icons`) y trazados vectoriales limpios.
- **Temas:** Modo Claro y Modo Oscuro persistentes con paleta tonal Material 3.

### 2.2. Flujo Didáctico de 4 Subpestañas por Pipeline
Cada pipeline se divide transparentemente en 4 etapas pedagógicas:
1. **Subpestaña 1 (Elegir fuente):** Carga de imágenes locales, descarga por URL, audios WAV/MP3 o archivos I/Q pregrabados.
2. **Subpestaña 2 (Codificar / Modular):** Explicación visual paso a paso de cada fase interna de procesamiento, preprocesamiento de imagen y síntesis de modulación.
3. **Subpestaña 3 (Efectos de canal):** Simulación de degradaciones de RF antes de la emisión.
4. **Subpestaña 4 (Transmisión):** Transmisión con hardware SDR o guardado en archivo I/Q.

### 2.3. Simulación de Canal y Ruido (Fase 2 Completada)
- **AWGN (Ruido Blanco Gaussiano Aditivo):** Control continuo de relación señal a ruido (SNR desde -10 dB hasta +40 dB) mediante generador pseudoaleatorio ultrarrápido y determinista.
- **Desplazamiento Doppler:** Simulación del paso orbital del satélite con desviación temporal de portadora de hasta $\pm 3\text{ kHz}$.
- **Fading y Atenuación:** Presets didácticos para simular desvanecimiento por multitrayecto y pérdidas de espacio libre.

### 2.4. Instrumentación de Laboratorio Integrada
- **Analizador de Espectro FFT de Alta Resolución:** Visualización de magnitud en dB con ventana configurable y retención de picos.
- **Cascada Espectral (Waterfall):** Representación bidimensional en tiempo y frecuencia con gradiente de color dinámico para inspeccionar la firma espectral de la señal.
- **Osciloscopio Interactivo:** Inspección en el dominio del tiempo con controles de zoom temporal y exportación de gráficos.
- **Reproductor de Audio en la Aplicación:** Monitoreo acústico inmediato de las señales generadas (FM, AM, audio subportadora APT y tonos SSTV) sin salir del programa.

### 2.5. Transmisores Satelitales y Analógicos Disponibles
1. **NOAA APT (Automatic Picture Transmission):**
   - Entrada: 1 o 2 imágenes (Canal A visible, Canal B infrarrojo).
   - Preproceso (escala de grises, 909 px ancho) $\to$ Generación de trama APT (2080 px ancho con sincro A/B, marcas de minuto y telemetría) $\to$ Modulación AM sobre subportadora de 2400 Hz $\to$ Modulación FM I/Q a 137.1 / 137.9125 MHz.
2. **METEOR LRPT (Low Rate Picture Transmission):**
   - Entrada: 1 a 3 imágenes (Ch1, Ch2, Ch3) o descomposición RGB.
   - Compresión JPEG DCT $8\times 8$ $\to$ Empaquetado CCSDS Space Packet y VCDU $\to$ Reed-Solomon (255,223) $\to$ Scrambler $\to$ Convolucional FEC ($k=7, r=1/2$) $\to$ Modulación digital OQPSK a 72 ksps a 137.1 / 137.9 MHz.
3. **ISS SSTV (Estación Espacial Internacional):**
   - Modos oficiales conmemorativos de la ISS: Robot 36, PD 120, Martin 1 modulados en FM estrecha en 145.800 MHz.
4. **Generic SSTV (Multiprotocolo):**
   - Soporte completo para 11 modos de radioaficionados: Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 120/180/290, BW 8/12 con síntesis analógica de tonos VIS.
5. **Generic FM (Frecuencia Modulada):**
   - Presets WFM (75 kHz), NFM (5 kHz), APT (17 kHz), pre-énfasis seleccionable (50 µs Europa / 75 µs EE.UU. / plano) y offset de frecuencia intermedia IF.
6. **Generic AM (Amplitud Modulada):**
   - Modos DSB-FC (Portadora Completa con índice $m$), DSB-SC (Portadora Suprimida), SSB USB y SSB LSB mediante transformada analítica de Hilbert.
7. **Generic I/Q (Gestor y Retransmisor I/Q):**
   - Entrada: Archivos I/Q pregrabados (`.wav`, `.raw`, `.iq`, `.sc16`, `.bin`).
   - Inspección espectral FFT, corrección de offset DC, rotación de portadora y retransmisión SDR o transcodificación.
8. **NOAA HRPT Didáctico:**
   - Módulo formativo de banda L a 1698.0 / 1707.0 MHz con modulación digital BPSK Manchester Split-Phase-L a 665.4 kbps.

### 2.6. Transmisión SDR y Formatos I/Q
- **Selector Jerárquico de 2 Etapas:**
  1. *Etapa 1:* Tipo de Dispositivo (HackRF One, Nuand bladeRF, Ettus USRP, SoapySDR o Simulación Dummy).
  2. *Etapa 2:* Selector de Dispositivo por Número de Serie / Dirección IP con detección de instancias conectadas.
- **Formatos de Archivo I/Q:** WAV I/Q PCM 16 bits estéreo, RAW SC16_Q11 (bladeRF-cli), RAW SC16 completo, RAW int8 (hackrf_transfer), RAW float32 (GNU Radio / inspectrum) y WAV Audio Mono.
- **Organización Automática de Carpetas:** Selector persistente de ruta raíz con subcarpetas automáticas (`noaa_apt/`, `meteor_lrpt/`, etc.).

---

## 3. Próximas Fases de Desarrollo

### Fase 3: Arquitectura Dual Web y Release v0.3.0 (En Progreso)
El hito **v0.3.0** implementará el soporte web a través de dos vertientes complementarias:

1. **Modalidad WebAssembly Estática (GitHub Pages + Dominio Personalizado):**
   - Despliegue estático a coste $0$ en GitHub Pages asociado a dominio personalizado mediante archivo `CNAME`.
   - Toda la interfaz `egui`, procesamiento DSP, síntesis de modulación, espectrograma cascada y efectos de canal se ejecutan 100% en el navegador del cliente.
   - Generación y descarga directa en el navegador de archivos `.wav` y `.iq`.
   - Avisos pedagógicos de limitaciones de seguridad web (incompatibilidad de drivers nativos USB en sandbox web) para evitar confusiones de los usuarios.
   - Soporte experimental de **WebUSB** para conexión directa de HackRF en navegadores basados en Chromium (Chrome, Edge, Android).

2. **Modalidad Servidor Web LAN (`signals-sdr --serve`):**
   - Flag de ejecución en el binario nativo: `signals-sdr --serve [puerto] [bind-address]` (por defecto abre la interfaz de escritorio nativa si no se especifican argumentos).
   - Inicia un servidor web interno ligero (Axum / WebSockets) para exponer la interfaz web en la red local (`http://192.168.1.X:8080`).
   - Permite controlar la transmisión SDR físicamente conectada a la PC o Raspberry Pi anfitriona de forma inalámbrica desde teléfonos Android, iPhone, iPad o laptops sin necesidad de cables OTG.

3. **Flujo de Integración Continua (CI/CD Web):**
   - Workflow en GitHub Actions con `trunk build --release` para desplegar automáticamente el bundle WebAssembly en cada etiqueta de versión o push a `main`.

### Fase 4: Nuevos Protocolos Satelitales y de Telemetría (Q2 2027)
1. **NOAA HRPT Completo:** Flujo de tramas Minor Frame / Major Frame con telemetría AVHRR de 5 canales espectrales en tiempo real.
2. **GOES LRIT / HRIT:** Transmisión digital de satélites geoestacionarios con tramas CCSDS y modulación QPSK.
3. **Inmarsat AERO / STD-C:** Señales de comunicaciones marítimas y aeronáuticas en banda L (~1.5 GHz).
4. **APRS (Automatic Packet Reporting System):** Paquetes AX.25 modulados en Bell 202 AFSK (1200 baudios) en 144.390 / 144.800 MHz.
5. **LoRa CSS (Chirp Spread Spectrum):** Modulación de espectro ensanchado para telemetría de picosatélites (PocketQube / CubeSat).

### Fase 5: Ecosistema GNU Radio y Exportación de Código (Q3-Q4 2027)
- Exportación directa de grafos de flujo GNU Radio Companion (`.grc`).
- Módulos Out-Of-Tree (OOT) en Rust para GNU Radio 3.10+.
- Generación de scripts en Python / C para reproducir transmisiones de forma headless.

---

## 4. Guía para Nuevos Desarrolladores y Agentes de IA

Para incorporar nuevos transmisores, optimizaciones DSP o mejoras de UI a **Signals SDR**:

1. **Consulta la documentación de contribución:** Lee y sigue estrictamente las políticas de [CONTRIBUTING.md](CONTRIBUTING.md), en particular la **Política de Responsabilidad Humana en Pull Requests generados o asistidos por Inteligencia Artificial**.
2. **Estructura modular de pipelines:** Implementa la estructura en `src/pipelines/<nombre>.rs` respetando las 4 subpestañas pedagógicas y registrándola en `src/pipelines/mod.rs`.
3. **Reglas de diseño de UI/UX:**
   - Cero emojis en la interfaz de usuario: utiliza Google Material Symbols vía `egui_icons` o trazados vectoriales.
   - Sigue Material Design 3 y Apple HIG con soporte responsivo estricto tanto en escritorio ($\ge 720\text{ px}$) como en móviles ($< 720\text{ px}$).
4. **Verificación:** Ejecuta `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test` antes de enviar cualquier cambio.
