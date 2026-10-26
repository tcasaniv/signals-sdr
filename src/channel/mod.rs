//! # Simulación de Canal Físico y Degradaciones de RF
//!
//! Este módulo implementa la cadena pedagógica completa de simulación de canal físico
//! de propagación en el espacio libre y medio terrestre, modelando las imperfecciones
//! y perturbaciones que experimentan las señales de radiofrecuencia entre el transmisor y el receptor.
//!
//! ## Modelos Físicos Implementados
//!
//! 1. **Ruido Blanco Gaussiano Aditivo (AWGN):**
//!    Inyección de ruido térmico con control estricto de relación señal a ruido ($\text{SNR}_{\text{dB}}$)
//!    desde $-10\text{ dB}$ hasta $+40\text{ dB}$, empleando la transformada de Box-Muller sobre un generador
//!    pseudoaleatorio determinista ultrarrápido [`effects::FastRng`].
//!
//! 2. **Desplazamiento Doppler Dinámico:**
//!    - **Paso Orbital LEO ([`DopplerMode::LeoPass`]):** Curva tangencial hiperbólica $f_{\text{doppler}}(t) = -f_{\text{max}} \tanh((t - t_0)/\tau)$
//!      que modela el acercamiento, punto de máxima elevación (CPA) y alejamiento de un satélite en órbita baja.
//!    - **Deriva Lineal ([`DopplerMode::LinearDrift`]):** Variación lineal de frecuencia para modelar derivas térmicas de oscilador.
//!    - **Offset Estático ([`DopplerMode::StaticOffset`]):** Error constante de sintonización de portadora.
//!
//! 3. **Desvanecimiento por Multitrayecto (Fading):**
//!    - **Rician:** Componente directa con línea de vista (LOS) más componentes difusas, regulable mediante el factor $K_{\text{dB}}$.
//!    - **Rayleigh:** Caso límite sin visión directa ($K = -\infty\text{ dB}$) para simular entornos urbanos con reflexiones múltiples.
//!    - Filtrado pasobajo de componentes difusas para controlar la tasa de desvanecimiento (*Doppler spread*).
//!
//! 4. **Pérdida por Trayectoria y Atenuación Atmosférica:**
//!    Atenuación basada en la fórmula de propagación en espacio libre (FSPL) y absorción de gases en la troposfera.
//!
//! 5. **Imperfecciones de Frontend RF:**
//!    Desbalance de amplitud I/Q, error de cuadratura de fase y desplazamiento de corriente continua (DC offset)
//!    para entrenar al estudiante en técnicas de corrección I/Q.
//!
//! 6. **Márgenes de Ruido (Noise Padding):**
//!    Inserción de ráfagas de ruido previo y posterior a la señal para permitir la estabilización del AGC
//!    y los bucles de enganche de fase (PLL) en el software receptor.
//!
//! ## Computación Asíncrona sin Truncamiento
//! El estado [`ChannelEffectsState`] gestiona el cómputo en segundo plano sobre un hilo del SO (`std::thread::spawn`),
//! garantizando que la señal resultante conserve exactamente el 100% de las muestras generadas
//! para su posterior emisión íntegra en el Paso 4 de Transmisión.

pub mod effects;

pub use effects::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChannelEffectsTab {
    WithoutEffects,
    WithEffects,
}

