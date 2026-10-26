use super::*;
use std::f32::consts::PI;

/// Generador de números pseudoaleatorios rápido, ligero y determinista (XorShift64)
/// Ideal para DSP sin requerir dependencias pesadas.
pub struct FastRng {
    state: u64,
}

impl FastRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x5432109876543210 } else { seed },
        }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        // [0.0, 1.0)
        (self.next_u64() >> 40) as f32 / 16777216.0
    }

    /// Genera dos variables aleatorias normales independientes N(0, 1) vía Box-Muller
    #[inline]
    pub fn next_gaussian_pair(&mut self) -> (f32, f32) {
        let mut u1 = self.next_f32();
        while u1 <= 1e-7 {
            u1 = self.next_f32();
        }
        let u2 = self.next_f32();

        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * PI * u2;

        (r * theta.cos(), r * theta.sin())
    }
}

/// Aplica la cadena completa de simulación de canal físico a las muestras I/Q de entrada
pub fn apply_channel_effects(
    input_iq: &[(f32, f32)],
    sample_rate: u32,
    params: &ChannelSimulationParams,
) -> Vec<(f32, f32)> {
    if input_iq.is_empty() || sample_rate == 0 {
        return Vec::new();
    }

    let mut rng = FastRng::new(0xABCD_EF01_2345_6789);
    let sample_rate_f = sample_rate as f32;
    let n_samples = input_iq.len();

    // 1. Clonar muestras base
    let mut current_iq: Vec<(f32, f32)> = input_iq.to_vec();

    // 2. Pérdida por Trayectoria (FSPL) y Atenuación
    if params.enable_path_loss {
        // FSPL(dB) = 20*log10(d_km) + 20*log10(f_mhz) + 32.44
        // Aplicamos atenuación normalizada en rango práctico para DSP
        let direct_gain = 10.0_f32.powf(params.direct_attenuation_db / 20.0);
        let atm_loss_factor = 10.0_f32.powf(-params.atmospheric_loss_db / 20.0);
        let total_scale = (direct_gain * atm_loss_factor).clamp(0.001, 1.0);

        for (i, q) in current_iq.iter_mut() {
            *i *= total_scale;
            *q *= total_scale;
        }
    }

    // 3. Desvanecimiento por Multitrayecto (Rician / Rayleigh)
    if params.enable_fading {
        // K_lin = 10^(K_db / 10)
        let k_lin = 10.0_f32.powf(params.rician_k_db / 10.0);
        let direct_coeff = (k_lin / (k_lin + 1.0)).sqrt();
        let diffuse_coeff = (1.0 / (k_lin + 1.0)).sqrt();

        // Filtro paso bajo para simular variación temporal lenta (Doppler spread fm)
        let fm = params.fading_rate_hz.max(0.05);
        let alpha = (2.0 * PI * fm / sample_rate_f).clamp(0.0001, 0.5);

        const FADING_BLOCK: usize = 64;
        let mut diff_i = 0.0_f32;
        let mut diff_q = 0.0_f32;
        let step_alpha = (alpha * FADING_BLOCK as f32).min(0.5);

        for chunk in current_iq.chunks_mut(FADING_BLOCK) {
            let (noise_i, noise_q) = rng.next_gaussian_pair();
            diff_i += step_alpha * (noise_i - diff_i);
            diff_q += step_alpha * (noise_q - diff_q);

            let h_i = direct_coeff + diffuse_coeff * diff_i;
            let h_q = diffuse_coeff * diff_q;

            for (i, q) in chunk {
                let orig_i = *i;
                let orig_q = *q;
                *i = orig_i * h_i - orig_q * h_q;
                *q = orig_i * h_q + orig_q * h_i;
            }
        }
    }

    // 4. Efecto Doppler dinámico (Rotación de fase continua)
    if params.enable_doppler && params.max_doppler_hz.abs() > 0.1 {
        let total_time = n_samples as f32 / sample_rate_f;
        let mid_time = total_time * 0.5;
        let tau = (total_time / 6.0).max(0.01);

        let mut phase = 0.0_f32;
        const DOPPLER_BLOCK: usize = 64;
        let mut sample_idx = 0usize;

        for chunk in current_iq.chunks_mut(DOPPLER_BLOCK) {
            let t = sample_idx as f32 / sample_rate_f;
            let freq_offset = match params.doppler_mode {
                DopplerMode::LeoPass => {
                    -params.max_doppler_hz * ((t - mid_time) / tau).tanh()
                }
                DopplerMode::LinearDrift => {
                    let progress = if total_time > 0.0 { t / total_time } else { 0.0 };
                    params.max_doppler_hz * (1.0 - 2.0 * progress)
                }
                DopplerMode::StaticOffset => params.max_doppler_hz,
            };

            let dphase = 2.0 * PI * (freq_offset / sample_rate_f);

            for (i, q) in chunk {
                phase += dphase;
                if phase > PI {
                    phase -= 2.0 * PI;
                } else if phase < -PI {
                    phase += 2.0 * PI;
                }

                let cos_p = phase.cos();
                let sin_p = phase.sin();

                let orig_i = *i;
                let orig_q = *q;
                *i = orig_i * cos_p - orig_q * sin_p;
                *q = orig_i * sin_p + orig_q * cos_p;
            }
            sample_idx += DOPPLER_BLOCK;
        }
    }

    // 5. Imperfecciones de Front-End RF (I/Q imbalance & DC offset)
    if params.enable_hardware_imp {
        let amp_imb = 10.0_f32.powf(params.iq_imbalance_db / 20.0);
        let phase_err_rad = params.phase_imbalance_deg * (PI / 180.0);
        let cos_phi = phase_err_rad.cos();
        let sin_phi = phase_err_rad.sin();

        for (i, q) in current_iq.iter_mut() {
            let orig_i = *i;
            let orig_q = *q;

            // Desbalance de amplitud y error de fase
            let i_imb = orig_i * amp_imb;
            let q_imb = orig_q * cos_phi + orig_i * sin_phi;

            // Añadir fuga de portadora (DC Spike)
            *i = i_imb + params.dc_offset_i;
            *q = q_imb + params.dc_offset_q;
        }
    }

    // 6. Ruido Blanco Gaussiano Aditivo (AWGN)
    let noise_sigma_per_comp = if params.enable_awgn {
        // Calcular potencia media de la señal: Ps = (1/N) * sum(I^2 + Q^2)
        let total_power: f32 = current_iq.iter().map(|(i, q)| i * i + q * q).sum();
        let signal_power = (total_power / n_samples.max(1) as f32).max(1e-9);

        // SNR = Ps / Pn => Pn = Ps / 10^(SNR/10)
        let snr_lin = 10.0_f32.powf(params.snr_db / 10.0);
        let noise_power = signal_power / snr_lin;

        // Desviación estándar por componente I y Q: sigma = sqrt(Pn / 2)
        let sigma = (noise_power * 0.5).sqrt();

        for (i, q) in current_iq.iter_mut() {
            let (noise_i, noise_q) = rng.next_gaussian_pair();
            *i += noise_i * sigma;
            *q += noise_q * sigma;
        }

        sigma
    } else {
        // Estimación de piso base para el padding si AWGN está apagado
        0.001
    };

    // 7. Margen de Ruido (Noise Padding pre y post)
    if params.enable_noise_padding && (params.pre_padding_sec > 0.0 || params.post_padding_sec > 0.0) {
        let n_pre = (params.pre_padding_sec.max(0.0) * sample_rate_f) as usize;
        let n_post = (params.post_padding_sec.max(0.0) * sample_rate_f) as usize;

        let mut output = Vec::with_capacity(n_pre + current_iq.len() + n_post);

        // Generar ruido previo
        for _ in 0..n_pre {
            let (ni, nq) = rng.next_gaussian_pair();
            output.push((ni * noise_sigma_per_comp, nq * noise_sigma_per_comp));
        }

        // Suavizado en la transición (Hann window ramp 5 ms)
        let ramp_len = ((sample_rate_f * 0.005) as usize).clamp(1, current_iq.len() / 4);

        for (idx, (i, q)) in current_iq.into_iter().enumerate() {
            let factor = if idx < ramp_len {
                0.5 * (1.0 - (PI * idx as f32 / ramp_len as f32).cos())
            } else if idx >= n_samples - ramp_len {
                let tail_idx = n_samples - 1 - idx;
                0.5 * (1.0 - (PI * tail_idx as f32 / ramp_len as f32).cos())
            } else {
                1.0
            };
            output.push((i * factor, q * factor));
        }

        // Generar ruido posterior
        for _ in 0..n_post {
            let (ni, nq) = rng.next_gaussian_pair();
            output.push((ni * noise_sigma_per_comp, nq * noise_sigma_per_comp));
        }

        output
    } else {
        current_iq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_rng_determinism() {
        let mut rng1 = FastRng::new(42);
        let mut rng2 = FastRng::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
            assert_eq!(rng1.next_gaussian_pair(), rng2.next_gaussian_pair());
        }
    }

    #[test]
    fn test_awgn_snr_statistics() {
        let sample_rate = 100_000;
        let n_samples = 50_000;
        // Señal senoidal pura con amplitud 1.0 (Potencia = 1.0)
        let mut clean_signal = Vec::with_capacity(n_samples);
        for n in 0..n_samples {
            let phase = 2.0 * PI * 1000.0 * (n as f32 / sample_rate as f32);
            clean_signal.push((phase.cos(), phase.sin()));
        }

        let mut params = ChannelSimulationParams::from_preset(ChannelPreset::Custom, 137.5);
        params.enable_awgn = true;
        params.snr_db = 10.0; // SNR objetivo = 10 dB
        params.enable_doppler = false;
        params.enable_fading = false;
        params.enable_path_loss = false;
        params.enable_noise_padding = false;

        let noisy = apply_channel_effects(&clean_signal, sample_rate, &params);
        assert_eq!(noisy.len(), clean_signal.len());

        // Comprobar potencia de ruido residual
        let mut noise_power = 0.0_f32;
        for (clean, with_noise) in clean_signal.iter().zip(noisy.iter()) {
            let di = with_noise.0 - clean.0;
            let dq = with_noise.1 - clean.1;
            noise_power += di * di + dq * dq;
        }
        noise_power /= n_samples as f32;

        let measured_snr_lin = 1.0 / noise_power;
        let measured_snr_db = 10.0 * measured_snr_lin.log10();

        // Debe coincidir con 10 dB con margen estadístico menor a 0.5 dB
        assert!(
            (measured_snr_db - 10.0).abs() < 0.6,
            "SNR medida ({:.2} dB) difiere del objetivo (10 dB)",
            measured_snr_db
        );
    }

    #[test]
    fn test_noise_padding_length() {
        let sample_rate = 10_000;
        let input = vec![(0.5, 0.5); 10_000]; // 1 segundo
        let mut params = ChannelSimulationParams::from_preset(ChannelPreset::Custom, 137.5);
        params.enable_awgn = false;
        params.enable_doppler = false;
        params.enable_noise_padding = true;
        params.pre_padding_sec = 0.5;  // 5000 muestras
        params.post_padding_sec = 1.0; // 10000 muestras

        let output = apply_channel_effects(&input, sample_rate, &params);
        let expected_len = 10_000 + 5_000 + 10_000;
        assert_eq!(output.len(), expected_len);
    }

    #[test]
    fn test_doppler_rotation_finite() {
        let sample_rate = 48_000;
        let input = vec![(0.8, -0.6); 20_000];
        let mut params = ChannelSimulationParams::from_preset(ChannelPreset::LeoVhfStandard, 137.5);
        params.enable_awgn = false;
        params.enable_fading = false;
        params.enable_noise_padding = false;

        let output = apply_channel_effects(&input, sample_rate, &params);
        assert_eq!(output.len(), input.len());
        for (i, q) in output {
            assert!(i.is_finite());
            assert!(q.is_finite());
            // Comprobar conservación aproximada de amplitud unitaria
            let mag = (i * i + q * q).sqrt();
            assert!((mag - 1.0).abs() < 0.05);
        }
    }

    #[test]
    fn test_channel_presets_filtering() {
        let apt_presets = ChannelPreset::for_pipeline("noaa_apt");
        assert!(apt_presets.contains(&ChannelPreset::LeoVhfStandard));
        assert!(apt_presets.contains(&ChannelPreset::LeoVhfZenith));
        assert!(!apt_presets.contains(&ChannelPreset::AirbandVhfAm));

        let am_presets = ChannelPreset::for_pipeline("generic_am");
        assert!(am_presets.contains(&ChannelPreset::AirbandVhfAm));
        assert!(!am_presets.contains(&ChannelPreset::LeoLBandStandard));
    }

    #[test]
    fn test_preset_descriptions_non_empty() {
        let presets = [
            ChannelPreset::LeoVhfStandard,
            ChannelPreset::LeoVhfZenith,
            ChannelPreset::LeoVhfMarginal,
            ChannelPreset::IssStandard,
            ChannelPreset::IssZenith,
            ChannelPreset::IssMarginal,
            ChannelPreset::LeoLBandStandard,
            ChannelPreset::LeoLBandAtmosphericLoss,
            ChannelPreset::LeoLBandZenith,
            ChannelPreset::TerrestrialBroadcastLos,
            ChannelPreset::TerrestrialUrbanRayleigh,
            ChannelPreset::TerrestrialPmr,
            ChannelPreset::AirbandVhfAm,
            ChannelPreset::HfIonospheric,
            ChannelPreset::MarginalWeakSignal,
            ChannelPreset::Custom,
        ];

        for p in presets {
            assert!(!p.display_name().is_empty());
            assert!(!p.description().is_empty());
        }
    }

    #[test]
    fn test_large_signal_preserves_length_without_truncation() {
        let sample_rate = 1_000_000;
        let n_samples = 1_000_000; // 1 millón de muestras (supera límites antiguos de 300k y 600k)
        let large_signal = vec![(0.3f32, -0.4f32); n_samples];

        let mut params = ChannelSimulationParams::from_preset(ChannelPreset::LeoVhfStandard, 137.5);
        params.enable_noise_padding = false; // Sin padding para comprobar 1:1

        let output = apply_channel_effects(&large_signal, sample_rate, &params);
        assert_eq!(
            output.len(),
            n_samples,
            "La señal resultante debe preservar exactamente el número total de muestras ({}) sin truncamientos",
            n_samples
        );
    }
}
