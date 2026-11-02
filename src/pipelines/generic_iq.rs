use crate::channel::ChannelEffectsState;
use crate::config::AppConfig;
use crate::export::formats::{read_iq_file, IqExportFormat};
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::channel_effects::render_channel_effects_tab;
use crate::ui::components::{accordion_section, info_card, step_bar, AccordionOptions};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub struct GenericIqPipeline {
    pub current_step: usize,
    pub input_file_path: Option<PathBuf>,
    pub input_format: IqExportFormat,
    pub sample_rate: u32,
    pub raw_iq_samples: Vec<(f32, f32)>,
    pub processed_iq_samples: Vec<(f32, f32)>,
    pub async_file_receiver: Option<Arc<Mutex<Option<(String, Vec<u8>)>>>>,
    // Ajustes de procesamiento y limpieza I/Q
    pub gain_factor: f32,
    pub freq_shift_hz: f32,
    pub remove_dc: bool,
    pub invert_iq: bool,
    pub iq_balance_correction: bool,
    pub lpf_enabled: bool,
    pub lpf_cutoff_hz: f32,
    pub agc_enabled: bool,
    pub agc_target_level: f32,
    pub squelch_enabled: bool,
    pub squelch_threshold_db: f32,
    pub status_message: String,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    pub channel_effects_state: ChannelEffectsState,
    pub transmission_state: TransmissionTabState,
}