#[inline]
pub fn select_effective_iq<'a>(
    tab: ChannelEffectsTab,
    original: &'a [(f32, f32)],
    processed: &'a [(f32, f32)],
) -> &'a [(f32, f32)] {
    match tab {
        ChannelEffectsTab::WithoutEffects => original,
        ChannelEffectsTab::WithEffects => {
            if !processed.is_empty() {
                processed
            } else {
                original
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DopplerMode {
    LeoPass,      // Curva S orbital (tanh)
    LinearDrift,  // Deriva lineal continua
    StaticOffset, // Desplazamiento fijo constante
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChannelPreset {
    // Satélites LEO VHF (APT / LRPT / CubeSat ~137 MHz)
    LeoVhfStandard,
    LeoVhfZenith,
    LeoVhfMarginal,

    // Estación Espacial ISS (145.8 MHz)
    IssStandard,
    IssZenith,
    IssMarginal,

    // Satélite LEO Banda L (HRPT 1.7 GHz)
    LeoLBandStandard,
    LeoLBandAtmosphericLoss,
    LeoLBandZenith,

    // Terrestres / HF / Aeronáutico
    TerrestrialBroadcastLos,
    TerrestrialUrbanRayleigh,
    TerrestrialPmr,
    AirbandVhfAm,
    HfIonospheric,
    MarginalWeakSignal,

    // Modo manual libre
    Custom,
}

impl ChannelPreset {
    /// Devuelve los presets físicos coherentes para un pipeline específico
    pub fn for_pipeline(pipeline_id: &str) -> &'static [ChannelPreset] {
        match pipeline_id {
            "noaa_apt" | "apt" => &[
                ChannelPreset::LeoVhfStandard,
                ChannelPreset::LeoVhfZenith,
                ChannelPreset::LeoVhfMarginal,
                ChannelPreset::Custom,
            ],
            "meteor_lrpt" | "lrpt" => &[
                ChannelPreset::LeoVhfStandard,
                ChannelPreset::LeoVhfZenith,
                ChannelPreset::LeoVhfMarginal,
                ChannelPreset::Custom,
            ],
            "iss_sstv" => &[
                ChannelPreset::IssStandard,
                ChannelPreset::IssZenith,
                ChannelPreset::IssMarginal,
                ChannelPreset::Custom,
            ],
            "generic_sstv" => &[
                ChannelPreset::HfIonospheric,
                ChannelPreset::TerrestrialBroadcastLos,
                ChannelPreset::LeoVhfStandard,
                ChannelPreset::TerrestrialUrbanRayleigh,
                ChannelPreset::Custom,
            ],
            "generic_fm" => &[
                ChannelPreset::TerrestrialBroadcastLos,
                ChannelPreset::TerrestrialUrbanRayleigh,
                ChannelPreset::TerrestrialPmr,
                ChannelPreset::LeoVhfStandard,
                ChannelPreset::MarginalWeakSignal,
                ChannelPreset::Custom,
            ],
            "generic_am" => &[
                ChannelPreset::TerrestrialBroadcastLos,
                ChannelPreset::AirbandVhfAm,
                ChannelPreset::HfIonospheric,
                ChannelPreset::MarginalWeakSignal,
                ChannelPreset::Custom,
            ],
            "noaa_hrpt" | "hrpt" => &[
                ChannelPreset::LeoLBandStandard,
                ChannelPreset::LeoLBandAtmosphericLoss,
                ChannelPreset::LeoLBandZenith,
                ChannelPreset::Custom,
            ],
            _ => &[
                ChannelPreset::LeoVhfStandard,
                ChannelPreset::LeoLBandStandard,
                ChannelPreset::IssStandard,
                ChannelPreset::TerrestrialBroadcastLos,
                ChannelPreset::TerrestrialUrbanRayleigh,
                ChannelPreset::AirbandVhfAm,
                ChannelPreset::HfIonospheric,
                ChannelPreset::MarginalWeakSignal,
                ChannelPreset::Custom,
            ],
        }
    }

    pub fn default_for_pipeline(pipeline_id: &str) -> ChannelPreset {
        Self::for_pipeline(pipeline_id)[0]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LeoVhfStandard => "Satélite LEO VHF Estándar (Órbita 850 km)",
            Self::LeoVhfZenith => "Pase Cenital LEO VHF (Alta Elevación / Señal Fuerte)",
            Self::LeoVhfMarginal => "Pase LEO VHF Marginal (Baja Elevación al Horizonte)",
            Self::IssStandard => "Pase Estación Espacial ISS (Órbita 420 km)",
            Self::IssZenith => "Pase Cenital ISS (Máxima Elevación)",
            Self::IssMarginal => "Señal ISS Débil al Horizonte",
            Self::LeoLBandStandard => "Satélite LEO Banda L (HRPT 1.7 GHz - 850 km)",
            Self::LeoLBandAtmosphericLoss => "Banda L con Atenuación Atmosférica / Lluvia",
            Self::LeoLBandZenith => "Pase Cenital Banda L (Doppler ±40 kHz)",
            Self::TerrestrialBroadcastLos => "Terrestre Línea de Vista (LOS / Canal Limpio)",
            Self::TerrestrialUrbanRayleigh => "Entorno Urbano Denso (Rayleigh NLOS / Multitrayecto)",
            Self::TerrestrialPmr => "Transceptor Portátil PMR / Walkie-Talkie",
            Self::AirbandVhfAm => "Banda Aérea VHF AM (Aeronáutico / Deriva Doppler)",
            Self::HfIonospheric => "Banda HF / Propagación Ionosférica (Fading Lento)",
            Self::MarginalWeakSignal => "Señal Marginal / Límite de Ruido (Deep Fringe)",
            Self::Custom => "Personalizado (Ajuste Manual)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::LeoVhfStandard => {
                "Simula el pase orbital típico de un satélite meteorológico (NOAA / Meteor) a ~850 km de altitud en 137 MHz. \
                 Aplica la curva S Doppler de ±3.5 kHz por velocidad orbital (≈7.5 km/s), atenuación en espacio libre (FSPL) y ruido \
                 térmico realista (SNR 14 dB). Permite comprobar si el demodulador de la estación terrena compensa el Doppler sin perder sincronización."
            }
            Self::LeoVhfZenith => {
                "Simula un pase directo sobre la estación terrena con elevación cercana a 90°. La componente con línea de vista (LOS) \
                 es sumamente fuerte con mínimo desvanecimiento (Rician K=18 dB) y alta relación señal-ruido (SNR 22 dB), \
                 pero con una tasa de variación de Doppler drástica en el punto de máximo acercamiento (TCA)."
            }
            Self::LeoVhfMarginal => {
                "Simula la recepción del satélite cerca del horizonte (< 10° de elevación). La señal atraviesa un mayor espesor atmosférico \
                 y sufre reflexiones en el relieve y construcciones, produciendo multitrayecto severo (Rician K=4 dB), ruido dominante \
                 (SNR 6 dB) y desvanecimiento profundo."
            }
            Self::IssStandard => {
                "Simula la recepción de paquetes de voz o imágenes SSTV desde la Estación Espacial Internacional a 420 km de altitud \
                 en 145.800 MHz. Modela el Doppler orbital característico (±3.2 kHz) y la ráfaga de squelch típica al activarse y \
                 desactivarse el transmisor de a bordo."
            }
            Self::IssZenith => {
                "Recepción directa y despejada de la ISS con visibilidad óptima de antena, máxima SNR (24 dB) y mínima distorsión por multitrayecto."
            }
            Self::IssMarginal => {
                "Simula los primeros o últimos segundos de visibilidad de la ISS antes de ocultarse tras el horizonte geográfico, \
                 con ruido térmico dominante y ráfagas de desvanecimiento."
            }
            Self::LeoLBandStandard => {
                "Simula la transmisión digital de alta velocidad (665 kbps) en microondas (1698 MHz). A esta frecuencia, el efecto Doppler \
                 alcanza ±38 kHz debido a la velocidad orbital de 7.5 km/s y requiere un seguimiento de frecuencia preciso en el demodulador."
            }
            Self::LeoLBandAtmosphericLoss => {
                "Modela la absorción hidrometeórica y dispersión por vapor de agua y precipitación en banda L, introduciendo 3 dB adicionales \
                 de atenuación y degradación de la relación señal-ruido."
            }
            Self::LeoLBandZenith => {
                "Simula el cenit en banda L con máxima potencia de recepción y máxima tasa de cambio de frecuencia Doppler."
            }
            Self::TerrestrialBroadcastLos => {
                "Enlace punto a punto terrestre con visión directa y sin obstrucciones. Modela una portadora dominante sin desplazamiento Doppler, \
                 con desvanecimiento mínimo (Rician K=20 dB) y alto SNR (28 dB). Típico de emisoras comerciales de FM/AM o enlaces de prueba directos."
            }
            Self::TerrestrialUrbanRayleigh => {
                "Simula la propagación en calles urbanas rodeadas de edificios altos sin línea de vista directa (NLOS). Las ondas rebotan \
                 en múltiples estructuras generando interferencia constructiva y destructiva (desvanecimiento Rayleigh puro), produciendo nulos \
                 profundos de señal y fluctuaciones rápidas de envolvente."
            }
            Self::TerrestrialPmr => {
                "Simula comunicaciones de radio bidireccional VHF/UHF portátiles a nivel del suelo. Modela caídas de señal por movimiento del operador, \
                 apantallamiento corporal y apertura/cierre de squelch."
            }
            Self::AirbandVhfAm => {
                "Simula comunicaciones entre torre de control y aeronaves comerciales en vuelo a alta velocidad (~900 km/h). Introduce deriva \
                 Doppler lineal (±500 Hz), reflexión en el suelo y una envolvente AM modulada con ruido ambiente."
            }
            Self::HfIonospheric => {
                "Simula el rebote de ondas de radio en las capas D, E y F de la ionosfera en frecuencias de 3 a 30 MHz. Modela el típico desvanecimiento \
                 selectivo en frecuencia y desvanecimiento lento (fading de 0.2 a 1 Hz), ruido estático atmosférico y rotación de polarización Faraday."
            }
            Self::MarginalWeakSignal => {
                "Simula una estación receptora remota ubicada al límite extremo de cobertura. El ruido blanco gaussiano (AWGN) domina casi por completo \
                 la señal (SNR ~3 a 4 dB), poniendo a prueba los algoritmos de sincronización de portadora y de símbolo del receptor."
            }
            Self::Custom => {
                "Permite configurar libremente cada parámetro físico: SNR de ruido blanco, desplazamiento Doppler dinámico, pérdida en espacio libre, \
                 factor K de multitrayecto y márgenes de ruido."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChannelSimulationParams {
    pub active_preset: ChannelPreset,

    // 1. AWGN (Ruido Blanco Gaussiano Aditivo)
    pub enable_awgn: bool,
    pub snr_db: f32, // SNR en dB (-5.0 a 40.0)

    // 2. Efecto Doppler dinámico
    pub enable_doppler: bool,
    pub doppler_mode: DopplerMode,
    pub max_doppler_hz: f32,

    // 3. Pérdida por Trayectoria (FSPL) y Atenuación
    pub enable_path_loss: bool,
    pub distance_km: f32,
    pub carrier_freq_mhz: f32,
    pub atmospheric_loss_db: f32,
    pub direct_attenuation_db: f32, // Atenuación normalizada aplicada (-40 a 0 dB)

    // 4. Desvanecimiento por Multitrayecto (Rician / Rayleigh)
    pub enable_fading: bool,
    pub rician_k_db: f32, // 0 dB = Rayleigh puro (sin LOS), >15 dB = Rician fuerte (LOS)
    pub fading_rate_hz: f32,

    // 5. Margen de Ruido (Noise Padding pre y post)
    pub enable_noise_padding: bool,
    pub pre_padding_sec: f32,
    pub post_padding_sec: f32,

    // 6. Imperfecciones de Front-End RF
    pub enable_hardware_imp: bool,
    pub iq_imbalance_db: f32,
    pub phase_imbalance_deg: f32,
    pub dc_offset_i: f32,
    pub dc_offset_q: f32,
}

impl Default for ChannelSimulationParams {
    fn default() -> Self {
        Self::from_preset(ChannelPreset::LeoVhfStandard, 137.5)
    }
}

impl ChannelSimulationParams {
    pub fn from_preset(preset: ChannelPreset, default_carrier_mhz: f32) -> Self {
        match preset {
            ChannelPreset::LeoVhfStandard => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 14.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 3500.0,
                enable_path_loss: true,
                distance_km: 850.0,
                carrier_freq_mhz: if default_carrier_mhz > 0.0 { default_carrier_mhz } else { 137.5 },
                atmospheric_loss_db: 0.3,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 12.0,
                fading_rate_hz: 0.5,
                enable_noise_padding: true,
                pre_padding_sec: 1.5,
                post_padding_sec: 1.5,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.1,
                phase_imbalance_deg: 0.5,
                dc_offset_i: 0.005,
                dc_offset_q: 0.005,
            },
            ChannelPreset::LeoVhfZenith => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 22.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 4200.0,
                enable_path_loss: true,
                distance_km: 850.0,
                carrier_freq_mhz: if default_carrier_mhz > 0.0 { default_carrier_mhz } else { 137.5 },
                atmospheric_loss_db: 0.2,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 18.0,
                fading_rate_hz: 0.3,
                enable_noise_padding: true,
                pre_padding_sec: 1.0,
                post_padding_sec: 1.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::LeoVhfMarginal => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 6.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 3000.0,
                enable_path_loss: true,
                distance_km: 1800.0,
                carrier_freq_mhz: if default_carrier_mhz > 0.0 { default_carrier_mhz } else { 137.5 },
                atmospheric_loss_db: 1.2,
                direct_attenuation_db: -6.0,
                enable_fading: true,
                rician_k_db: 4.0,
                fading_rate_hz: 1.2,
                enable_noise_padding: true,
                pre_padding_sec: 2.5,
                post_padding_sec: 2.5,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.2,
                phase_imbalance_deg: 1.0,
                dc_offset_i: 0.01,
                dc_offset_q: 0.01,
            },
            ChannelPreset::IssStandard => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 15.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 3200.0,
                enable_path_loss: true,
                distance_km: 420.0,
                carrier_freq_mhz: 145.8,
                atmospheric_loss_db: 0.2,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 14.0,
                fading_rate_hz: 0.4,
                enable_noise_padding: true,
                pre_padding_sec: 2.0,
                post_padding_sec: 2.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::IssZenith => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 24.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 3800.0,
                enable_path_loss: true,
                distance_km: 420.0,
                carrier_freq_mhz: 145.8,
                atmospheric_loss_db: 0.1,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 20.0,
                fading_rate_hz: 0.2,
                enable_noise_padding: true,
                pre_padding_sec: 1.0,
                post_padding_sec: 1.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::IssMarginal => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 5.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 2800.0,
                enable_path_loss: true,
                distance_km: 1500.0,
                carrier_freq_mhz: 145.8,
                atmospheric_loss_db: 1.0,
                direct_attenuation_db: -8.0,
                enable_fading: true,
                rician_k_db: 3.0,
                fading_rate_hz: 1.5,
                enable_noise_padding: true,
                pre_padding_sec: 3.0,
                post_padding_sec: 3.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.15,
                phase_imbalance_deg: 0.8,
                dc_offset_i: 0.01,
                dc_offset_q: 0.01,
            },
            ChannelPreset::LeoLBandStandard => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 16.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 38000.0,
                enable_path_loss: true,
                distance_km: 850.0,
                carrier_freq_mhz: 1698.0,
                atmospheric_loss_db: 0.5,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 15.0,
                fading_rate_hz: 2.0,
                enable_noise_padding: true,
                pre_padding_sec: 1.0,
                post_padding_sec: 1.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.1,
                phase_imbalance_deg: 0.5,
                dc_offset_i: 0.005,
                dc_offset_q: 0.005,
            },
            ChannelPreset::LeoLBandAtmosphericLoss => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 10.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 38000.0,
                enable_path_loss: true,
                distance_km: 850.0,
                carrier_freq_mhz: 1698.0,
                atmospheric_loss_db: 3.0,
                direct_attenuation_db: -3.0,
                enable_fading: true,
                rician_k_db: 8.0,
                fading_rate_hz: 3.0,
                enable_noise_padding: true,
                pre_padding_sec: 1.5,
                post_padding_sec: 1.5,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.1,
                phase_imbalance_deg: 0.5,
                dc_offset_i: 0.005,
                dc_offset_q: 0.005,
            },
            ChannelPreset::LeoLBandZenith => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 22.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 42000.0,
                enable_path_loss: true,
                distance_km: 850.0,
                carrier_freq_mhz: 1698.0,
                atmospheric_loss_db: 0.3,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 18.0,
                fading_rate_hz: 1.5,
                enable_noise_padding: true,
                pre_padding_sec: 0.8,
                post_padding_sec: 0.8,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::TerrestrialBroadcastLos => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 28.0,
                enable_doppler: false,
                doppler_mode: DopplerMode::StaticOffset,
                max_doppler_hz: 0.0,
                enable_path_loss: false,
                distance_km: 15.0,
                carrier_freq_mhz: if default_carrier_mhz > 0.0 { default_carrier_mhz } else { 98.5 },
                atmospheric_loss_db: 0.0,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 20.0,
                fading_rate_hz: 0.1,
                enable_noise_padding: true,
                pre_padding_sec: 0.5,
                post_padding_sec: 0.5,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::TerrestrialUrbanRayleigh => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 11.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LinearDrift,
                max_doppler_hz: 50.0,
                enable_path_loss: true,
                distance_km: 5.0,
                carrier_freq_mhz: if default_carrier_mhz > 0.0 { default_carrier_mhz } else { 433.0 },
                atmospheric_loss_db: 0.0,
                direct_attenuation_db: -4.0,
                enable_fading: true,
                rician_k_db: 0.0, // Rayleigh puro
                fading_rate_hz: 15.0,
                enable_noise_padding: true,
                pre_padding_sec: 1.0,
                post_padding_sec: 1.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.1,
                phase_imbalance_deg: 0.5,
                dc_offset_i: 0.005,
                dc_offset_q: 0.005,
            },
            ChannelPreset::TerrestrialPmr => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 15.0,
                enable_doppler: false,
                doppler_mode: DopplerMode::StaticOffset,
                max_doppler_hz: 0.0,
                enable_path_loss: false,
                distance_km: 2.0,
                carrier_freq_mhz: 446.0,
                atmospheric_loss_db: 0.0,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 8.0,
                fading_rate_hz: 1.0,
                enable_noise_padding: true,
                pre_padding_sec: 0.8,
                post_padding_sec: 0.8,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::AirbandVhfAm => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 18.0,
                enable_doppler: true,
                doppler_mode: DopplerMode::LinearDrift,
                max_doppler_hz: 500.0,
                enable_path_loss: true,
                distance_km: 50.0,
                carrier_freq_mhz: 125.0,
                atmospheric_loss_db: 0.1,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 16.0,
                fading_rate_hz: 0.8,
                enable_noise_padding: true,
                pre_padding_sec: 1.0,
                post_padding_sec: 1.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.05,
                phase_imbalance_deg: 0.2,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::HfIonospheric => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 12.0,
                enable_doppler: false,
                doppler_mode: DopplerMode::StaticOffset,
                max_doppler_hz: 0.0,
                enable_path_loss: false,
                distance_km: 1200.0,
                carrier_freq_mhz: 14.23,
                atmospheric_loss_db: 0.0,
                direct_attenuation_db: 0.0,
                enable_fading: true,
                rician_k_db: 2.0,
                fading_rate_hz: 0.5,
                enable_noise_padding: true,
                pre_padding_sec: 1.2,
                post_padding_sec: 1.2,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
            ChannelPreset::MarginalWeakSignal => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 3.0,
                enable_doppler: false,
                doppler_mode: DopplerMode::StaticOffset,
                max_doppler_hz: 0.0,
                enable_path_loss: false,
                distance_km: 500.0,
                carrier_freq_mhz: if default_carrier_mhz > 0.0 { default_carrier_mhz } else { 137.5 },
                atmospheric_loss_db: 0.0,
                direct_attenuation_db: -12.0,
                enable_fading: true,
                rician_k_db: 3.0,
                fading_rate_hz: 2.0,
                enable_noise_padding: true,
                pre_padding_sec: 3.0,
                post_padding_sec: 3.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.1,
                phase_imbalance_deg: 0.5,
                dc_offset_i: 0.01,
                dc_offset_q: 0.01,
            },
            ChannelPreset::Custom => Self {
                active_preset: preset,
                enable_awgn: true,
                snr_db: 15.0,
                enable_doppler: false,
                doppler_mode: DopplerMode::LeoPass,
                max_doppler_hz: 3000.0,
                enable_path_loss: false,
                distance_km: 100.0,
                carrier_freq_mhz: if default_carrier_mhz > 0.0 { default_carrier_mhz } else { 137.5 },
                atmospheric_loss_db: 0.0,
                direct_attenuation_db: 0.0,
                enable_fading: false,
                rician_k_db: 10.0,
                fading_rate_hz: 1.0,
                enable_noise_padding: false,
                pre_padding_sec: 1.0,
                post_padding_sec: 1.0,
                enable_hardware_imp: false,
                iq_imbalance_db: 0.0,
                phase_imbalance_deg: 0.0,
                dc_offset_i: 0.0,
                dc_offset_q: 0.0,
            },
        }
    }
}

