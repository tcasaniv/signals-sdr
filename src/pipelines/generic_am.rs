use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::audio_player::AudioPlayerWidget;
use crate::ui::components::{info_card, step_bar};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use am_modulator::{
    am_modulate_iq, load_audio_file, AmMode, AudioData,
};
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};

pub struct GenericAmPipeline {
    pub current_step: usize,
    pub audio_path: Option<String>,
    pub audio_data: Option<AudioData>,
    pub audio_player: AudioPlayerWidget,
    // Modulación
    pub am_mode: AmMode,
    pub modulation_index: f32,
    pub if_offset_hz: f32,
    pub sdr_sample_rate: u32,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    pub transmission_state: TransmissionTabState,
}

impl Default for GenericAmPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            audio_path: None,
            audio_data: None,
            audio_player: AudioPlayerWidget::new(),
            am_mode: AmMode::DsbFc,
            modulation_index: 0.85,
            if_offset_hz: 0.0,
            sdr_sample_rate: 288_000,
            iq_samples: Vec::new(),
            status_message: "Listo para cargar archivo de audio o generar tono sintético.".to_string(),
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl GenericAmPipeline {
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
            "Generic AM (Modulador de Amplitud Analógico)",
            "Modula cualquier audio (WAV, MP3) en Amplitud Modulada (AM) con soporte completo para Doble Banda Lateral \
             con Portadora Completa (DSB-FC comercial / aviación VHF 118-137 MHz), Portadora Suprimida (DSB-SC) \
             y Banda Lateral Única (SSB USB / LSB para HF radioaficionados) con transformada de Hilbert analítica.",
        );

        ui.add_space(8.0);
        step_bar(ui, theme, &mut self.current_step, "2. Modular");

        match self.current_step {
            0 => self.render_step_source(ui, theme, config),
            1 => self.render_step_modulate(ui, theme, config),
            2 => {
                let default_freq = match self.am_mode {
                    AmMode::DsbFc => 118_100_000.0, // 118.1 MHz Torre de Aviación AM
                    AmMode::DsbSc => 14_200_000.0,  // 14.2 MHz HF
                    AmMode::SsbUsb => 14_230_000.0, // 14.230 MHz HF USB
                    AmMode::SsbLsb => 7_150_000.0,   // 7.150 MHz 40m LSB
                };

                render_transmission_tab(
                    ui,
                    theme,
                    config,
                    "generic_am",
                    default_freq,
                    &mut self.transmission_state,
                    sdr_controller,
                    &self.iq_samples,
                    self.sdr_sample_rate,
                );
            }
            _ => {}
        }
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme, config: &AppConfig) {
        ui.vertical(|ui| {
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Seleccionar Fuente de Audio").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            let pick_btn = egui::Button::new("Examinar Audio (WAV, MP3)...")
                                .fill(theme.surface_subtle())
                                .stroke(Stroke::new(1.0_f32, theme.card_border()));

                            if ui.add(pick_btn).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Archivos de audio", &["wav", "mp3", "flac", "ogg"])
                                    .pick_file()
                                {
                                    let path_str = path.display().to_string();
                                    self.load_audio(&path_str);
                                }
                            }

                            if ui.button("Generar Tono 1 kHz de Prueba").clicked() {
                                self.generate_test_tone(config);
                            }
                        });

                        ui.add_space(8.0);
                        self.audio_player.render(ui, theme, self.audio_path.as_deref());

                        if let Some(audio) = &self.audio_data {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!(
                                    "Tasa de muestreo original: {} Hz | Canales: {} | Muestras cargadas: {}",
                                    audio.sample_rate, audio.original_channels, audio.samples.len()
                                ))
                                .size(12.0)
                                .color(theme.accent_green()),
                            );

                            ui.add_space(8.0);
                            crate::ui::spectrum::render_audio_waveform(
                                ui,
                                theme,
                                &audio.samples,
                                audio.sample_rate,
                                "Osciloscopio de Audio (Dominio del Tiempo)",
                            );
                        }
                    });
                });

            ui.add_space(10.0);
            if self.audio_data.is_some() {
                if ui
                    .button(RichText::new("Continuar a Parámetros de Modulación ->").strong())
                    .clicked()
                {
                    self.current_step = 1;
                }
            } else {
                ui.label(
                    RichText::new("Cargue un archivo de audio o genere un tono sintético para continuar.")
                        .size(12.0)
                        .color(theme.text_secondary()),
                );
            }
        });
    }

    fn render_step_modulate(&mut self, ui: &mut Ui, theme: &AppTheme, config: &AppConfig) {
        ui.vertical(|ui| {
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Parámetros del Modulador AM").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label("Tipo de Modulación AM:");
                            let am_mode_name = match self.am_mode {
                                AmMode::DsbFc => "DSB-FC (Portadora Completa - Broadcast / Aviación)",
                                AmMode::DsbSc => "DSB-SC (Portadora Suprimida)",
                                AmMode::SsbUsb => "SSB USB (Banda Lateral Superior)",
                                AmMode::SsbLsb => "SSB LSB (Banda Lateral Inferior)",
                            };

                            egui::ComboBox::from_id_salt("am_mode_combo")
                                .selected_text(am_mode_name)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.am_mode, AmMode::DsbFc, "DSB-FC (Portadora Completa)");
                                    ui.selectable_value(&mut self.am_mode, AmMode::DsbSc, "DSB-SC (Portadora Suprimida)");
                                    ui.selectable_value(&mut self.am_mode, AmMode::SsbUsb, "SSB USB (Banda Lateral Superior)");
                                    ui.selectable_value(&mut self.am_mode, AmMode::SsbLsb, "SSB LSB (Banda Lateral Inferior)");
                                });
                        });

                        ui.add_space(6.0);
                        if self.am_mode == AmMode::DsbFc {
                            ui.horizontal(|ui| {
                                ui.label("Índice de modulación (m):");
                                ui.add(egui::Slider::new(&mut self.modulation_index, 0.05..=1.5).text(" (1.0 = 100%)"));
                            });
                        }

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Desplazamiento IF Portadora (Offset Hz):");
                            ui.add(egui::Slider::new(&mut self.if_offset_hz, -50_000.0..=50_000.0).suffix(" Hz"));
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Tasa de Muestreo SDR I/Q:");
                            egui::ComboBox::from_id_salt("am_sdr_rate_combo")
                                .selected_text(format!("{} Sps ({} kHz)", self.sdr_sample_rate, self.sdr_sample_rate / 1000))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.sdr_sample_rate, 288_000, "288 kHz (Estándar)");
                                    ui.selectable_value(&mut self.sdr_sample_rate, 500_000, "500 kHz");
                                    ui.selectable_value(&mut self.sdr_sample_rate, 1_000_000, "1.0 MHz");
                                    ui.selectable_value(&mut self.sdr_sample_rate, 2_000_000, "2.0 MHz (HackRF/bladeRF)");
                                    ui.selectable_value(&mut self.sdr_sample_rate, 2_400_000, "2.4 MHz (RTL-SDR/HackRF)");
                                });
                        });

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let mod_btn = egui::Button::new(
                                RichText::new("Modular Señal AM").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(mod_btn).clicked() {
                                self.execute_modulation();
                            }

                            if !self.iq_samples.is_empty() {
                                if ui.button(RichText::new("Continuar a Transmisión ->").strong()).clicked() {
                                    self.current_step = 2;
                                }
                            }
                        });

                        ui.add_space(6.0);
                        ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                    });
                });

            if !self.iq_samples.is_empty() {
                ui.add_space(10.0);
                crate::ui::spectrum::render_signal_visualizer(
                    ui,
                    theme,
                    &self.iq_samples,
                    self.sdr_sample_rate,
                );

                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let spec_btn_text = if self.spectrogram_state.show {
                        "Ocultar Espectro"
                    } else {
                        "Mostrar Espectro"
                    };
                    let spec_btn = egui::Button::new(
                        RichText::new(spec_btn_text).strong().size(12.5),
                    )
                    .fill(if self.spectrogram_state.show { theme.card_border() } else { theme.surface_subtle() })
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .min_size(Vec2::new(0.0, 32.0));

                    if ui.add(spec_btn).clicked() {
                        self.spectrogram_state.show = !self.spectrogram_state.show;
                    }
                });

                if self.spectrogram_state.show {
                    ui.add_space(8.0);
                    let out_dir = config.get_pipeline_dir("generic_am");
                    crate::ui::spectrogram::render_spectrogram_widget(
                        ui,
                        theme,
                        &mut self.spectrogram_state,
                        &self.iq_samples,
                        self.sdr_sample_rate,
                        "generic_am",
                        Some(&out_dir),
                    );
                }
            }
        });
    }

    fn load_audio(&mut self, path: &str) {
        match load_audio_file(path) {
            Ok(data) => {
                let duration = data.samples.len() as f32 / data.sample_rate as f32;
                self.audio_player.total_duration_sec = duration;
                self.audio_player.current_time_sec = 0.0;
                self.audio_path = Some(path.to_string());
                self.status_message = format!("Audio cargado exitosamente: {:.1} seg, {} muestras.", duration, data.samples.len());
                self.audio_data = Some(data);
            }
            Err(e) => {
                self.status_message = format!("Error al cargar audio: {}", e);
            }
        }
    }

    fn generate_test_tone(&mut self, config: &AppConfig) {
        let sample_rate = 48_000;
        let duration_sec = 3.0;
        let num_samples = (sample_rate as f32 * duration_sec) as usize;
        let mut samples = Vec::with_capacity(num_samples);
        let mut i16_samples = Vec::with_capacity(num_samples);

        for n in 0..num_samples {
            let t = n as f32 / sample_rate as f32;
            let val = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.8;
            samples.push(val);
            i16_samples.push((val * 32767.0) as i16);
        }

        let out_dir = config.get_pipeline_dir("generic_am");
        let wav_path = out_dir.join("tono_1khz_am.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        if let Ok(mut writer) = hound::WavWriter::create(&wav_path, spec) {
            for &s in &i16_samples {
                let _ = writer.write_sample(s);
            }
            let _ = writer.finalize();
            self.audio_path = Some(wav_path.to_string_lossy().to_string());
        } else {
            self.audio_path = Some("tono_1khz_am.wav".to_string());
        }

        self.audio_player.total_duration_sec = duration_sec;
        self.audio_player.current_time_sec = 0.0;
        self.status_message = "Generado tono sintético sinusoidal de 1000 Hz (3 segundos, WAV guardado).".to_string();
        self.audio_data = Some(AudioData {
            samples,
            sample_rate,
            original_channels: 1,
        });
    }

    fn execute_modulation(&mut self) {
        let audio = match &self.audio_data {
            Some(a) => a,
            None => {
                self.status_message = "Debe cargar un archivo de audio primero.".to_string();
                return;
            }
        };

        self.iq_samples = am_modulate_iq(
            &audio.samples,
            audio.sample_rate,
            self.sdr_sample_rate,
            self.modulation_index,
            self.am_mode,
            self.if_offset_hz,
        );

        let mode_desc = match self.am_mode {
            AmMode::DsbFc => "DSB-FC",
            AmMode::DsbSc => "DSB-SC",
            AmMode::SsbUsb => "SSB-USB",
            AmMode::SsbLsb => "SSB-LSB",
        };

        self.status_message = format!(
            "Modulación AM ({}) completada: {} muestras I/Q generadas a {} Sps.",
            mode_desc,
            self.iq_samples.len(),
            self.sdr_sample_rate
        );
    }
}
