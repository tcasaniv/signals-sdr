# Signals SDR

Signals SDR es una suite y entorno gráfico de generación, codificación y modulación de señales de radiofrecuencia (RF) y satelitales, desarrollada en Rust con egui y eframe.

El objetivo pedagógico central de Signals SDR es permitir a docentes, estudiantes universitarios y radioaficionados generar señales idénticas a las que emiten satélites reales y emisoras de radio, facilitando el entrenamiento y aprendizaje en la recepción, demodulación y decodificación con receptores SDR, software existente (SDR#, GQRX, WXtoImg, SatDump, QSSTV) o código propio.

---

## Características Principales

- **Multiplataforma y Responsivo:**
  - Ejecutable nativo para escritorio (Windows, Linux, macOS).
  - Soporte para compilación en WebAssembly (WASM) para navegadores web y dispositivos móviles.
  - Interfaz adaptativa: diseño tipo riel lateral en pantallas de escritorio (>= 720 px) y diseño táctil con barra inferior en dispositivos móviles (< 720 px).
  - Modo claro y oscuro intercambiable.

- **Soporte de Hardware SDR mediante Seify:**
  - Compatible con HackRF One (control de ganancia y amplificador RF +14 dB).
  - Compatible con Nuand bladeRF (formato nativo escalado SC16_Q11).
  - Compatible con Ettus USRP.
  - Dispositivo de simulación Dummy integrado para pruebas sin hardware conectado.

- **Múltiples Formatos de Exportación e Importación I/Q:**
  - WAV I/Q (16 bits PCM estéreo compatible con SDR#, GQRX, WebSDR).
  - RAW SC16_Q11 (compatible con bladeRF-cli).
  - RAW SC16 Completo.
  - RAW con signo int8 (compatible con hackrf_transfer).
  - RAW float32 (compatible con GNU Radio e inspectrum).
  - WAV Audio Mono analógico.

- **Organización Automática de Carpetas:**
  - Configuración persistente de una ruta raíz en Ajustes.
  - Creación automática de subcarpetas temáticas (`noaa_apt/`, `meteor_lrpt/`, `iss_sstv/`, `generic_sstv/`, `generic_fm/`, `generic_am/`, `generic_iq/`).

---

## Canales y Protocolos Disponibles

### 1. Satélites Meteorológicos NOAA APT
- **Subpestaña 1 (Elegir fuente):** Carga de dos imágenes (Canal A visible, Canal B infrarrojo) o descarga directa por URL.
- **Subpestaña 2 (Codificar):** 
  - Fase 1: Preprocesamiento a escala de grises, ancho de 909 píxeles y recorte a la menor altura.
  - Fase 2: Codificación de trama APT (cuadro de 2080 píxeles por línea con marcas de sincronismo, minutos y telemetría).
  - Fase 3: Modulación AM sobre subportadora de 2400 Hz (audio WAV) y posterior modulación en frecuencia (FM 17 kHz de desviación a 288 kHz de tasa de muestreo I/Q).
- **Subpestaña 3 (Transmisión):** Transmisión SDR en 137.100 / 137.9125 MHz o guardado en archivo I/Q.

### 2. Satélites Meteorológicos METEOR LRPT
- **Subpestaña 1 (Elegir fuente):** Selección de hasta 3 canales de imagen (rojo, verde, azul/infrarrojo).
- **Subpestaña 2 (Codificar):**
  - Compresión DCT JPEG de 8x8 píxeles.
  - Encapsulamiento en paquetes CCSDS Space Packet y Virtual Channel Frames.
  - Codificación convolucional FEC (k=7, r=1/2) y Reed-Solomon (255, 223).
  - Modulación digital OQPSK a 72 kbaudios.
- **Subpestaña 3 (Transmisión):** Transmisión SDR en 137.100 / 137.900 MHz o guardado en archivo I/Q.

### 3. ISS SSTV (Estación Espacial Internacional)
- **Subpestaña 1 (Elegir fuente):** Carga de imagen conmemorativa o patrón de prueba.
- **Subpestaña 2 (Codificar):** Codificación en modos oficiales de la ISS (Robot 36, PD 120 o Martin 1) con modulación en frecuencia estrecha (FM 5 kHz) a 288 kHz.
- **Subpestaña 3 (Transmisión):** Transmisión en 145.800 MHz FM o guardado en archivo I/Q.

### 4. Generic SSTV (Multiprotocolo)
- **Subpestaña 1 (Elegir fuente):** Selección de imagen fuente.
- **Subpestaña 2 (Codificar):** Selección de todos los modos reconocidos de radioaficionados:
  - Martin 1, Martin 2
  - Scottie 1, Scottie 2, Scottie DX
  - Robot 36, Robot 72
  - PD 120, PD 180, PD 290
  - Monocromo BW 8, BW 12
- **Subpestaña 3 (Transmisión):** Transmisión en frecuencias de llamada (ej. 14.230 MHz o 145.800 MHz) o archivo I/Q.

### 5. Generic FM (Modulación de Frecuencia)
- **Subpestaña 1 (Elegir fuente):** Carga de audio (WAV, MP3) con reproductor integrado o generador de tono 1 kHz.
- **Subpestaña 2 (Modular):**
  - Presets: Broadcast WFM (75 kHz), VHF/UHF NFM (5 kHz), NOAA APT (17 kHz).
  - Filtro de pre-énfasis: Desactivado, 50 microsegundos (Europa/ITU), 75 microsegundos (EE.UU./FCC).
  - Desplazamiento IF y tasa de muestreo SDR configurable (288 kHz, 1 MHz, 2 MHz).
- **Subpestaña 3 (Transmisión):** Transmisión SDR o exportación I/Q.

### 6. Generic AM (Modulación de Amplitud)
- **Subpestaña 1 (Elegir fuente):** Carga de audio o tono sintético.
- **Subpestaña 2 (Modular):**
  - Modos: DSB-FC (Portadora Completa), DSB-SC (Portadora Suprimida), SSB USB (Banda Lateral Superior) y SSB LSB (Banda Lateral Inferior) mediante transformada analítica de Hilbert.
  - Ajuste de índice de modulación y offset IF.
- **Subpestaña 3 (Transmisión):** Transmisión SDR en aviación VHF (118.1 MHz) o HF / archivo I/Q.

### 7. Generic IQ (Gestor y Transmisor de Archivos I/Q)
- **Subpestaña 1 (Elegir fuente):** Carga de archivos I/Q pregrabados en formatos estándar (.wav, .raw, .iq, .sc16, .bin).
- **Subpestaña 2 (Procesamiento):**
  - Visualización en osciloscopio y analizador de espectro FFT.
  - Ajuste de ganancia, rotación de frecuencia (shift de portadora) y cancelación de DC Offset.
- **Subpestaña 3 (Transmisión):** Transmisión inmediata con hardware SDR o conversión de formato I/Q.

### 8. NOAA HRPT (Banda L 1.7 GHz)
- Módulo educativo en desarrollo con generador de tramas BPSK Manchester Split-Phase-L a 665.4 kbps (consulte ROADMAP.md).

---

## Estructura del Proyecto

```
signals-sdr/
├── Cargo.toml
├── LICENSE
├── README.md
├── ROADMAP.md
├── src/
│   ├── app.rs               # Implementación de eframe::App y responsividad
│   ├── config.rs            # Configuración persistente y subcarpetas
│   ├── icons.rs             # Iconografía vectorial (sin emojis)
│   ├── lib.rs               # Módulos públicos de la biblioteca
│   ├── main.rs              # Punto de entrada nativo y WebAssembly
│   ├── theme.rs             # Paleta de colores claro/oscuro
│   ├── export/
│   │   ├── formats.rs       # Convertidores y exportadores de formatos I/Q
│   │   └── mod.rs
│   ├── pipelines/
│   │   ├── apt.rs           # Pipeline NOAA APT (Preproceso, Codificación, AM, FM)
│   │   ├── generic_am.rs    # Pipeline Generic AM (DSB, SSB)
│   │   ├── generic_fm.rs    # Pipeline Generic FM (WFM, NFM, Pre-énfasis)
│   │   ├── generic_iq.rs    # Pipeline Generic IQ (Inspección y re-transmisión)
│   │   ├── generic_sstv.rs  # Pipeline Generic SSTV (Martin, Scottie, Robot, PD, BW)
│   │   ├── hrpt.rs          # Pipeline didáctico NOAA HRPT
│   │   ├── iss_sstv.rs      # Pipeline ISS SSTV (Robot 36, PD 120, Martin 1)
│   │   ├── lrpt.rs          # Pipeline METEOR LRPT (CCSDS, FEC, OQPSK)
│   │   └── mod.rs
│   ├── sdr/
│   │   ├── mod.rs
│   │   └── worker.rs        # Controlador asíncrono Seify (HackRF, bladeRF, USRP, Dummy)
│   └── ui/
│       ├── audio_player.rs  # Reproductor y visor de audio
│       ├── components.rs    # Componentes visuales reutilizables
│       ├── desktop_nav.rs   # Barra lateral de escritorio (Rail + Búsqueda)
│       ├── mobile_nav.rs    # Navegación móvil (Encabezado + Barra inferior)
│       ├── mod.rs
│       ├── spectrum.rs      # Osciloscopio y analizador de espectro
│       └── transmission.rs  # Subpestaña 3 (SDR vs Archivo IQ)
└── tests/
    ├── export_tests.rs      # Pruebas de exportación e importación I/Q
    └── pipeline_tests.rs    # Pruebas de procesamiento de señales
```

---

## Compilación y Ejecución

### Requisitos
- Rust 1.80 o superior (Toolchain estable).
- Cargo.

### Ejecución en Escritorio (Modo Simulación Dummy por defecto)
```bash
cargo run --release
```

### Habilitar Soporte para Hardware SDR Específico
Para compilar con soporte para dispositivos de radiofrecuencia conectados por USB:
```bash
# Para HackRF One:
cargo run --release --features hackrf

# Para Nuand bladeRF:
cargo run --release --features bladerf

# Para Ettus Research USRP:
cargo run --release --features usrp
```

### Ejecución de Pruebas Automatizadas
```bash
cargo test
```

---

## Licencia y Uso Responsable

Este proyecto se distribuye bajo la licencia GNU General Public License v3.0 (GPL-3.0-or-later).

**Aviso Legal:** Signals SDR ha sido diseñado exclusivamente con fines formativos y de experimentación en laboratorio. Al transmitir con dispositivos de radio definida por software (SDR), asegúrese de cumplir estrictamente con las regulaciones de telecomunicaciones vigentes en su jurisdicción o de utilizar cargas ficticias (dummy loads) para no radiar señales sobre el espectro electromagnético sin la debida licencia de radioaficionado.
