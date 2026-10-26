//! # Módulos Didácticos de Satélites y Moduladores de Radio
//!
//! Este módulo contiene la implementación de cada uno de los 8 transmisores y moduladores
//! didácticos de Signals SDR. Cada pipeline es un módulo autónomo que gestiona las 4 fases
//! pedagógicas de la señal (1. Fuente, 2. Codificación/Modulación, 3. Efectos de canal y 4. Transmisión).
//!
//! ## Catálogo de Pipelines Disponibles
//!
//! - [`apt::AptPipeline`]: Transmisor satelital meteorológico NOAA APT en 137.100 / 137.9125 MHz.
//!   Convierte imágenes en tramas analógicas de 2080 píxeles por línea con marcas de sincronismo,
//!   minutero y telemetría, modulándolas sobre una subportadora AM de 2400 Hz y modulador FM de 17 kHz de desviación.
//!
//! - [`lrpt::LrptPipeline`]: Transmisor satelital meteorológico digital ruso Meteor-M LRPT en 137.100 / 137.900 MHz.
//!   Implementa compresión JPEG DCT $8\times 8$, empaquetado CCSDS (Space Packet y VCDU), codificación
//!   Reed-Solomon (255, 223), Scrambler pseudoaleatorio CCSDS, código convolucional FEC ($k=7, r=1/2$)
//!   y modulación por desplazamiento de fase en cuadratura compensada (OQPSK) a 72 ksps.
//!
//! - [`iss_sstv::IssSstvPipeline`]: Transmisor de televisión de barrido lento (SSTV) oficial de la Estación
//!   Espacial Internacional (ISS) en 145.800 MHz FM estrecha (modos conmemorativos Robot 36, PD 120 y Martin 1).
//!
//! - [`generic_sstv::GenericSstvPipeline`]: Modulador SSTV multiprotocolo para radioaficionados en bandas HF y VHF
//!   con soporte completo de 11 modos (Martin 1/2, Scottie 1/2/DX, Robot 36/72, PD 120/180/290 y BW 8/12).
//!
//! - [`generic_fm::GenericFmPipeline`]: Modulador de Frecuencia Modulada (FM) analógica de propósito general,
//!   con presets para Broadcast comercial WFM (75 kHz de desviación), comunicaciones VHF/UHF NFM (5 kHz),
//!   filtro de pre-énfasis analógico configurable (50 µs, 75 µs o plano) y reproductor de audio integrado.
//!
//! - [`generic_am::GenericAmPipeline`]: Modulador de Amplitud Modulada (AM) analógica con soporte para
//!   DSB-FC (Portadora Completa con índice $m$), DSB-SC (Portadora Suprimida) y Banda Lateral Única (SSB USB / LSB)
//!   sintetizada digitalmente mediante transformada analítica de Hilbert.
//!
//! - [`generic_iq::GenericIqPipeline`]: Procesador y retransmisor de archivos I/Q pregrabados (`.wav`, `.raw`, `.iq`, `.sc16`),
//!   con análisis espectral FFT, eliminación de offset de continua (DC spike) y rotación Doppler de portadora.
//!
//! - [`hrpt::HrptPipeline`]: Módulo didáctico de alta resolución para satélites en banda L (1.7 GHz)
//!   con modulación digital BPSK y codificación de línea Manchester Split-Phase-L a 665.4 kbps.

pub mod apt;
pub mod generic_am;
pub mod generic_fm;
pub mod generic_iq;
pub mod generic_sstv;
pub mod hrpt;
pub mod iss_sstv;
pub mod lrpt;

pub use apt::AptPipeline;
pub use generic_am::GenericAmPipeline;
pub use generic_fm::GenericFmPipeline;
pub use generic_iq::GenericIqPipeline;
pub use generic_sstv::GenericSstvPipeline;
pub use hrpt::HrptPipeline;
pub use iss_sstv::IssSstvPipeline;
pub use lrpt::LrptPipeline;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineId {
    NoaaApt,
    MeteorLrpt,
    IssSstv,
    GenericSstv,
    GenericFm,
    GenericAm,
    GenericIq,
    NoaaHrpt,
}

