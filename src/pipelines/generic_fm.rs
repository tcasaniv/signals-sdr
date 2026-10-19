use crate::channel::ChannelEffectsState;
use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::audio_player::AudioPlayerWidget;
use crate::ui::channel_effects::render_channel_effects_tab;
use crate::ui::components::{accordion_section, info_card, step_bar, AccordionOptions};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui};
use fm_modulator::{
    apply_pre_emphasis, fm_modulate, load_audio_file, AudioData, FmPreset, PreEmphasisMode,
};

pub struct GenericFmPipeline {
    pub current_step: usize,
    pub audio_path: Option<String>,
    pub audio_data: Option<AudioData>,
    pub audio_player: AudioPlayerWidget,
    // Modulación
    pub preset: FmPreset,
    pub custom_deviation: f32,
    pub pre_emphasis_mode: PreEmphasisMode,
    pub if_offset_hz: f32,
    pub sdr_sample_rate: u32,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    pub channel_effects_state: ChannelEffectsState,
    pub transmission_state: TransmissionTabState,
}

impl Default for GenericFmPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            audio_path: None,
            audio_data: None,
            audio_player: AudioPlayerWidget::new(),
            preset: FmPreset::BroadcastWfm,
            custom_deviation: 75_000.0,
            pre_emphasis_mode: PreEmphasisMode::Eu50us,
            if_offset_hz: 0.0,
            sdr_sample_rate: 288_000,
            iq_samples: Vec::new(),
            status_message: "Listo para cargar archivo de audio (WAV / MP3) o generar tono sintético.".to_string(),
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            channel_effects_state: ChannelEffectsState::new_for_pipeline("generic_fm", 100.1),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl GenericFmPipeline {
    pub fn get_effective_iq(&self) -> &[(f32, f32)] {
        crate::channel::select_effective_iq(
            self.channel_effects_state.active_tab,
            &self.iq_samples,
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
            "Generic FM (Modulador de Frecuencia Analógico)",
            "Modula cualquier señal de audio (WAV, MP3) en Frecuencia Modulada (FM) en banda ancha (Broadcast WFM 75 kHz), \
             banda estrecha (NFM 5 kHz / 2.5 kHz para VHF/UHF) o subportadora NOAA APT (17 kHz), con filtrado de \
             pre-énfasis estándar (50 µs Europa / 75 µs América) y desplazamiento IF opcional.",
        );

        ui.add_space(8.0);
        step_bar(ui, theme, &mut self.current_step, "2. Modular");

        match self.current_step {
            0 => self.render_step_source(ui, theme, config),
            1 => self.render_step_modulate(ui, theme, config),
            2 => {
                let default_freq = match self.preset {
                    FmPreset::BroadcastWfm => 100_100_000.0,
                    FmPreset::NarrowbandNfm => 144_800_000.0,
                    FmPreset::NarrowbandNfm2k5 => 446_006_250.0,
                    FmPreset::NoaaApt => 137_100_000.0,
                };
                let mut next_step = false;
                render_channel_effects_tab(
                    ui,
                    theme,
                    config,
                    "generic_fm",
                    self.sdr_sample_rate,
                    default_freq,
                    &self.iq_samples,
                    &mut self.channel_effects_state,
                    || next_step = true,
                );
                if next_step {
                    self.current_step = 3;
                }
            }
            3 => {
                let default_freq = match self.preset {
                    FmPreset::BroadcastWfm => 100_100_000.0, // 100.1 MHz comercial
                    FmPreset::NarrowbandNfm => 144_800_000.0, // 144.800 MHz radioaficionados
                    FmPreset::NarrowbandNfm2k5 => 446_006_250.0, // 446.00625 MHz PMR446 Ch 1
                    FmPreset::NoaaApt => 137_100_000.0,      // 137.1 MHz satélite NOAA
                };

                let effective_iq = crate::channel::select_effective_iq(
                    self.channel_effects_state.active_tab,
                    &self.iq_samples,
                    &self.channel_effects_state.processed_iq,
                );

                render_transmission_tab(
                    ui,
                    theme,
                    config,
                    "generic_fm",
                    default_freq,
                    &mut self.transmission_state,
                    sdr_controller,
                    effective_iq,
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
            accordion_section(
                ui,
                theme,
                "fm_params_phase",
                AccordionOptions::new("Parámetros del Modulador FM").default_open(true),
                |ui| {
                    // Presets
                    ui.horizontal(|ui| {
                        ui.label("Perfil de Modulación:");
                        let current_preset_name = match self.preset {
                            FmPreset::BroadcastWfm => "Broadcast Comercial WFM (75 kHz dev, 50 us pre-énfasis)",
                            FmPreset::NarrowbandNfm => "VHF/UHF NFM Estándar (5 kHz dev, Canales 25 kHz)",
                            FmPreset::NarrowbandNfm2k5 => "PMR446 / NFM Estrecho (2.5 kHz dev, Canales 12.5 kHz)",
                            FmPreset::NoaaApt => "Satélite Meteorológico NOAA APT (17 kHz dev)",
                        };

                        egui::ComboBox::from_id_salt("fm_preset_combo")
                            .selected_text(current_preset_name)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_value(
                                        &mut self.preset,
                                        FmPreset::BroadcastWfm,
                                        "Broadcast Comercial WFM (75 kHz dev, 50 us)",
                                    )
                                    .clicked()
                                {
                                    self.custom_deviation = 75_000.0;
                                    self.pre_emphasis_mode = PreEmphasisMode::Eu50us;
                                }

                                if ui
                                    .selectable_value(
                                        &mut self.preset,
                                        FmPreset::NarrowbandNfm,
                                        "VHF/UHF NFM Estándar (5 kHz dev, Canales 25 kHz)",
                                    )
                                    .clicked()
                                {
                                    self.custom_deviation = 5_000.0;
                                    self.pre_emphasis_mode = PreEmphasisMode::None;
                                }

                                if ui
                                    .selectable_value(
                                        &mut self.preset,
                                        FmPreset::NarrowbandNfm2k5,
                                        "PMR446 / NFM Estrecho (2.5 kHz dev, Canales 12.5 kHz)",
                                    )
                                    .clicked()
                                {
                                    self.custom_deviation = 2_500.0;
                                    self.pre_emphasis_mode = PreEmphasisMode::None;
                                }

                                if ui
                                    .selectable_value(
                                        &mut self.preset,
                                        FmPreset::NoaaApt,
                                        "Satélite NOAA APT (17 kHz dev)",
                                    )
                                    .clicked()
                                {
                                    self.custom_deviation = 17_000.0;
                                    self.pre_emphasis_mode = PreEmphasisMode::None;
                                }
                            });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Desviación Máxima de Frecuencia (Delta f):");
                        ui.add(egui::Slider::new(&mut self.custom_deviation, 1_000.0..=100_000.0).suffix(" Hz"));
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Filtro de Pre-énfasis:");
                        let pre_name = match self.pre_emphasis_mode {
                            PreEmphasisMode::None => "Desactivado (Lineal)",
                            PreEmphasisMode::Eu50us => "50 µs (Europa / ITU)",
                            PreEmphasisMode::Us75us => "75 µs (EE.UU. / FCC)",
                        };
                        egui::ComboBox::from_id_salt("fm_preemph_combo")
                            .selected_text(pre_name)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.pre_emphasis_mode, PreEmphasisMode::None, "Desactivado");
                                ui.selectable_value(&mut self.pre_emphasis_mode, PreEmphasisMode::Eu50us, "50 µs (Europa/ITU)");
                                ui.selectable_value(&mut self.pre_emphasis_mode, PreEmphasisMode::Us75us, "75 µs (EE.UU./FCC)");
                            });
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Desplazamiento IF Portadora (Offset Hz):");
                        ui.add(egui::Slider::new(&mut self.if_offset_hz, -50_000.0..=50_000.0).suffix(" Hz"));
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Tasa de Muestreo SDR I/Q:");
                        egui::ComboBox::from_id_salt("fm_sdr_rate_combo")
                            .selected_text(format!("{} Sps ({} kHz)", self.sdr_sample_rate, self.sdr_sample_rate / 1000))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.sdr_sample_rate, 100_000, "100 kHz");
                                ui.selectable_value(&mut self.sdr_sample_rate, 250_000, "250 kHz");
                                ui.selectable_value(&mut self.sdr_sample_rate, 288_000, "288 kHz");
                                ui.selectable_value(&mut self.sdr_sample_rate, 500_000, "500 kHz");
                                ui.selectable_value(&mut self.sdr_sample_rate, 1_000_000, "1.0 MHz");
                                ui.selectable_value(&mut self.sdr_sample_rate, 2_000_000, "2.0 MHz");
                                ui.selectable_value(&mut self.sdr_sample_rate, 2_400_000, "2.4 MHz");
                                ui.selectable_value(&mut self.sdr_sample_rate, 8_000_000, "8.0 MHz");
                            });
                    });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let mod_btn = egui::Button::new(
                            RichText::new("Modular Señal FM").strong().color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(6.0);

                        if ui.add(mod_btn).clicked() {
                            self.execute_modulation();
                        }

                        if !self.iq_samples.is_empty() {
                            if ui.button(RichText::new("Continuar a Efectos de Canal ->").strong()).clicked() {
                                self.current_step = 2;
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                },
            );

            if !self.iq_samples.is_empty() {
                ui.add_space(10.0);
                accordion_section(
                    ui,
                    theme,
                    "fm_iq_osc",
                    AccordionOptions::new("Osciloscopio I/Q (Dominio del Tiempo)").default_open(true),
                    |ui| {
                        crate::ui::spectrum::render_signal_visualizer(
                            ui,
                            theme,
                            &self.iq_samples,
                            self.sdr_sample_rate,
                        );
                    },
                );

                ui.add_space(8.0);
                accordion_section(
                    ui,
                    theme,
                    "fm_iq_spec",
                    AccordionOptions::new("Espectrograma y Cascada de Radiofrecuencia (FFT en Vivo)").default_open(true),
                    |ui| {
                        let out_dir = config.get_pipeline_dir("generic_fm");
                        crate::ui::spectrogram::render_spectrogram_widget(
                            ui,
                            theme,
                            &mut self.spectrogram_state,
                            &self.iq_samples,
                            self.sdr_sample_rate,
                            "generic_fm",
                            Some(&out_dir),
                        );
                    },
                );
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

        let out_dir = config.get_pipeline_dir("generic_fm");
        let wav_path = out_dir.join("tono_1khz.wav");
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
            self.audio_path = Some("tono_1khz.wav".to_string());
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

        // 1. Aplicar pre-énfasis si está configurado
        let pre_emphasized = apply_pre_emphasis(&audio.samples, audio.sample_rate, self.pre_emphasis_mode);

        // 2. Modular en FM a la tasa deseada
        self.iq_samples = fm_modulate(
            &pre_emphasized,
            audio.sample_rate,
            self.sdr_sample_rate,
            self.custom_deviation,
            self.if_offset_hz,
        );

        self.channel_effects_state.is_dirty = true;
        self.channel_effects_state.processed_iq.clear();

        self.status_message = format!(
            "Modulación FM completada: {} muestras I/Q generadas a {} Sps (Delta f = {:.1} kHz).",
            self.iq_samples.len(),
            self.sdr_sample_rate,
            self.custom_deviation / 1000.0
        );
    }
}
