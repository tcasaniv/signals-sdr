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

    pub fn all() -> &'static [PipelineId] {
        &[
            PipelineId::NoaaApt,
            PipelineId::MeteorLrpt,
            PipelineId::IssSstv,
            PipelineId::GenericSstv,
            PipelineId::GenericFm,
            PipelineId::GenericAm,
            PipelineId::GenericIq,
            PipelineId::NoaaHrpt,
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
