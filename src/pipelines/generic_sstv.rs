use crate::channel::ChannelEffectsState;
use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::channel_effects::render_channel_effects_tab;
use crate::ui::components::{accordion_section, image_input_card, info_card, step_bar, AccordionOptions};
use crate::ui::spectrum::render_audio_waveform_i16;
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};
use fm_modulator::fm_modulate;
use image::{DynamicImage, ImageReader, RgbImage};
use sstv_encoder::{encode_sstv, Mode};
use std::path::PathBuf;

pub struct GenericSstvPipeline {
    pub current_step: usize,
    pub img_path: Option<String>,
    pub url_input: String,
    pub selected_mode: Mode,
    pub fm_deviation_hz: f32,
    pub audio_samples: Option<Vec<i16>>,
    pub audio_wav_path: Option<PathBuf>,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    pub channel_effects_state: ChannelEffectsState,
    pub sdr_sample_rate: u32,
    pub transmission_state: TransmissionTabState,
}

impl Default for GenericSstvPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            img_path: None,
            url_input: String::new(),
            selected_mode: Mode::Martin1,
            fm_deviation_hz: 5_000.0,
            audio_samples: None,
            audio_wav_path: None,
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imagen para modulación SSTV multiprotocolo.".to_string(),
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            channel_effects_state: ChannelEffectsState::new_for_pipeline("generic_sstv", 14.23),
            sdr_sample_rate: 288_000,
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl GenericSstvPipeline {
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
        ui.vertical(|ui| {
            info_card(
                ui,
                theme,
                "Generic SSTV (Televisión de Barrido Lento Multiprotocolo)",
                "Generador de señales SSTV para todos los estándares analógicos de radioaficionados: Martin, Scottie, \
                 Robot, PD y B/W. La imagen se remuestrea a la resolución nativa del modo, se sintetizan los tonos de \
                 sincronismo (1200 Hz), porche (1500 Hz) y luminancia/crominancia (1500 - 2300 Hz) con cabecera VIS y finalmente se modulan en FM.",
            );

            ui.add_space(8.0);
            step_bar(ui, theme, &mut self.current_step, "2. Codificar");

            match self.current_step {
                0 => self.render_step_source(ui, theme),
                1 => self.render_step_encode(ui, theme, config),
                2 => {
                    let mut next_step = false;
                    render_channel_effects_tab(
                        ui,
                        theme,
                        config,
                        "generic_sstv",
                        self.sdr_sample_rate,
                        14_230_000.0,
                        &self.iq_samples,
                        &mut self.channel_effects_state,
                        || next_step = true,
                    );
                    if next_step {
                        self.current_step = 3;
                    }
                }
                3 => {
                    let effective_iq = crate::channel::select_effective_iq(
                        self.channel_effects_state.active_tab,
                        &self.iq_samples,
                        &self.channel_effects_state.processed_iq,
                    );

                    render_transmission_tab(
                        ui,
                        theme,
                        config,
                        "generic_sstv",
                        14_230_000.0, // 14.230 MHz (Frecuencia internacional SSTV en 20 metros USB/FM)
                        &mut self.transmission_state,
                        sdr_controller,
                        effective_iq,
                        self.sdr_sample_rate,
                    );
                }
                _ => {}
            }
        });
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            image_input_card(
                ui,
                theme,
                "Imagen Fuente para SSTV",
                &mut self.img_path,
                &mut self.url_input,
                crate::ui::components::ChannelColor::Blue,
            );

            ui.add_space(12.0);

            if let Some(path) = &self.img_path {
                Frame::new()
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .corner_radius(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Imagen seleccionada:").strong().color(theme.accent_blue()));
                        ui.label(RichText::new(path).monospace().size(12.0));
                    });

                ui.add_space(16.0);
                let next_btn = egui::Button::new(
                    RichText::new("Continuar a Selección de Modo ->")
                        .strong()
                        .size(13.5)
                        .color(Color32::WHITE),
                )
                .fill(theme.accent_blue())
                .corner_radius(8.0)
                .min_size(Vec2::new(260.0, 38.0));

                if ui.add(next_btn).clicked() {
                    self.current_step = 1;
                }
            } else {
                ui.label(
                    RichText::new("Selecciona una imagen o presiona Continuar para usar un patrón de prueba sintético.")
                        .size(12.0)
                        .color(theme.text_secondary()),
                );
                ui.add_space(10.0);

                let dummy_btn = egui::Button::new(
                    RichText::new("Continuar con patrón de prueba ->")
                        .strong()
                        .size(13.5)
                        .color(Color32::WHITE),
                )
                .fill(theme.accent_blue())
                .corner_radius(8.0)
                .min_size(Vec2::new(260.0, 38.0));

                if ui.add(dummy_btn).clicked() {
                    self.current_step = 1;
                }
            }
        });
    }

    fn mode_info_label(mode: Mode) -> (&'static str, &'static str, &'static str) {
        match mode {
            Mode::Martin1 => ("Martin 1", "320x256 RGB", "114 seg"),
            Mode::Martin2 => ("Martin 2", "320x256 RGB", "57 seg"),
            Mode::Scottie1 => ("Scottie 1", "320x256 RGB", "110 seg"),
            Mode::Scottie2 => ("Scottie 2", "320x256 RGB", "71 seg"),
            Mode::ScottieDx => ("Scottie DX", "320x256 RGB", "269 seg"),
            Mode::Robot36 => ("Robot 36", "320x240 YUV", "36 seg"),
            Mode::Robot72 => ("Robot 72", "320x240 YUV", "72 seg"),
            Mode::Pd120 => ("PD 120", "640x496 YUV", "126 seg"),
            Mode::Pd180 => ("PD 180", "640x496 YUV", "187 seg"),
            Mode::Pd290 => ("PD 290", "800x616 YUV", "289 seg"),
            Mode::Bw8 => ("BW 8", "160x120 Monocromo", "8 seg"),
            Mode::Bw12 => ("BW 12", "160x120 Monocromo", "12 seg"),
        }
    }

    fn render_step_encode(&mut self, ui: &mut Ui, theme: &AppTheme, config: &AppConfig) {
        let (current_name, current_res, current_dur) = Self::mode_info_label(self.selected_mode);

        ui.vertical(|ui| {
            // Cabecera superior didáctica
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(10.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Flujo Didáctico Paso a Paso Generic SSTV").strong().size(15.0).color(theme.accent_blue()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let auto_btn = egui::Button::new(
                                RichText::new("Ejecutar Todo el Pipeline en un Clic").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(8.0)
                            .min_size(Vec2::new(0.0, 32.0));

                            if ui.add(auto_btn).clicked() {
                                self.execute_pipeline(config);
                            }
                        });
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                });

            ui.add_space(12.0);

            // ==========================================
            // FASE 1: SELECCIÓN DEL MODO SSTV
            // ==========================================
            accordion_section(
                ui,
                theme,
                "generic_sstv_phase_1",
                AccordionOptions::new("Fase 1: Configuración del Modo SSTV")
                    .with_badge("CONFIGURADO", Color32::from_rgb(34, 197, 94))
                    .default_open(false),
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Modo de Transmisión:");
                        egui::ComboBox::from_id_salt("generic_sstv_mode_combo")
                            .selected_text(format!("{} ({} - {})", current_name, current_res, current_dur))
                            .show_ui(ui, |ui| {
                                let all_modes = [
                                    Mode::Martin1,
                                    Mode::Martin2,
                                    Mode::Scottie1,
                                    Mode::Scottie2,
                                    Mode::ScottieDx,
                                    Mode::Robot36,
                                    Mode::Robot72,
                                    Mode::Pd120,
                                    Mode::Pd180,
                                    Mode::Pd290,
                                    Mode::Bw8,
                                    Mode::Bw12,
                                ];

                                for mode in all_modes {
                                    let (name, res, dur) = Self::mode_info_label(mode);
                                    let label_text = format!("{} ({} - {})", name, res, dur);
                                    ui.selectable_value(&mut self.selected_mode, mode, label_text);
                                }
                            });
                    });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Resolución requerida: {} | Duración estimada de escaneo: {}", current_res, current_dur))
                            .size(11.5)
                            .color(theme.text_secondary()),
                    );
                },
            );

            ui.add_space(10.0);

            // ==========================================
            // FASE 2: CODIFICACIÓN ANALÓGICA SSTV A AUDIO
            // ==========================================
            let (badge_text_2, badge_color_2) = if self.audio_samples.is_some() {
                ("LISTO", Color32::from_rgb(34, 197, 94))
            } else {
                ("PENDIENTE", Color32::from_rgb(156, 163, 175))
            };

            accordion_section(
                ui,
                theme,
                "generic_sstv_phase_2",
                AccordionOptions::new("Fase 2: Estructura de la Señal SSTV (Audio a 48 kHz)")
                    .with_badge(badge_text_2, badge_color_2)
                    .default_open(false),
                |ui| {
                    ui.label(
                        RichText::new("Encabezado VIS: Tono de calibración a 1900 Hz (300 ms), Tono de ruptura a 1200 Hz (10 ms), Código VIS digital de 8 bits con bit de paridad. Líneas de imagen: Tono de sincro a 1200 Hz, Porche negro a 1500 Hz, y barrido analógico de luminancia/color (1500-2300 Hz). Se genera y guarda como archivo WAV a 48 kHz.")
                            .size(11.5)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        let btn = egui::Button::new(
                            RichText::new("2. Codificar Audio SSTV (Guardar WAV 48 kHz)").strong().color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add(btn).clicked() {
                            self.step_encode_audio(config);
                        }
                    });

                    if let Some(ref audio) = self.audio_samples {
                        ui.add_space(8.0);
                        let dur_sec = audio.len() as f32 / 48_000.0;
                        ui.label(
                            RichText::new(format!(
                                "Métricas de Audio SSTV: Tasa 48,000 Hz | Total {} muestras PCM | Duración estimada: {:.1} segundos",
                                audio.len(),
                                dur_sec
                            ))
                            .size(12.0)
                            .color(theme.accent_green()),
                        );

                        ui.add_space(8.0);
                        accordion_section(
                            ui,
                            theme,
                            "generic_sstv_audio_osc",
                            AccordionOptions::new("Osciloscopio de Audio SSTV (48 kHz)").default_open(false),
                            |ui| {
                                render_audio_waveform_i16(
                                    ui,
                                    theme,
                                    audio,
                                    48_000,
                                    "Osciloscopio de Audio SSTV (48 kHz)",
                                );
                            },
                        );

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            if let Some(ref path) = self.audio_wav_path {
                                let play_btn = egui::Button::new(RichText::new("Reproducir en Reproductor Predeterminado").strong().size(12.5))
                                    .min_size(Vec2::new(0.0, 32.0));
                                if ui.add(play_btn).clicked() {
                                    if let Some(p) = path.to_str() {
                                        crate::utils::open_path_or_url(p);
                                    }
                                }
                            }

                            let out_dir = config.get_pipeline_dir("generic_sstv");
                            let folder_btn = egui::Button::new(RichText::new("Abrir Carpeta de Salida").size(12.5))
                                .min_size(Vec2::new(0.0, 32.0));
                            if ui.add(folder_btn).clicked() {
                                crate::utils::open_folder(&out_dir);
                            }
                        });
                    }
                },
            );

            ui.add_space(10.0);

            // ==========================================
            // FASE 3: MODULACIÓN FM RF PARA SDR
            // ==========================================
            let (badge_text_3, badge_color_3) = if !self.iq_samples.is_empty() {
                ("LISTO", Color32::from_rgb(34, 197, 94))
            } else {
                ("PENDIENTE", Color32::from_rgb(156, 163, 175))
            };

            accordion_section(
                ui,
                theme,
                "generic_sstv_phase_3",
                AccordionOptions::new("Fase 3: Modulación FM RF para Transmisión SDR (288 kHz)")
                    .with_badge(badge_text_3, badge_color_3)
                    .default_open(false),
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Desviación de frecuencia FM:");
                        ui.add(egui::Slider::new(&mut self.fm_deviation_hz, 2_500.0..=25_000.0).suffix(" Hz"));
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Tasa de Muestreo SDR I/Q:");
                        egui::ComboBox::from_id_salt("generic_sstv_sdr_rate_combo")
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

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let can_mod = self.audio_samples.is_some() || self.img_path.is_some();
                        let btn_label = format!("3. Modular en FM RF ({} kHz)", self.sdr_sample_rate / 1000);
                        let start_btn = egui::Button::new(
                            RichText::new(btn_label).strong().color(Color32::WHITE),
                        )
                        .fill(if can_mod { theme.accent_blue() } else { theme.surface_subtle() })
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add_enabled(can_mod, start_btn).clicked() {
                            self.step_modulate_fm(config);
                        }

                        if !self.iq_samples.is_empty() {
                            let tx_btn = egui::Button::new(
                                RichText::new("Continuar a Efectos de Canal ->").strong().color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(34, 197, 94))
                            .corner_radius(8.0)
                            .min_size(Vec2::new(200.0, 34.0));

                            if ui.add(tx_btn).clicked() {
                                self.current_step = 2;
                            }
                        }
                    });

                    if !self.iq_samples.is_empty() {
                        ui.add_space(8.0);
                        let dur_sec = self.iq_samples.len() as f32 / self.sdr_sample_rate as f32;
                        ui.label(
                            RichText::new(format!(
                                "Métricas de Señal I/Q: Tasa {} Sps | Total {} muestras complejas | Duración: {:.2} s",
                                self.sdr_sample_rate,
                                self.iq_samples.len(),
                                dur_sec
                            ))
                            .size(12.0)
                            .color(theme.accent_green()),
                        );

                        ui.add_space(8.0);
                        accordion_section(
                            ui,
                            theme,
                            "generic_sstv_iq_osc",
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
                            "generic_sstv_iq_spec",
                            AccordionOptions::new("Espectrograma / Cascada RF").default_open(false),
                            |ui| {
                                let out_dir = config.get_pipeline_dir("generic_sstv");
                                crate::ui::spectrogram::render_spectrogram_widget(
                                    ui,
                                    theme,
                                    &mut self.spectrogram_state,
                                    &self.iq_samples,
                                    self.sdr_sample_rate,
                                    "generic_sstv",
                                    Some(&out_dir),
                                );
                            },
                        );
                    }
                },
            );
        });
    }

    /// Paso 2: Codificación Analógica SSTV a PCM 16-bit 48 kHz y guardado en WAV
    pub fn step_encode_audio(&mut self, config: &AppConfig) {
        let (name, res, _) = Self::mode_info_label(self.selected_mode);
        let img = self.img_path.as_ref()
            .and_then(|p| ImageReader::open(p).ok()?.decode().ok())
            .unwrap_or_else(|| {
                // Generar imagen de barras de color de prueba
                let mut rgb = RgbImage::new(640, 480);
                for (x, _, pixel) in rgb.enumerate_pixels_mut() {
                    let band = (x / 80) % 8;
                    *pixel = match band {
                        0 => image::Rgb([255, 255, 255]), // Blanco
                        1 => image::Rgb([255, 255, 0]),   // Amarillo
                        2 => image::Rgb([0, 255, 255]),   // Cyan
                        3 => image::Rgb([0, 255, 0]),     // Verde
                        4 => image::Rgb([255, 0, 255]),   // Magenta
                        5 => image::Rgb([255, 0, 0]),     // Rojo
                        6 => image::Rgb([0, 0, 255]),     // Azul
                        _ => image::Rgb([0, 0, 0]),       // Negro
                    };
                }
                DynamicImage::ImageRgb8(rgb)
            });

        match encode_sstv(&img, self.selected_mode, 48_000) {
            Ok(samples_i16) => {
                let out_dir = config.get_pipeline_dir("generic_sstv");
                let wav_path = out_dir.join("audio_sstv.wav");
                let _ = save_wav_file(&wav_path, &samples_i16, 48_000);

                self.audio_wav_path = Some(wav_path);
                self.audio_samples = Some(samples_i16);
                self.status_message = format!(
                    "Fase 2 completada: Audio SSTV codificado en modo {} ({}) (WAV guardado a 48 kHz).",
                    name,
                    res
                );
            }
            Err(e) => {
                self.status_message = format!("Error codificando SSTV: {}", e);
            }
        }
    }

    /// Paso 3: Modulación FM con tasa SDR seleccionada
    pub fn step_modulate_fm(&mut self, config: &AppConfig) {
        if self.audio_samples.is_none() {
            self.step_encode_audio(config);
        }

        if let Some(ref samples_i16) = self.audio_samples {
            let audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();
            self.iq_samples = fm_modulate(&audio_f32, 48_000, self.sdr_sample_rate, self.fm_deviation_hz, 0.0);
            self.channel_effects_state.is_dirty = true;
            self.channel_effects_state.processed_iq.clear();
            self.status_message = format!(
                "Fase 3 completada: Señal FM RF modulada a {} kHz ({} muestras complejas, Delta f = {:.1} kHz).",
                self.sdr_sample_rate / 1000,
                self.iq_samples.len(),
                self.fm_deviation_hz / 1000.0
            );
        }
    }

    pub fn execute_pipeline(&mut self, config: &AppConfig) {
        self.step_encode_audio(config);
        self.step_modulate_fm(config);
    }
}

fn save_wav_file(path: &std::path::Path, samples: &[i16], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        writer.write_sample(s).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(())
}