pub struct ChannelEffectsState {
    pub active_tab: ChannelEffectsTab,
    pub params: ChannelSimulationParams,
    pub processed_iq: Vec<(f32, f32)>,
    pub is_dirty: bool,
    pub last_processing_ms: f32,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    /// true mientras se está calculando en background (WASM async / Desktop thread)
    pub is_computing: bool,
    /// Canal de retorno del cómputo asíncrono
    pub compute_result: std::sync::Arc<std::sync::Mutex<Option<(Vec<(f32, f32)>, f32)>>>,
    /// Bandera que indica si debe avanzarse automáticamente al Paso 4 de Transmisión al finalizar el cómputo
    pub pending_continue_to_tx: bool,
}

impl ChannelEffectsState {
    pub fn new_for_pipeline(pipeline_id: &str, default_carrier_mhz: f32) -> Self {
        let preset = ChannelPreset::default_for_pipeline(pipeline_id);
        Self {
            active_tab: ChannelEffectsTab::WithoutEffects,
            params: ChannelSimulationParams::from_preset(preset, default_carrier_mhz),
            processed_iq: Vec::new(),
            is_dirty: true,
            last_processing_ms: 0.0,
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            is_computing: false,
            compute_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
            pending_continue_to_tx: false,
        }
    }

    /// Libera vectores pesados e imágenes de memoria cuando el pipeline está inactivo.
    pub fn release_memory(&mut self) {
        self.processed_iq.clear();
        self.processed_iq.shrink_to_fit();
        self.is_dirty = true;
        self.is_computing = false;
        self.pending_continue_to_tx = false;
        self.spectrogram_state.release_memory();
    }

    /// Recoge el resultado del cómputo asíncrono si está listo. Devuelve true si se actualizó.
    pub fn poll_compute_result(&mut self) -> bool {
        if let Ok(mut guard) = self.compute_result.try_lock() {
            if let Some((samples, elapsed_ms)) = guard.take() {
                self.processed_iq = samples;
                self.last_processing_ms = elapsed_ms;
                self.is_dirty = false;
                self.is_computing = false;
                return true;
            }
        }
        false
    }
}

impl Default for ChannelEffectsState {
    fn default() -> Self {
        Self::new_for_pipeline("generic_iq", 137.5)
    }
}