impl PipelineId {
    pub fn title(&self) -> &'static str {
        match self {
            PipelineId::NoaaApt => "NOAA APT",
            PipelineId::MeteorLrpt => "METEOR LRPT",
            PipelineId::IssSstv => "ISS SSTV",
            PipelineId::GenericSstv => "Generic SSTV",
            PipelineId::GenericFm => "Generic FM",
            PipelineId::GenericAm => "Generic AM",
            PipelineId::GenericIq => "Generic IQ",
            PipelineId::NoaaHrpt => "NOAA HRPT",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            PipelineId::NoaaApt | PipelineId::MeteorLrpt | PipelineId::NoaaHrpt => {
                "Satélites Meteorológicos"
            }
            PipelineId::IssSstv | PipelineId::GenericSstv => "Imágenes SSTV",
            PipelineId::GenericFm | PipelineId::GenericAm => "Moduladores Analógicos",
            PipelineId::GenericIq => "Herramientas de Señal",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PipelineId::NoaaApt => "Transmisión analógica de satélites NOAA (137 MHz, AM/FM, 2400 Hz)",
            PipelineId::MeteorLrpt => "Transmisión digital de satélites Meteor-M (137 MHz, OQPSK 72k)",
            PipelineId::IssSstv => "Imágenes SSTV de la Estación Espacial Internacional (145.8 MHz FM)",
            PipelineId::GenericSstv => "Transmisión de imágenes en modos Martin, Scottie, Robot, PD, BW",
            PipelineId::GenericFm => "Modulación de audio en FM (Broadcast WFM, VHF/UHF NFM)",
            PipelineId::GenericAm => "Modulación de audio en AM (DSB-FC comercial/aviación, DSB-SC, SSB)",
            PipelineId::GenericIq => "Carga, análisis, acondicionamiento y transmisión de archivos I/Q",
            PipelineId::NoaaHrpt => "Transmisión en banda L (1.7 GHz) de radiómetro AVHRR (BPSK 665k)",
        }
    }

    pub fn frequency_badge(&self) -> &'static str {
        match self {
            PipelineId::NoaaApt => "137.1 MHz",
            PipelineId::MeteorLrpt => "137.9 MHz",
            PipelineId::IssSstv => "145.8 MHz",
            PipelineId::GenericSstv => "HF / VHF",
            PipelineId::GenericFm => "88-108 MHz",
            PipelineId::GenericAm => "HF / Airband",
            PipelineId::GenericIq => "Banda Base",
            PipelineId::NoaaHrpt => "1.7 GHz",
        }
    }

    pub fn all() -> &'static [PipelineId] {
        &[
            PipelineId::NoaaApt,
            PipelineId::MeteorLrpt,
            PipelineId::IssSstv,
            PipelineId::GenericSstv,
            PipelineId::GenericFm,
            PipelineId::GenericAm,
            PipelineId::GenericIq,
        ]
    }
}

#[derive(Default)]
pub struct PipelinesState {
    pub apt: AptPipeline,
    pub lrpt: LrptPipeline,
    pub iss_sstv: IssSstvPipeline,
    pub generic_sstv: GenericSstvPipeline,
    pub generic_fm: GenericFmPipeline,
    pub generic_am: GenericAmPipeline,
    pub generic_iq: GenericIqPipeline,
    pub hrpt: HrptPipeline,
}

impl PipelinesState {
    /// Libera la memoria pesada (muestras I/Q intermedias, efectos, espectrogramas y audio)
    /// de todos los pipelines inactivos para evitar acumulaciones de RAM.
    pub fn release_inactive(&mut self, active: PipelineId) {
        if active != PipelineId::NoaaApt {
            self.apt.release_heavy_memory();
        }
        if active != PipelineId::MeteorLrpt {
            self.lrpt.release_heavy_memory();
        }
        if active != PipelineId::IssSstv {
            self.iss_sstv.release_heavy_memory();
        }
        if active != PipelineId::GenericSstv {
            self.generic_sstv.release_heavy_memory();
        }
        if active != PipelineId::GenericFm {
            self.generic_fm.release_heavy_memory();
        }
        if active != PipelineId::GenericAm {
            self.generic_am.release_heavy_memory();
        }
        if active != PipelineId::GenericIq {
            self.generic_iq.release_heavy_memory();
        }
        if active != PipelineId::NoaaHrpt {
            self.hrpt.release_heavy_memory();
        }
    }
}
