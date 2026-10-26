//! # Signals SDR: Generador Didáctico de Señales de Radio y Satélites
//!
//! **Signals SDR** es una suite de laboratorio interactivo y entorno de experimentación
//! para la generación, modulación, degradación de canal y transmisión de señales de
//! radiofrecuencia (RF) y satelitales en tiempo real, desarrollada en Rust con [`egui`] y [`eframe`].
//!
//! ## Misión y Visión Pedagógica
//! El objetivo central de Signals SDR es proporcionar una plataforma docente para estudiantes
//! universitarios, institutos de telecomunicaciones y comunidades de radioaficionados. Permite
//! sintetizar señales **idénticas** a las emitidas por satélites orbitales reales y estaciones terrestres,
//! facilitando el aprendizaje en demodulación y decodificación con receptores SDR (HackRF, bladeRF,
//! USRP, RTL-SDR), software de recepción (SDR#, SDR++, GQRX, WXtoImg, SatDump, QSSTV, GNU Radio) o
//! algoritmos propios de procesamiento digital de señales (DSP).
//!
//! ---
//!
//! ## Metodología Didáctica de 4 Fases por Pipeline
//! Cada transmisor en Signals SDR sigue un ciclo transparente de 4 etapas:
//!
//! 1. **1. Elegir fuente:** Carga de imágenes locales (PNG, JPG, BMP, WebP), descarga remota por URL,
//!    audios (WAV, MP3) o archivos I/Q pregrabados.
//! 2. **2. Codificar / Modular:** Desglose visual interactivo de cada algoritmo: conversión de color,
//!    empaquetado de tramas, sincronismos, corrección de errores (FEC Reed-Solomon / Convolucional),
//!    subportadoras y síntesis analógica o digital I/Q en banda base.
//! 3. **3. Efectos de canal:** Simulación en segundo plano de degradaciones físicas del medio de propagación:
//!    ruido térmico AWGN con SNR regulable en dB, desplazamiento Doppler orbital $\pm 3\text{ kHz}$,
//!    desvanecimiento multitrayecto (fading Rayleigh / Rician) y pérdida por trayectoria (FSPL).
//! 4. **4. Transmisión:** Emisión física con transceptores SDR mediante [`crate::sdr`], transmisión por red
//!    hacia clientes remotos mediante el servidor didáctico RTL-TCP o streaming UDP hacia GNU Radio,
//!    o exportación a múltiples formatos de archivo I/Q normalizados.
//!
//! ---
//!
//! ## Catálogo de Transmisores y Protocolos
//!
//! | Pipeline | Banda / Frecuencia | Modulación Base | Algoritmos y Estándares Clave |
//! | :--- | :--- | :--- | :--- |
//! | **NOAA APT** | VHF (137.100 / 137.9125 MHz) | AM subportadora 2400 Hz en FM (17 kHz desv.) | Trama analógica APT, 2 canales (visible/IR), marcas de minuto y telemetría. |
//! | **METEOR LRPT** | VHF (137.100 / 137.900 MHz) | OQPSK digital a 72 ksps | Compresión JPEG DCT $8\times 8$, paquetes CCSDS Space Packet, VCDU, Reed-Solomon (255,223), Scrambler y FEC convolucional ($k=7, r=1/2$). |
//! | **ISS SSTV** | VHF (145.800 MHz) | FM estrecha (5 kHz desv.) a 288 kHz | Modos oficiales conmemorativos de la Estación Espacial Internacional: Robot 36, PD 120 y Martin 1. |
//! | **Generic SSTV** | HF (14.230 MHz) / VHF | FM analógica con tonos VIS | 11 modos de radioaficionados: Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 120/180/290 y Monocromo BW 8/12. |
//! | **Generic FM** | Broadcast / VHF / UHF | Frecuencia Modulada (WFM / NFM) | Pre-énfasis analógico (50 µs Europa / 75 µs EE.UU. / plano), generador de tono 1 kHz, audio WAV/MP3 y traslación IF. |
//! | **Generic AM** | HF / Banda Aérea (118.1 MHz) | Amplitud Modulada (DSB / SSB) | DSB-FC con índice $m$, DSB-SC y SSB (USB / LSB) sintetizada mediante transformada analítica de Hilbert. |
//! | **Generic IQ** | Configurable | Reproducción I/Q directa | Carga de grabaciones I/Q preexistentes, análisis espectral FFT, corrección de offset DC y rotación de portadora. |
//! | **NOAA HRPT** | Banda L (1698.0 / 1707.0 MHz) | BPSK digital a 665.4 kbps | Codificación Split-Phase-L (Manchester) para telemetría directa de alta resolución. |
//!
//! ---
//!
//! ## Destinos de Transmisión y Salidas
//! Signals SDR proporciona cinco destinos intercambiables en tiempo real:
//!
//! - **Hardware SDR (Transceptores Físicos):** Integración nativa con HackRF One, Nuand bladeRF,
//!   Ettus Research USRP y dispositivos compatibles con SoapySDR mediante la capa de abstracción [`crate::sdr::SdrTransmissionController`].
//! - **Servidor RTL-TCP Didáctico:** Emulación de un dongle RTL-SDR en `127.0.0.1:1234` con cabecera `RTL0`,
//!   emulación de los 29 pasos de ganancia del sintonizador R820T (comandos `0x0D` y `0x04`), atenuación
//!   logarítmica en dB, dither continuo de ADC a -75 dBFS y 15 tasas estándar de muestreo compatibles con SDR# y SDR++.
//! - **Streaming GNU Radio de Red (UDP):** Transmisión continua de tramas I/Q hacia grafos de flujo en GNU Radio
//!   Companion en formatos `Float32`, `Complex32` o `Int16`.
//! - **Instrumento Virtual de Laboratorio:** Panel integrado en el área de trabajo con analizador de espectro FFT,
//!   cascada espectral temporal (waterfall), osciloscopio I/Q en cuadratura, diagrama de constelación I/Q y telemetría de RF.
//! - **Exportación e Importación de Archivos I/Q:** WAV I/Q PCM 16-bit estéreo, RAW SC16_Q11 (bladeRF-cli),
//!   RAW SC16 completo, RAW int8 con signo (hackrf_transfer), RAW float32 (GNU Radio / inspectrum) y WAV audio mono.
//!
//! ---
//!
//! ## Organización Modular del Crate
//!
//! - [`app`]: Punto de entrada de la aplicación gráfica [`SignalsSdrApp`], gestión de layout y pestañas.
//! - [`channel`]: Modelado físico de canal de propagación de RF (AWGN, Doppler satelital, Fading multitrayecto).
//! - [`config`]: Persistencia de preferencias del usuario y organización automática de subcarpetas en disco.
//! - [`export`]: Convertidores y gestores de formatos de archivos I/Q estándar.
//! - [`file_dialog`]: Abstracción multiplataforma para apertura y guardado de archivos (nativo y WebAssembly).
//! - [`icons`]: Trazados vectoriales auxiliares e integración con Google Material Symbols.
//! - [`pipelines`]: Módulos independientes de síntesis y modulación para cada satélite o estándar de radio.
//! - [`sdr`]: Controlador de hardware SDR mediante Seify, servidor RTL-TCP y streaming de red UDP.
//! - [`theme`]: Sistema de diseño y paleta tonal dinámica Material Design 3 (temas claro y oscuro).
//! - [`ui`]: Componentes visuales, controles de laboratorio, espectrograma, osciloscopio y reproductor de audio.
//! - [`utils`]: Utilidades matemáticas de DSP, remuestreo polifásico y manipulación de buffers de señal.

pub mod app;
pub mod channel;
pub mod config;
pub mod export;
pub mod file_dialog;
pub mod icons;
pub mod pipelines;
pub mod sdr;
pub mod theme;
pub mod ui;
pub mod utils;
pub mod web_utils;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

pub use app::SignalsSdrApp;
