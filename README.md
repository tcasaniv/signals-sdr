# Signals SDR

[![Licencia: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Versión](https://img.shields.io/badge/version-0.4.4-green.svg)](Cargo.toml)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![UI: Material 3 / HIG](https://img.shields.io/badge/UI-Material_3_%2F_Apple_HIG-purple.svg)](docs/ARCHITECTURE.md)
[![Tests: 32 passed](https://img.shields.io/badge/tests-32%20passed-brightgreen.svg)](tests/)

**Signals SDR** es una suite y entorno gráfico de generación, modulación, simulación de canal físico y transmisión de señales de radiofrecuencia (RF) y satelitales en tiempo real, desarrollada en Rust con `egui` y `eframe`.

El objetivo central de Signals SDR es proporcionar una plataforma de entrenamiento, laboratorio interactivo y docencia para estudiantes universitarios, institutos técnicos y comunidades de radioaficionados. Permite generar señales **idénticas** a las que emiten satélites orbitales reales y emisoras de radio, facilitando el aprendizaje en recepción, demodulación y decodificación con receptores SDR (HackRF, bladeRF, USRP, RTL-SDR), software especializado (SDR#, SDR++, GQRX, WXtoImg, SatDump, QSSTV, GNU Radio) o código propio de DSP.

---

## Características Principales

- **Diseño Adaptativo y Responsivo (Google Material 3 + Apple HIG):**
  - **Escritorio ($\ge$ 720 px):** *Navigation Rail* lateral estrecho (72 px) con píldoras M3, selector de tema claro/oscuro, panel de catálogo deslizable y redimensionable libremente (220 a 450 px) con buscador en tiempo real y chips de categoría (`Todos`, `Satélites`, `SSTV`, `Analógicas`, `I/Q DSP`). Tarjetas de ajustes a ancho completo con chevrons vectoriales.
  - **Móvil ($<$ 720 px):** Barra superior táctil con navegación asistida *Master-Detail* y barra inferior (*Bottom Bar*) con indicadores de píldora activa.
  - **Iconografía 100% Vectorial:** Ausencia absoluta de emojis en la interfaz gráfica; empleo exclusivo de Google Material Symbols & Icons (`egui_icons` y trazados vectoriales limpios).

- **Flujo Didáctico Transparente de 4 Fases por Pipeline:**
  1. **1. Elegir fuente:** Carga de imágenes locales (PNG, JPG, BMP, WebP), descarga directa por URL, audios (WAV, MP3) o archivos I/Q.
  2. **2. Codificar / Modular:** Desglose transparente paso a paso de cada algoritmo de codificación de trama, empaquetado, corrección de errores (FEC Reed-Solomon / Convolucional), subportadoras y síntesis analógica o digital I/Q.
  3. **3. Efectos de canal:** Simulación física en segundo plano (hilo del SO `std::thread::spawn` sin bloquear la UI) de degradaciones de RF: AWGN con control de SNR en dB, desplazamiento Doppler orbital $\pm 3\text{ kHz}$, desvanecimiento multicamino (Rayleigh / Rician), pérdidas por trayectoria (FSPL) y márgenes de ruido (*Noise Padding*), preservando el 100% de las muestras generadas sin recortes.
  4. **4. Transmisión:** Emisión interactiva hacia transceptores SDR físicos, servidor RTL-TCP emulado, streaming UDP hacia GNU Radio o exportación a archivos I/Q normalizados.

- **Servidor RTL-TCP Didáctico Integrado:**
  - Emulación completa de un dongle RTL-SDR en `127.0.0.1:1234` con cabecera oficial `RTL0` (sintonizador tipo 5 Rafael Micro R820T).
  - Compatible de inmediato con **SDR#**, **SDR++**, **GQRX**, **WebSDR** y **GNU Radio**.
  - Soporte de comandos de control de ganancia: indexada `0x0D` (29 pasos de ganancia oficiales del chip R820T de 0.0 dB a 49.6 dB) y ganancia directa `0x04` en décimas de dB.
  - Modelo físico de ganancia logarítmica ($10^{(\text{gain}-38)/20}$) con punto óptimo a 38 dB y piso de ruido de dither continuo del ADC RTL2832U a **~ -75 dBFS** (eliminando el espectro plano artificial a -200 dBFS).
  - **15 tasas de muestreo estándar** con cobertura total de los selectores de SDR# y SDR++ (`0.25 MSps` a `3.2 MSps`) y sincronización dinámica de pacing según la tasa configurada por el cliente.

- **Instrumento Virtual de Laboratorio (Panel Flexible Integrado):**
  - Renderizado directamente en el área de trabajo derecha (`CentralPanel`) sin modales flotantes invasivos que tapen los menús, con botón de cierre `[X]` para retornar al transmisor.
  - **Espectro RF (FFT):** Analizador de magnitud en dB con ventana configurable y retención de picos máximos y mínimos.
  - **Cascada Espectral (Waterfall):** Representación 2D tiempo-frecuencia en falso color con 3 resoluciones de columna.
  - **Dominio del Tiempo:** Osciloscopio en cuadratura para inspeccionar las señales $I(t)$, $Q(t)$ y su envolvente $|IQ|$.
  - **Constelación I/Q:** Diagrama de plano complejo para analizar constelaciones digitales (OQPSK, BPSK, etc.).
  - **Telemetría y Métricas:** Cálculo en vivo de potencia pico (dBFS), potencia RMS, factor de cresta (PAPR), desbalance DC y aislamiento de portadora.

- **Streaming de Red GNU Radio (UDP):**
  - Envío continuo de paquetes I/Q a través de sockets UDP hacia bloques *UDP Source* en GNU Radio Companion.
  - Formatos seleccionables: `Float32` (estándar GNU Radio), `Complex32` e `Int16`.

- **Soporte de Hardware SDR mediante Seify:**
  - Selector jerárquico de 2 etapas (tipo de hardware y número de serie / IP) para HackRF One, Nuand bladeRF, Ettus Research USRP, SoapySDR o transceptor virtual Dummy.
  - Controles contextuales de ganancia TX, interruptor de amplificador RF (+14 dB en HackRF) y tasas de muestreo adaptadas.

- **Múltiples Formatos de Exportación e Importación I/Q:**
  - WAV I/Q (PCM 16 bits estéreo compatible con SDR#, GQRX, WebSDR).
  - RAW SC16_Q11 (compatible con bladeRF-cli).
  - RAW SC16 Completo ($\pm 32767$).
  - RAW con signo int8 (compatible con hackrf_transfer).
  - RAW float32 (compatible con GNU Radio, inspectrum y SigMF).
  - WAV Audio Mono analógico.

- **Organización Automática de Carpetas:**
  - Configuración persistente de una ruta raíz en Ajustes con creación automática de subcarpetas temáticas (`noaa_apt/`, `meteor_lrpt/`, `iss_sstv/`, `generic_sstv/`, `generic_fm/`, `generic_am/`, `generic_iq/`).

---

## Canales y Protocolos Disponibles

### 1. Satélites Meteorológicos NOAA APT
- **Fuente:** 1 o 2 imágenes (Canal A visible, Canal B infrarrojo) o descarga por URL.
- **Codificación:** Preprocesamiento a 909 px de ancho $\to$ Trama APT (2080 px/línea con sincro A/B, marcas de minuto y telemetría) $\to$ Subportadora AM a 2400 Hz $\to$ Modulación FM I/Q (17 kHz de desviación).
- **Transmisión:** 137.100 / 137.9125 MHz, servidor RTL-TCP o guardado I/Q.

### 2. Satélites Meteorológicos METEOR LRPT
- **Fuente:** 1 a 3 imágenes (Ch1, Ch2, Ch3) o descomposición RGB.
- **Codificación:** Compresión JPEG DCT $8\times 8$ $\to$ Empaquetado CCSDS Space Packet y VCDU $\to$ Reed-Solomon (255,223) $\to$ Scrambler CCSDS $\to$ Convolucional FEC ($k=7, r=1/2$) $\to$ Modulación digital OQPSK a 72 ksps.
- **Transmisión:** 137.100 / 137.900 MHz, servidor RTL-TCP o guardado I/Q.

### 3. ISS SSTV (Estación Espacial Internacional)
- **Fuente:** Imagen conmemorativa o de misión.
- **Codificación:** Modos oficiales de la ISS (Robot 36, PD 120 o Martin 1) con modulación en frecuencia estrecha (FM 5 kHz) a 288 kHz.
- **Transmisión:** 145.800 MHz FM, servidor RTL-TCP o guardado I/Q.

### 4. Generic SSTV (Multiprotocolo)
- **Fuente:** Selección de imagen fuente.
- **Codificación:** Soporte completo de 11 modos de radioaficionados: Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 120/180/290, Monocromo BW 8/12 con síntesis analógica de tonos VIS.
- **Transmisión:** Frecuencias de llamada (ej. 14.230 MHz o 145.800 MHz), servidor RTL-TCP o archivo I/Q.

### 5. Generic FM (Frecuencia Modulada)
- **Fuente:** Carga de audio (WAV, MP3) con reproductor integrado o generador de tono de 1 kHz.
- **Modulación:** Presets Broadcast WFM (75 kHz), VHF/UHF NFM (5 kHz), NOAA APT (17 kHz); filtro de pre-énfasis analógico (50 µs Europa / 75 µs EE.UU. / plano); y desplazamiento de frecuencia intermedia IF.
- **Transmisión:** Transmisión SDR, servidor RTL-TCP o exportación I/Q.

### 6. Generic AM (Amplitud Modulada)
- **Fuente:** Carga de audio o tono sintético.
- **Modulación:** Modos DSB-FC (Portadora Completa), DSB-SC (Portadora Suprimida), SSB USB (Banda Lateral Superior) y SSB LSB (Banda Lateral Inferior) mediante transformada analítica de Hilbert.
- **Transmisión:** Banda aérea VHF (118.1 MHz), HF, servidor RTL-TCP o guardado I/Q.

### 7. Generic IQ (Gestor y Retransmisor I/Q)
- **Fuente:** Archivos I/Q pregrabados (`.wav`, `.raw`, `.iq`, `.sc16`, `.bin`).
- **Procesamiento:** Analizador de espectro FFT, osciloscopio, ganancia digital, rotación de portadora Doppler y cancelación de DC.
- **Transmisión:** Retransmisión inmediata con hardware SDR, servidor RTL-TCP o transcodificación de formato.

### 8. NOAA HRPT (Didáctico)
- Módulo educativo en banda L (1.7 GHz) con generador de tramas BPSK Manchester Split-Phase-L a 665.4 kbps (consulte [ROADMAP.md](ROADMAP.md)).

---

## Guía de Conexión con Clientes SDR (RTL-TCP)

Signals SDR permite emitir directamente hacia programas receptores SDR mediante su servidor RTL-TCP integrado:

1. En Signals SDR, diríjase a la pestaña **4. Transmisión** de cualquier transmisor.
2. Seleccione el destino **RTL-TCP** e inicie el servidor en el puerto predeterminado `1234`.
3. Abra su software receptor preferido y configure el origen de señal:

### Conexión desde SDR# (AIRSPY Studio)
- En el menú lateral izquierdo, seleccione como **Source:** `RTL-SDR TCP`.
- En la rueda de ajustes del dispositivo, configure:
  - **Host:** `127.0.0.1` (o la IP de su red local si transmite desde otra PC).
  - **Port:** `1234`.
  - **Sample Rate:** Seleccione cualquiera de las 10 tasas compatibles (ej. `0.25 MSPS`, `1.024 MSPS` o `2.048 MSPS`).
- Ajuste el control deslizante **RF Gain** para comprobar cómo la señal emerge sobre el piso de ruido térmico.

### Conexión desde SDR++
- En la sección **Source**, seleccione `RTL-TCP`.
- Configure la dirección `localhost` (o IP LAN) y el puerto `1234`.
- Seleccione la tasa de muestreo (ej. `250KHz`, `1.024MHz`, `1.536MHz`, `2.048MHz`, etc.).
- Presione el botón **Play**.

---

## Estructura del Repositorio

```
signals-sdr/
├── Cargo.toml               # Configuración del paquete y dependencias (v0.4.4)
├── CHANGELOG.md             # Registro de versiones según Keep a Changelog
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
│   ├── lib.rs               # Documentación y exportaciones de biblioteca
│   ├── main.rs              # Punto de entrada nativo y WebAssembly
│   ├── theme.rs             # Sistema de color tonal Material 3 (Claro / Oscuro)
│   ├── channel/             # Fase 3: AWGN, Doppler, Fading y estado asíncrono
│   │   ├── effects.rs
│   │   └── mod.rs
│   ├── export/              # Conversores y exportadores de formatos I/Q
│   │   ├── formats.rs
│   │   └── mod.rs
│   ├── pipelines/           # Módulos didácticos de satélites y moduladores
│   ├── sdr/                 # Controlador Seify, servidor RTL-TCP y streaming UDP
│   │   ├── network_tx.rs    # Servidor RTL-TCP y streaming GNU Radio UDP
│   │   ├── worker.rs        # Controlador asíncrono de hardware SDR
│   │   └── mod.rs
│   └── ui/                  # Componentes M3, Instrumento Virtual, espectrograma y audio
│       ├── channel_effects.rs
│       ├── transmission.rs
│       ├── virtual_sink.rs  # Instrumento Virtual de Laboratorio integrado
│       └── mod.rs
└── tests/                   # 32 pruebas automatizadas unitarias y de integración
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

### Modalidad 2: Servidor Web LAN (`--serve`)
Permite ejecutar el binario nativo en una PC anfitriona o Raspberry Pi conectada físicamente a los SDRs y exponer la interfaz web en la red local (Wi-Fi), controlando la transmisión inalámbricamente desde cualquier celular (Android/iOS), tablet (iPad) o laptop:

```bash
# Iniciar servidor web en el puerto predeterminado (8080)
cargo run --release -- --serve

# O con opciones avanzadas de puerto y dirección de red:
cargo run --release -- --serve --port 8080 --bind 0.0.0.0 --open
```

Al iniciarse, el servidor imprime las direcciones de conexión directa:
- **Acceso Local:** `http://localhost:8080`
- **Acceso en Red LAN:** `http://192.168.1.X:8080` (desde tu teléfono o tablet)

### Modalidad 3: WebAssembly Estático (GitHub Pages / Dominio Personalizado)
Compila la suite como una aplicación web estática autónoma que se ejecuta al 100% en el navegador del usuario a través de WebAssembly (WASM) y Canvas:

```bash
# Instalar trunk (empaquetador web de Rust)
cargo install trunk

# Compilar release estática WebAssembly para producción (salida en carpeta dist/)
trunk build --release --no-default-features

# O probar la web localmente con servidor de desarrollo y auto-recarga (Hot-Reload)
trunk serve --no-default-features
```

### Generación de Documentación Técnica con Rustdoc (`cargo doc`)
Signals SDR cuenta con una suite completa de documentación enriquecida personalizada en Rustdoc:
```bash
# Generar y abrir la documentación web local del proyecto
cargo doc --no-deps --open
```

### Flujo de Desarrollo Diario
```bash
# Chequeo rápido de sintaxis y tipos
cargo check

# Ejecución de la suite completa de pruebas automáticas (32 pruebas)
cargo test

# Linter de calidad de código y buenas prácticas
cargo clippy

# Formato automático del código
cargo fmt
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
