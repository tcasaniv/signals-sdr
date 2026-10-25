use crate::channel::ChannelEffectsState;
use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::audio_player::AudioPlayerWidget;
use crate::ui::channel_effects::render_channel_effects_tab;
use crate::ui::components::{accordion_section, info_card, step_bar, AccordionOptions};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use am_modulator::{
    am_modulate_iq, load_audio_file, AmMode, AudioData,
};
use egui::{Color32, Frame, RichText, Stroke, Ui};

use std::sync::{Arc, Mutex};

pub struct GenericAmPipeline {
    pub current_step: usize,
    pub audio_path: Option<String>,
    pub audio_data: Option<AudioData>,
    pub audio_player: AudioPlayerWidget,
    pub async_audio_receiver: Option<Arc<Mutex<Option<(String, Vec<u8>)>>>>,
    // Modulación
    pub am_mode: AmMode,
    pub modulation_index: f32,
    pub if_offset_hz: f32,
    pub sdr_sample_rate: u32,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    pub channel_effects_state: ChannelEffectsState,
    pub transmission_state: TransmissionTabState,
}

impl Default for GenericAmPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            audio_path: None,
            audio_data: None,
            audio_player: AudioPlayerWidget::new(),
            async_audio_receiver: None,
            am_mode: AmMode::DsbFc,
            modulation_index: 0.85,
            if_offset_hz: 0.0,
            sdr_sample_rate: 288_000,
            iq_samples: Vec::new(),
            status_message: "Listo para cargar archivo de audio o generar tono sintético.".to_string(),
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            channel_effects_state: ChannelEffectsState::new_for_pipeline("generic_am", 118.1),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl GenericAmPipeline {
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
                let mut next_step = false;
                render_channel_effects_tab(
                    ui,
                    theme,
                    config,
                    "generic_am",
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
                let default_freq = match self.am_mode {
                    AmMode::DsbFc => 118_100_000.0, // 118.1 MHz Torre de Aviación AM
                    AmMode::DsbSc => 14_200_000.0,  // 14.2 MHz HF
                    AmMode::SsbUsb => 14_230_000.0, // 14.230 MHz HF USB
                    AmMode::SsbLsb => 7_150_000.0,   // 7.150 MHz 40m LSB
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
                    "generic_am",
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
        // Sondeo de archivo de audio cargado asíncronamente
        let loaded_async = if let Some(ref rx) = self.async_audio_receiver {
            rx.lock().ok().and_then(|mut guard| guard.take())
        } else {
            None
        };
        if let Some((name, bytes)) = loaded_async {
            self.async_audio_receiver = None;
            self.load_audio_bytes(&name, bytes);
        }

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

                        ui.horizontal_wrapped(|ui| {
                            let pick_label = format!(
                                "{} Examinar Audio (WAV, MP3)...",
                                crate::icons::ICON_FOLDER_OPEN.codepoint
                            );
                            let pick_btn = egui::Button::new(RichText::new(pick_label).size(12.5))
                                .fill(theme.surface_subtle())
                                .stroke(Stroke::new(1.0_f32, theme.card_border()));

                            if ui.add(pick_btn).clicked() {
                                let slot = Arc::new(Mutex::new(None));
                                self.async_audio_receiver = Some(slot.clone());
                                crate::file_dialog::pick_file_async(
                                    "Archivos de audio",
                                    &["wav", "mp3", "flac", "ogg"],
                                    move |name, bytes| {
                                        if let Ok(mut guard) = slot.lock() {
                                            *guard = Some((name, bytes));
                                        }
                                    },
                                );
                            }

                            let synth_label = format!(
                                "{} Generar Tono 1 kHz de Prueba",
                                crate::icons::ICON_TUNE.codepoint
                            );
                            if ui.button(RichText::new(synth_label).size(12.5)).clicked() {
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
                let next_label = format!(
                    "Continuar a Parámetros de Modulación {}",
                    crate::icons::ICON_ARROW_FORWARD.codepoint
                );
                if ui
                    .button(RichText::new(next_label).strong())
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
                "am_params_phase",
                AccordionOptions::new("Parámetros del Modulador AM / SSB").default_open(false),
                |ui| {
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
                    ui.horizontal_wrapped(|ui| {
                        let mod_btn = egui::Button::new(
                            RichText::new("Modular Señal AM").strong().color(Color32::WHITE),
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
                    "am_iq_osc",
                    AccordionOptions::new("Osciloscopio I/Q (Dominio del Tiempo)").default_open(false),
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
                    "am_iq_spec",
                    AccordionOptions::new("Espectrograma / Cascada RF").default_open(false),
                    |ui| {
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
                    },
                );
            }
        });
    }

    pub fn load_audio_bytes(&mut self, filename: &str, bytes: Vec<u8>) {
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str());
        match crate::utils::load_audio_from_memory(&bytes, ext) {
            Ok(data) => {
                let duration = data.samples.len() as f32 / data.sample_rate as f32;
                self.audio_player.load_bytes(filename, bytes);
                self.audio_player.total_duration_sec = duration;
                self.audio_path = Some(filename.to_string());
                self.status_message = format!("Audio cargado exitosamente: {:.1} seg, {} muestras.", duration, data.samples.len());
                self.audio_data = Some(AudioData {
                    samples: data.samples,
                    sample_rate: data.sample_rate,
                    original_channels: data.original_channels,
                });
            }
            Err(e) => {
                self.status_message = format!("Error al decodificar audio: {}", e);
            }
        }
    }

    #[allow(dead_code)]
    fn load_audio(&mut self, path: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(bytes) = std::fs::read(path) {
            self.load_audio_bytes(path, bytes);
            return;
        }

        match load_audio_file(path) {
            Ok(data) => {
                let duration = data.samples.len() as f32 / data.sample_rate as f32;
                self.audio_player.load_path(path);
                self.audio_player.total_duration_sec = duration;
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

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = std::io::Cursor::new(Vec::new());
        if let Ok(mut writer) = hound::WavWriter::new(&mut cursor, spec) {
            for &s in &i16_samples {
                let _ = writer.write_sample(s);
            }
            let _ = writer.finalize();
        }
        let wav_bytes = cursor.into_inner();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let out_dir = config.get_pipeline_dir("generic_am");
            let wav_path = out_dir.join("tono_1khz_am.wav");
            let _ = std::fs::write(&wav_path, &wav_bytes);
            self.audio_path = Some(wav_path.to_string_lossy().to_string());
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = config;
            self.audio_path = Some("tono_1khz_am.wav".to_string());
        }

        self.audio_player.load_bytes("tono_1khz_am.wav", wav_bytes);
        self.audio_player.total_duration_sec = duration_sec;
        self.audio_player.current_time_sec = 0.0;
        self.status_message = "Generado tono sintético sinusoidal de 1000 Hz (3 segundos).".to_string();
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

        self.channel_effects_state.is_dirty = true;
        self.channel_effects_state.processed_iq.clear();

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