impl Default for GenericIqPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            input_file_path: None,
            input_format: IqExportFormat::WavIq16,
            sample_rate: 288_000,
            raw_iq_samples: Vec::new(),
            processed_iq_samples: Vec::new(),
            async_file_receiver: None,
            gain_factor: 1.0,
            freq_shift_hz: 0.0,
            remove_dc: true,
            invert_iq: false,
            iq_balance_correction: false,
            lpf_enabled: false,
            lpf_cutoff_hz: 75_000.0,
            agc_enabled: false,
            agc_target_level: 0.8,
            squelch_enabled: false,
            squelch_threshold_db: -55.0,
            status_message: "Listo para cargar archivo de muestras I/Q pregrabadas.".to_string(),
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            channel_effects_state: ChannelEffectsState::new_for_pipeline("generic_iq", 144.8),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl GenericIqPipeline {
    pub fn release_heavy_memory(&mut self) {
        self.processed_iq_samples.clear();
        self.processed_iq_samples.shrink_to_fit();
        self.spectrogram_state.release_memory();
        self.channel_effects_state.release_memory();
        self.transmission_state.release_memory();
    }

    pub fn get_effective_iq(&self) -> &[(f32, f32)] {
        let base = if !self.processed_iq_samples.is_empty() {
            &self.processed_iq_samples
        } else {
            &self.raw_iq_samples
        };
        crate::channel::select_effective_iq(
            self.channel_effects_state.active_tab,
            base,
            &self.channel_effects_state.processed_iq,
        )
    }

    pub fn render(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        config: &AppConfig,
        sdr_controller: &mut SdrTransmissionController,
    ) {
        info_card(
            ui,
            theme,
            "Generic IQ (Gestor y Transmisor de Archivos I/Q)",
            "Permite cargar archivos de señal I/Q existentes en formatos estándar (WAV I/Q estéreo, RAW SC16 bladeRF, \
             RAW int8 HackRF, RAW float32), inspeccionar la envolvente en el osciloscopio y analizador de espectro, \
             aplicar corrección de DC/frecuencia, y transmitirlos directamente con SDR o convertirlos a otro formato.",
        );

        ui.add_space(8.0);
        step_bar(ui, theme, &mut self.current_step, "2. Procesamiento");

        match self.current_step {
            0 => self.render_step_source(ui, theme),
            1 => self.render_step_processing(ui, theme, config),
            2 => {
                let base = if !self.processed_iq_samples.is_empty() {
                    &self.processed_iq_samples
                } else {
                    &self.raw_iq_samples
                };
                let mut next_step = false;
                render_channel_effects_tab(
                    ui,
                    theme,
                    config,
                    "generic_iq",
                    self.sample_rate,
                    144_800_000.0,
                    base,
                    &mut self.channel_effects_state,
                    || next_step = true,
                );
                if next_step {
                    self.current_step = 3;
                }
            }
            3 => {
                let base = if !self.processed_iq_samples.is_empty() {
                    &self.processed_iq_samples
                } else {
                    &self.raw_iq_samples
                };
                let effective_iq = crate::channel::select_effective_iq(
                    self.channel_effects_state.active_tab,
                    base,
                    &self.channel_effects_state.processed_iq,
                );

                render_transmission_tab(
                    ui,
                    theme,
                    config,
                    "generic_iq",
                    144_800_000.0,
                    &mut self.transmission_state,
                    sdr_controller,
                    effective_iq,
                    self.sample_rate,
                );
            }
            _ => {}
        }
    }

    pub fn load_iq_bytes(&mut self, filename: &str, bytes: Vec<u8>) {
        self.input_file_path = Some(PathBuf::from(filename));
        if let Some(ext) = std::path::Path::new(filename).extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("wav") {
                self.input_format = IqExportFormat::WavIq16;
            }
        }
        match crate::export::formats::read_iq_from_memory(&bytes, self.input_format) {
            Ok((samples, detected_sr)) => {
                self.sample_rate = detected_sr;
                let count = samples.len();
                self.raw_iq_samples = samples;
                self.process_samples();
                self.status_message = format!(
                    "Archivo cargado exitosamente: {} muestras a {} Hz ({:.2} seg).",
                    count,
                    self.sample_rate,
                    count as f32 / self.sample_rate as f32
                );
            }
            Err(e) => {
                self.status_message = format!("Error al cargar archivo I/Q: {}", e);
            }
        }
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme) {
        // Sondeo de archivo I/Q cargado asíncronamente
        let loaded_async = if let Some(ref rx) = self.async_file_receiver {
            rx.lock().ok().and_then(|mut guard| guard.take())
        } else {
            None
        };
        if let Some((name, bytes)) = loaded_async {
            self.async_file_receiver = None;
            self.load_iq_bytes(&name, bytes);
        }

        ui.vertical(|ui| {
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Seleccionar Archivo de Muestras I/Q").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            let pick_label = format!(
                                "{} Examinar Archivo I/Q...",
                                crate::icons::ICON_FOLDER_OPEN.codepoint
                            );
                            let pick_btn = egui::Button::new(RichText::new(pick_label).size(12.5))
                                .fill(theme.surface_subtle())
                                .stroke(Stroke::new(1.0_f32, theme.card_border()));

                            if ui.add(pick_btn).clicked() {
                                let slot = Arc::new(Mutex::new(None));
                                self.async_file_receiver = Some(slot.clone());
                                crate::file_dialog::pick_file_async(
                                    "Muestras I/Q o WAV",
                                    &["wav", "raw", "iq", "bin", "sc16", "dat"],
                                    move |name, bytes| {
                                        if let Ok(mut guard) = slot.lock() {
                                            *guard = Some((name, bytes));
                                        }
                                    },
                                );
                            }

                            let synth_label = format!(
                                "{} Generar Tono I/Q Sintético",
                                crate::icons::ICON_TUNE.codepoint
                            );
                            if ui.button(RichText::new(synth_label).size(12.5)).clicked() {
                                self.generate_synthetic_iq();
                            }
                        });

                        ui.add_space(8.0);
                        if let Some(path) = &self.input_file_path {
                            ui.label(RichText::new(format!("Archivo: {}", path.display())).monospace().size(12.0));
                        }

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Formato de origen:");
                            egui::ComboBox::from_id_salt("iq_input_format_combo")
                                .selected_text(self.input_format.label())
                                .show_ui(ui, |ui| {
                                    let formats = [
                                        IqExportFormat::WavIq16,
                                        IqExportFormat::RawSc16Q11,
                                        IqExportFormat::RawSc16Full,
                                        IqExportFormat::RawInt8,
                                        IqExportFormat::RawFloat32,
                                        IqExportFormat::WavAudioMono,
                                    ];
                                    for fmt in formats {
                                        if ui.selectable_value(&mut self.input_format, fmt, fmt.label()).clicked() {
                                            if self.input_file_path.is_some() {
                                                self.load_selected_file();
                                            }
                                        }
                                    }
                                });
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Tasa de muestreo (Hz):");
                            ui.add(egui::DragValue::new(&mut self.sample_rate).speed(1000).range(8_000..=20_000_000));
                        });

                        ui.add_space(8.0);
                        ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                    });
                });

            ui.add_space(10.0);
            if !self.raw_iq_samples.is_empty() {
                let next_label = format!(
                    "Continuar a Procesamiento y Análisis {}",
                    crate::icons::ICON_ARROW_FORWARD.codepoint
                );
                if ui
                    .button(RichText::new(next_label).strong())
                    .clicked()
                {
                    self.process_samples();
                    self.current_step = 1;
                }
            }
        });
    }

    fn render_step_processing(&mut self, ui: &mut Ui, theme: &AppTheme, config: &AppConfig) {
        ui.vertical(|ui| {
            accordion_section(
                ui,
                theme,
                "generic_iq_params_basic",
                AccordionOptions::new("1. Acondicionamiento Básico y Frecuencia").default_open(true),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Factor de Escala / Ganancia:");
                        if ui.add(egui::Slider::new(&mut self.gain_factor, 0.1..=5.0).text("x")).changed() {
                            self.process_samples();
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Desplazamiento de Frecuencia (Shift Hz):");
                        if ui.add(egui::Slider::new(&mut self.freq_shift_hz, -100_000.0..=100_000.0).suffix(" Hz")).changed() {
                            self.process_samples();
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Tasa de Muestreo SDR I/Q:");
                        egui::ComboBox::from_id_salt("generic_iq_sdr_rate_combo")
                            .selected_text(format!("{} Sps ({} kHz)", self.sample_rate, self.sample_rate / 1000))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.sample_rate, 100_000, "100 kHz");
                                ui.selectable_value(&mut self.sample_rate, 250_000, "250 kHz");
                                ui.selectable_value(&mut self.sample_rate, 288_000, "288 kHz (Meteor LRPT)");
                                ui.selectable_value(&mut self.sample_rate, 500_000, "500 kHz");
                                ui.selectable_value(&mut self.sample_rate, 1_000_000, "1.0 MHz");
                                ui.selectable_value(&mut self.sample_rate, 2_000_000, "2.0 MHz");
                                ui.selectable_value(&mut self.sample_rate, 2_400_000, "2.4 MHz");
                                ui.selectable_value(&mut self.sample_rate, 8_000_000, "8.0 MHz");
                            });
                        ui.add(egui::DragValue::new(&mut self.sample_rate).speed(1000).range(8_000..=20_000_000).suffix(" Hz"));
                    });

                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.checkbox(&mut self.remove_dc, "Eliminar componente continua (DC Offset)").changed() {
                            self.process_samples();
                        }
                        if ui.checkbox(&mut self.invert_iq, "Invertir Espectro (Invert I/Q / Conjugate)").changed() {
                            self.process_samples();
                        }
                    });
                },
            );

            ui.add_space(6.0);
            accordion_section(
                ui,
                theme,
                "generic_iq_params_filtering",
                AccordionOptions::new("2. Filtros y Supresión de Ruido (Denoising)").default_open(true),
                |ui| {
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.lpf_enabled, "Activar Filtro Pasa-Bajos FIR").changed() {
                            self.process_samples();
                        }
                        if self.lpf_enabled {
                            ui.label("Frecuencia de Corte:");
                            let max_cutoff = (self.sample_rate / 2).saturating_sub(1000) as f32;
                            if ui.add(egui::Slider::new(&mut self.lpf_cutoff_hz, 1_000.0..=max_cutoff).suffix(" Hz")).changed() {
                                self.process_samples();
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.squelch_enabled, "Activar Squelch / Supresor de Ruido").changed() {
                            self.process_samples();
                        }
                        if self.squelch_enabled {
                            ui.label("Umbral de Silencio:");
                            if ui.add(egui::Slider::new(&mut self.squelch_threshold_db, -80.0..=-20.0).suffix(" dBFS")).changed() {
                                self.process_samples();
                            }
                        }
                    });
                },
            );

            ui.add_space(6.0);
            accordion_section(
                ui,
                theme,
                "generic_iq_params_balance",
                AccordionOptions::new("3. Corrección de Hardware y Balance I/Q").default_open(false),
                |ui| {
                    if ui.checkbox(&mut self.iq_balance_correction, "Corrección Automática de Desbalance de Ganancia y Ortogonalidad I/Q").changed() {
                        self.process_samples();
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.agc_enabled, "Control Automático de Ganancia (AGC)").changed() {
                            self.process_samples();
                        }
                        if self.agc_enabled {
                            ui.label("Nivel Objetivo:");
                            if ui.add(egui::Slider::new(&mut self.agc_target_level, 0.2..=1.0)).changed() {
                                self.process_samples();
                            }
                        }
                    });
                },
            );

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                let apply_btn = egui::Button::new(
                    RichText::new("Aplicar Procesamiento").strong().color(Color32::WHITE),
                )
                .fill(theme.accent_blue())
                .corner_radius(6.0);

                if ui.add(apply_btn).clicked() {
                    self.process_samples();
                }

                if !self.processed_iq_samples.is_empty() {
                    if ui.button(RichText::new("Continuar a Efectos de Canal ->").strong()).clicked() {
                        self.current_step = 2;
                    }
                }
            });

            ui.add_space(6.0);
            ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));

            let active_samples = if !self.processed_iq_samples.is_empty() {
                &self.processed_iq_samples
            } else {
                &self.raw_iq_samples
            };

            if !active_samples.is_empty() {
                ui.add_space(10.0);
                accordion_section(
                    ui,
                    theme,
                    "generic_iq_osc",
                    AccordionOptions::new("Osciloscopio I/Q (Dominio del Tiempo)").default_open(false),
                    |ui| {
                        crate::ui::spectrum::render_signal_visualizer(
                            ui,
                            theme,
                            active_samples,
                            self.sample_rate,
                        );
                    },
                );

                ui.add_space(8.0);
                accordion_section(
                    ui,
                    theme,
                    "generic_iq_spec",
                    AccordionOptions::new("Espectrograma / Cascada RF").default_open(false),
                    |ui| {
                        let out_dir = config.get_pipeline_dir("generic_iq");
                        crate::ui::spectrogram::render_spectrogram_widget(
                            ui,
                            theme,
                            &mut self.spectrogram_state,
                            active_samples,
                            self.sample_rate,
                            "generic_iq",
                            Some(&out_dir),
                        );
                    },
                );
            }
        });
    }

    fn load_selected_file(&mut self) {
        if let Some(path) = &self.input_file_path {
            match read_iq_file(path, self.input_format) {
                Ok((samples, detected_sr)) => {
                    self.sample_rate = detected_sr;
                    let count = samples.len();
                    self.raw_iq_samples = samples;
                    self.process_samples();
                    self.status_message = format!(
                        "Archivo cargado exitosamente: {} muestras a {} Hz ({:.2} seg).",
                        count,
                        self.sample_rate,
                        count as f32 / self.sample_rate as f32
                    );
                }
                Err(e) => {
                    self.status_message = format!("Error al cargar archivo I/Q: {}", e);
                }
            }
        }
    }

    fn generate_synthetic_iq(&mut self) {
        let sr = 288_000;
        let count = sr * 2; // 2 segundos
        let mut samples = Vec::with_capacity(count as usize);

        for n in 0..count {
            let t = n as f32 / sr as f32;
            let phase = 2.0 * std::f32::consts::PI * 10_000.0 * t; // Tono a +10 kHz
            let i = phase.cos() * 0.7;
            let q = phase.sin() * 0.7;
            samples.push((i, q));
        }

        self.sample_rate = sr;
        self.input_file_path = Some(PathBuf::from("Sintético (Portadora compleja +10 kHz)"));
        self.raw_iq_samples = samples;
        self.process_samples();
        self.status_message = "Generadas 576,000 muestras I/Q de tono sintético a +10 kHz.".to_string();
    }

    fn process_samples(&mut self) {
        if self.raw_iq_samples.is_empty() {
            self.processed_iq_samples.clear();
            return;
        }

        let total = self.raw_iq_samples.len();

        // 1. Cancelación de DC Offset
        let mut dc_i = 0.0f32;
        let mut dc_q = 0.0f32;
        if self.remove_dc {
            let count = total as f32;
            let sum_i: f32 = self.raw_iq_samples.iter().map(|s| s.0).sum();
            let sum_q: f32 = self.raw_iq_samples.iter().map(|s| s.1).sum();
            dc_i = sum_i / count;
            dc_q = sum_q / count;
        }

        // 2. Extracción base con DC offset removido
        let mut samples: Vec<(f32, f32)> = self.raw_iq_samples
            .iter()
            .map(|&(r_i, r_q)| (r_i - dc_i, r_q - dc_q))
            .collect();

        // 3. Inversión Espectral (Invert I/Q / Conjugate)
        if self.invert_iq {
            for s in &mut samples {
                s.1 = -s.1;
            }
        }

        // 4. Corrección de Desbalance de Ganancia y Ortogonalidad I/Q
        if self.iq_balance_correction && total > 64 {
            let p_i: f32 = samples.iter().map(|s| s.0 * s.0).sum::<f32>() / (total as f32);
            let p_q: f32 = samples.iter().map(|s| s.1 * s.1).sum::<f32>() / (total as f32);
            let p_iq: f32 = samples.iter().map(|s| s.0 * s.1).sum::<f32>() / (total as f32);

            if p_i > 1e-9 && p_q > 1e-9 {
                let gain_ratio = (p_i / p_q).sqrt();
                let sin_phi = (p_iq / (p_i * p_q).sqrt()).clamp(-0.95, 0.95);
                let cos_phi = (1.0 - sin_phi * sin_phi).sqrt().max(0.01);

                for s in &mut samples {
                    let i = s.0;
                    let q = s.1;
                    // Corrección ortogonal Gram-Schmidt con compensación de ganancia
                    let q_ortho = (q - i * sin_phi) / cos_phi * gain_ratio;
                    s.1 = q_ortho;
                }
            }
        }

        // 5. Filtro FIR Pasa-Bajos (Denoising fuera de banda)
        if self.lpf_enabled {
            let taps = design_fir_lpf(self.lpf_cutoff_hz, self.sample_rate as f32, 31);
            apply_fir_filter(&mut samples, &taps);
        }

        // 6. Desplazamiento de Frecuencia (Frequency Shift)
        if self.freq_shift_hz.abs() > 0.01 {
            let shift_angular = 2.0 * std::f32::consts::PI * self.freq_shift_hz / (self.sample_rate as f32);
            for (idx, s) in samples.iter_mut().enumerate() {
                let angle = shift_angular * (idx as f32);
                let (sin_val, cos_val) = angle.sin_cos();
                let rot_i = s.0 * cos_val - s.1 * sin_val;
                let rot_q = s.0 * sin_val + s.1 * cos_val;
                s.0 = rot_i;
                s.1 = rot_q;
            }
        }

        // 7. Supresor de Ruido / Squelch
        if self.squelch_enabled {
            let thresh = 10.0f32.powf(self.squelch_threshold_db / 20.0);
            let thresh_sq = thresh * thresh;
            for s in &mut samples {
                let mag_sq = s.0 * s.0 + s.1 * s.1;
                if mag_sq < thresh_sq {
                    s.0 = 0.0;
                    s.1 = 0.0;
                }
            }
        }

        // 8. Control Automático de Ganancia (AGC) o Factor de Escala Manual
        if self.agc_enabled {
            let max_mag = samples.iter().map(|s| (s.0 * s.0 + s.1 * s.1).sqrt()).fold(0.0f32, f32::max);
            if max_mag > 1e-6 {
                let scale = (self.agc_target_level / max_mag).clamp(0.01, 100.0);
                for s in &mut samples {
                    s.0 = (s.0 * scale).clamp(-1.0, 1.0);
                    s.1 = (s.1 * scale).clamp(-1.0, 1.0);
                }
            }
        } else {
            for s in &mut samples {
                s.0 = (s.0 * self.gain_factor).clamp(-1.0, 1.0);
                s.1 = (s.1 * self.gain_factor).clamp(-1.0, 1.0);
            }
        }

        self.processed_iq_samples = samples;
        self.channel_effects_state.is_dirty = true;
        self.channel_effects_state.processed_iq.clear();

        let mut flags = Vec::new();
        if self.remove_dc { flags.push("DC"); }
        if self.invert_iq { flags.push("Invert-Q"); }
        if self.iq_balance_correction { flags.push("I/Q-Balance"); }
        if self.lpf_enabled { flags.push("FIR-LPF"); }
        if self.freq_shift_hz.abs() > 0.01 { flags.push("Shift"); }
        if self.squelch_enabled { flags.push("Squelch"); }
        if self.agc_enabled { flags.push("AGC"); } else { flags.push("Gain"); }

        self.status_message = format!(
            "Procesamiento aplicado [{}]: {} muestras listas a {} Hz.",
            flags.join(", "),
            self.processed_iq_samples.len(),
            self.sample_rate
        );
    }
}

