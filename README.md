# Signals SDR

[![Licencia: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Versión](https://img.shields.io/badge/version-0.2.1-green.svg)](Cargo.toml)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![UI: Material 3 / HIG](https://img.shields.io/badge/UI-Material_3_%2F_Apple_HIG-purple.svg)](docs/ARCHITECTURE.md)

**Signals SDR** es una suite y entorno gráfico de generación, modulación y transmisión de señales de radiofrecuencia (RF) y satelitales, desarrollada en Rust con `egui` y `eframe`.

El objetivo central de Signals SDR es proporcionar una plataforma de entrenamiento, laboratorio interactivo y docencia para estudiantes universitarios, institutos técnicos y comunidades de radioaficionados. Permite generar señales idénticas a las que emiten satélites reales y emisoras de radio, facilitando el aprendizaje en recepción, demodulación y decodificación con receptores SDR (HackRF, bladeRF, USRP, RTL-SDR), software especializado (SDR#, GQRX, WXtoImg, SatDump, QSSTV, GNU Radio) o código propio.

---

## Características Principales

- **Diseño Adaptativo y Responsivo (Material 3 + Apple HIG):**
  - **Escritorio ($\ge$ 720 px):** Navigation Rail lateral estrecho, selector de modo claro/oscuro, panel de catálogo deslizable y redimensionable libremente por el usuario con barra de búsqueda instantánea y chips de categoría (`Todos`, `Satélites`, `SSTV`, `Analógicas`, `I/Q DSP`).
  - **Móvil ($<$ 720 px):** Barra superior táctil con botón de retorno en flujo *Master-Detail* y barra de navegación inferior con indicadores de píldora M3.
  - **Iconografía Vectorial:** Ausencia total de emojis en la interfaz de usuario; uso exclusivo de Google Material Symbols & Icons (`egui_icons`).

- **Flujo Didáctico Transparente de 4 Fases por Pipeline:**
  1. **1. Elegir fuente:** Carga de imágenes locales (PNG, JPG, BMP, WebP), descarga directa por URL, audios o archivos I/Q.
  2. **2. Codificar / Modular:** Desglose transparente paso a paso de cada algoritmo de codificación de trama y síntesis de modulación.
  3. **3. Efectos de canal:** Simulación realista de degradaciones de RF (AWGN con control de SNR en dB, desplazamiento Doppler satelital $\pm 3\text{ kHz}$ y fading Rayleigh/Rician).
  4. **4. Transmisión:** Transmisión con transceptores SDR físicos o guardado en archivo I/Q.

- **Instrumentación de Laboratorio en Tiempo Real:**
  - **Analizador de Espectro FFT:** Gráficas de magnitud en dB con ventana configurable.
  - **Cascada Espectral (Waterfall):** Representación bidimensional tiempo-frecuencia con mapas de color dinámicos.
  - **Osciloscopio Interactivo:** Inspección de señal en el dominio temporal con zoom y exportación.
  - **Reproductor de Audio Integrado:** Monitoreo acústico inmediato de las señales generadas (FM, AM, subportadoras y tonos SSTV).

- **Soporte de Hardware SDR mediante Seify:**
  - **Selector Jerárquico de 2 Etapas:** Selección de tipo de hardware (HackRF One, Nuand bladeRF, Ettus USRP, SoapySDR o Simulación Dummy) y selector de instancia por número de serie / dirección IP.
  - Controles contextuales de ganancia TX, interruptor de amplificador RF (+14 dB en HackRF) y tasas de muestreo adaptadas.

- **Múltiples Formatos de Exportación e Importación I/Q:**
  - WAV I/Q (16 bits PCM estéreo compatible con SDR#, GQRX, WebSDR).
  - RAW SC16_Q11 (compatible con bladeRF-cli).
  - RAW SC16 Completo.
  - RAW con signo int8 (compatible con hackrf_transfer).
  - RAW float32 (compatible con GNU Radio e inspectrum).
  - WAV Audio Mono analógico.

- **Organización Automática de Carpetas:**
  - Configuración persistente de una ruta raíz en Ajustes con creación automática de subcarpetas temáticas (`noaa_apt/`, `meteor_lrpt/`, `iss_sstv/`, `generic_sstv/`, `generic_fm/`, `generic_am/`, `generic_iq/`).

---

## Canales y Protocolos Disponibles

### 1. Satélites Meteorológicos NOAA APT
- **Fuente:** 1 o 2 imágenes (Canal A visible, Canal B infrarrojo) o descarga por URL.
- **Codificación:** Preprocesamiento a 909 px de ancho $\to$ Trama APT (2080 px/línea con sincro A/B, marcas de minuto y telemetría) $\to$ Subportadora AM a 2400 Hz $\to$ Modulación FM I/Q (17 kHz de desviación).
- **Transmisión:** 137.100 / 137.9125 MHz o guardado I/Q.

### 2. Satélites Meteorológicos METEOR LRPT
- **Fuente:** 1 a 3 imágenes (Ch1, Ch2, Ch3) o descomposición RGB.
- **Codificación:** Compresión JPEG DCT $8\times 8$ $\to$ Empaquetado CCSDS Space Packet y VCDU $\to$ Reed-Solomon (255,223) $\to$ Scrambler CCSDS $\to$ Convolucional FEC ($k=7, r=1/2$) $\to$ Modulación digital OQPSK a 72 ksps.
- **Transmisión:** 137.100 / 137.900 MHz o guardado I/Q.

### 3. ISS SSTV (Estación Espacial Internacional)
- **Fuente:** Imagen conmemorativa o de misión.
- **Codificación:** Modos oficiales de la ISS (Robot 36, PD 120 o Martin 1) con modulación en frecuencia estrecha (FM 5 kHz) a 288 kHz.
- **Transmisión:** 145.800 MHz FM o guardado I/Q.

### 4. Generic SSTV (Multiprotocolo)
- **Fuente:** Selección de imagen fuente.
- **Codificación:** Soporte completo de 11 modos de radioaficionados: Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 120/180/290, Monocromo BW 8/12 con síntesis analógica de tonos VIS.
- **Transmisión:** Frecuencias de llamada (ej. 14.230 MHz o 145.800 MHz) o archivo I/Q.

### 5. Generic FM (Frecuencia Modulada)
- **Fuente:** Carga de audio (WAV, MP3) con reproductor integrado o generador de tono de 1 kHz.
- **Modulación:** Presets Broadcast WFM (75 kHz), VHF/UHF NFM (5 kHz), NOAA APT (17 kHz); filtro de pre-énfasis analógico (50 µs Europa / 75 µs EE.UU. / plano); y desplazamiento de frecuencia intermedia IF.
- **Transmisión:** Transmisión SDR o exportación I/Q.

### 6. Generic AM (Amplitud Modulada)
- **Fuente:** Carga de audio o tono sintético.
- **Modulación:** Modos DSB-FC (Portadora Completa), DSB-SC (Portadora Suprimida), SSB USB (Banda Lateral Superior) y SSB LSB (Banda Lateral Inferior) mediante transformada analítica de Hilbert.
- **Transmisión:** Banda aérea VHF (118.1 MHz), HF o guardado I/Q.

### 7. Generic IQ (Gestor y Retransmisor I/Q)
- **Fuente:** Archivos I/Q pregrabados (`.wav`, `.raw`, `.iq`, `.sc16`, `.bin`).
- **Procesamiento:** Analizador de espectro FFT, osciloscopio, ganancia digital, rotación de portadora Doppler y cancelación de DC.
- **Transmisión:** Retransmisión inmediata con hardware SDR o transcodificación de formato.

### 8. NOAA HRPT (Didáctico)
- Módulo educativo en banda L (1.7 GHz) con generador de tramas BPSK Manchester Split-Phase-L a 665.4 kbps (consulte [ROADMAP.md](ROADMAP.md)).

---

## Estructura del Repositorio

```
signals-sdr/
├── Cargo.toml               # Configuración del paquete y dependencias (v0.2.1)
├── CONTRIBUTING.md          # Guía de contribución y política de responsabilidad humana en IA
├── CODE_OF_CONDUCT.md       # Código de conducta Contributor Covenant v2.1
├── LICENSE                  # Licencia GNU General Public License v3.0
├── README.md                # Presentación general y guía rápida
├── ROADMAP.md               # Hoja de ruta y estado de fases del proyecto
├── docs/
│   └── ARCHITECTURE.md      # Arquitectura modular y modalidades de despliegue
├── assets/                  # Iconos multiplataforma y logos oficiales
├── src/
│   ├── app.rs               # Implementación de eframe::App y responsividad M3
│   ├── config.rs            # Configuración persistente y subcarpetas temáticas
│   ├── icons.rs             # Trazados vectoriales auxiliares
│   ├── lib.rs               # Exportaciones de biblioteca
│   ├── main.rs              # Punto de entrada nativo y WebAssembly
│   ├── theme.rs             # Sistema de color tonal Material 3 (Claro / Oscuro)
│   ├── channel/
│   │   ├── effects.rs       # Fase 3: AWGN, Doppler y Fading
│   │   └── mod.rs
│   ├── export/
│   │   ├── formats.rs       # Convertidores y exportadores de formatos I/Q
│   │   └── mod.rs
│   ├── pipelines/           # Módulos didácticos de satélites y moduladores
│   ├── sdr/                 # Controlador asíncrono Seify (HackRF, bladeRF, USRP, Dummy)
│   └── ui/                  # Componentes M3, cascada espectral, osciloscopio y audio
└── tests/                   # Pruebas automatizadas unitarias y de integración
```

---

## Compilación y Ejecución

### Requisitos Previos
- **Rust:** Versión 1.80 o superior (Toolchain estable).
- **Cargo.**

### Modalidad 1: Escritorio Nativo (Predeterminada)
Por defecto compila en modo de simulación Dummy (no requiere hardware físico conectado):
```bash
cargo run --release
```

#### Habilitar Soporte para Hardware SDR Específico
Para compilar con soporte para transceptores conectados por USB:
```bash
# Para HackRF One:
cargo run --release --features hackrf

# Para Nuand bladeRF:
cargo run --release --features bladerf

# Para Ettus Research USRP:
cargo run --release --features usrp
```

### Modalidad 2: Servidor Web LAN (`--serve`) *(Próximamente en v0.3.0)*
Permite ejecutar el programa en una PC anfitriona o Raspberry Pi conectada a los SDRs y controlarla de forma inalámbrica desde el navegador de cualquier celular, tablet o laptop en la misma red Wi-Fi:
```bash
signals-sdr --serve --port 8080 --bind 0.0.0.0
```

### Modalidad 3: WebAssembly Estático *(Próximamente en v0.3.0)*
Despliegue estático sin servidor en GitHub Pages con dominio personalizado, permitiendo generar señales, inspeccionar espectrogramas y descargar archivos I/Q directamente desde el navegador web. Consulte [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para más detalles técnicos.

### Ejecución de Pruebas Automatizadas
```bash
cargo test
```

---

## Contribución y Comunidad Open Source

¡Las contribuciones de la comunidad son bienvenidas!

- **Política de Inteligencia Artificial:** Si utilizas asistentes de IA (ChatGPT, Claude, Gemini, Copilot, etc.) para programar, ten en cuenta que **el contribuidor humano que firma el Pull Request es el responsable legal y técnico exclusivo de cada línea de código introducida**.
- Consulta nuestra [Guía de Contribución (CONTRIBUTING.md)](CONTRIBUTING.md) para detalles sobre el estilo de código, convenciones de commits y estándares de calidad.
- Consulta nuestro [Código de Conducta (CODE_OF_CONDUCT.md)](CODE_OF_CONDUCT.md).

---

## Licencia y Uso Responsable

Este proyecto se distribuye bajo la licencia **GNU General Public License v3.0 (GPL-3.0-or-later)**. Consulte el archivo [LICENSE](LICENSE) para más información.

> [!CAUTION]
> **Aviso Legal:** Signals SDR ha sido desarrollado exclusivamente con fines educativos, de laboratorio y docencia técnica. Al transmitir con dispositivos de radio definida por software (SDR), asegúrese de cumplir estrictamente con las regulaciones de telecomunicaciones vigentes en su jurisdicción y de utilizar cargas ficticias (*dummy loads*) para no radiar señales sobre el espectro electromagnético sin la debida licencia de radioaficionado o autorización legal.