/// Diseña coeficientes de un filtro FIR Pasa-Bajos simétrico con ventana de Hann
fn design_fir_lpf(cutoff_hz: f32, sample_rate: f32, num_taps: usize) -> Vec<f32> {
    let mut taps = vec![0.0f32; num_taps];
    let fc = (cutoff_hz / sample_rate).clamp(0.001, 0.499);
    let m = (num_taps - 1) as f32 / 2.0;

    for k in 0..num_taps {
        let n = k as f32 - m;
        let sinc = if n.abs() < 1e-6 {
            2.0 * fc
        } else {
            (2.0 * std::f32::consts::PI * fc * n).sin() / (std::f32::consts::PI * n)
        };
        // Ventana de Hann
        let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * k as f32 / (num_taps - 1) as f32).cos());
        taps[k] = sinc * window;
    }

    let sum: f32 = taps.iter().sum();
    if sum.abs() > 1e-6 {
        for t in &mut taps {
            *t /= sum;
        }
    }
    taps
}

/// Aplica convolución FIR a las muestras I y Q
fn apply_fir_filter(samples: &mut [(f32, f32)], taps: &[f32]) {
    if taps.is_empty() || samples.is_empty() {
        return;
    }
    let n_taps = taps.len();
    let half_taps = n_taps / 2;
    let n = samples.len();
    let mut filtered = Vec::with_capacity(n);

    for i in 0..n {
        let mut acc_i = 0.0f32;
        let mut acc_q = 0.0f32;
        for (k, &tap) in taps.iter().enumerate() {
            let sample_idx = (i + n + k).saturating_sub(half_taps) % n;
            acc_i += samples[sample_idx].0 * tap;
            acc_q += samples[sample_idx].1 * tap;
        }
        filtered.push((acc_i, acc_q));
    }
    samples.copy_from_slice(&filtered);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_iq_processing_pipeline() {
        let mut pipeline = GenericIqPipeline::default();
        pipeline.generate_synthetic_iq();
        assert_eq!(pipeline.raw_iq_samples.len(), 576_000);
        assert!(!pipeline.processed_iq_samples.is_empty());

        // Probar inversión de I/Q
        pipeline.invert_iq = true;
        pipeline.process_samples();
        assert!(pipeline.status_message.contains("Invert-Q"));

        // Probar filtro FIR pasa-bajos
        pipeline.lpf_enabled = true;
        pipeline.lpf_cutoff_hz = 50_000.0;
        pipeline.process_samples();
        assert!(pipeline.status_message.contains("FIR-LPF"));

        // Probar AGC
        pipeline.agc_enabled = true;
        pipeline.process_samples();
        assert!(pipeline.status_message.contains("AGC"));
    }
}
