use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::components::{image_input_card, info_card, intermediate_preview_card, step_bar, ChannelColor};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use apt_encoder::{crop_to_matching_height, encode_apt, fm_modulate, modulate_apt, preprocess_image, CHANNEL_WIDTH};
use egui::{Color32, Frame, RichText, Stroke, Ui};
use image::{DynamicImage, GrayImage, ImageReader};
use std::path::PathBuf;

pub struct AptPipeline {
    pub current_step: usize,
    // Paso 1: Fuente
    pub img_a_path: Option<String>,
    pub url_a_input: String,
    pub img_b_path: Option<String>,
    pub url_b_input: String,
    // Paso 2: Fases Didácticas de Codificación
    pub preprocessed_a: Option<GrayImage>,
    pub preprocessed_b: Option<GrayImage>,
    pub path_preprocessed_a: Option<PathBuf>,
    pub path_preprocessed_b: Option<PathBuf>,
    pub apt_image: Option<GrayImage>,
    pub path_apt_image: Option<PathBuf>,
    pub audio_am_samples: Option<Vec<i16>>,
    pub path_audio_wav: Option<PathBuf>,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    // Paso 3: Transmisión
    pub transmission_state: TransmissionTabState,
}

impl Default for AptPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            img_a_path: None,
            url_a_input: String::new(),
            img_b_path: None,
            url_b_input: String::new(),
            preprocessed_a: None,
            preprocessed_b: None,
            path_preprocessed_a: None,
            path_preprocessed_b: None,
            apt_image: None,
            path_apt_image: None,
            audio_am_samples: None,
            path_audio_wav: None,
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imágenes e iniciar el proceso.".to_string(),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl AptPipeline {
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
                "NOAA APT (Automatic Picture Transmission)",
                "Sistema de transmisión analógica satelital utilizado por los satélites polares NOAA (NOAA-15, 18, 19). \
                 Transmite dos imágenes (visible e infrarroja) a 2 líneas por segundo usando una subportadora AM a 2400 Hz modulada en FM a 137.1 / 137.9125 MHz.",
            );

            ui.add_space(8.0);
            step_bar(ui, theme, &mut self.current_step, "2. Codificar");

            match self.current_step {
                0 => self.render_step_source(ui, theme),
                1 => self.render_step_encode(ui, theme, config),
                2 => render_transmission_tab(
                    ui,
                    theme,
                    config,
                    "noaa_apt",
                    137_100_000.0,
                    &mut self.transmission_state,
                    sdr_controller,
                    &self.iq_samples,
                    288_000,
                ),
                _ => {}
            }
        });
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Selecciona una o dos imágenes para codificar en los canales A y B (haz clic en la vista previa para abrirla):")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(8.0);

            let avail_w = ui.available_width();
            if avail_w >= 680.0 {
                ui.columns(2, |cols| {
                    image_input_card(
                        &mut cols[0],
                        theme,
                        "Imagen A",
                        &mut self.img_a_path,
                        &mut self.url_a_input,
                        ChannelColor::Blue,
                    );
                    image_input_card(
                        &mut cols[1],
                        theme,
                        "Imagen B",
                        &mut self.img_b_path,
                        &mut self.url_b_input,
                        ChannelColor::Green,
                    );
                });
            } else {
                image_input_card(
                    ui,
                    theme,
                    "Imagen A",
                    &mut self.img_a_path,
                    &mut self.url_a_input,
                    ChannelColor::Blue,
                );
                ui.add_space(10.0);
                image_input_card(
                    ui,
                    theme,
                    "Imagen B",
                    &mut self.img_b_path,
                    &mut self.url_b_input,
                    ChannelColor::Green,
                );
            }

            ui.add_space(10.0);
            if self.img_a_path.is_some() || self.img_b_path.is_some() {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Continuar a Codificación ->").strong()).clicked() {
                        self.current_step = 1;
                    }
                });
            }
        });
    }

    fn render_step_encode(&mut self, ui: &mut Ui, theme: &AppTheme, config: &AppConfig) {
        let avail_w = ui.available_width();

        ui.vertical(|ui| {
            // Barra superior de estado general y botón para codificar todo de una sola vez si se desea
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Flujo Didáctico Paso a Paso de Codificación NOAA APT").strong().size(14.5).color(theme.accent_blue()));
                        ui.add_space(12.0);
                        let auto_btn = egui::Button::new(RichText::new("⚡ Ejecutar Todo el Proceso").strong().size(12.0));
                        if ui.add(auto_btn).clicked() {
                            self.step_preprocess(config);
                            self.step_encode_apt(config);
                            self.step_modulate_am(config);
                            self.step_modulate_fm();
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                });

            ui.add_space(10.0);

            // ==========================================
            // FASE 1: PREPROCESAMIENTO DE IMÁGENES
            // ==========================================
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Fase 1: Preprocesamiento de Imágenes (Canales A y B)").strong().size(14.0).color(theme.text_primary()));
                            let status_text = if self.preprocessed_a.is_some() && self.preprocessed_b.is_some() {
                                "LISTO"
                            } else {
                                "PENDIENTE"
                            };
                            let status_color = if status_text == "LISTO" {
                                Color32::from_rgb(34, 197, 94)
                            } else {
                                Color32::from_rgb(156, 163, 175)
                            };
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(status_text).strong().size(11.0).color(status_color));
                            });
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Cada canal del sensor AVHRR satelital requiere exactamente 909 píxeles de ancho en escala de grises de 8 bits (0-255). Además, ambas imágenes deben recortarse simétricamente a la misma altura para sincronizar el barrido orbital.")
                                .size(11.5)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            let pre_btn = egui::Button::new(
                                RichText::new("1. Preprocesar Imágenes A y B (909 px)").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(pre_btn).clicked() {
                                self.step_preprocess(config);
                            }
                        });

                        ui.add_space(8.0);

                        // Miniaturas interactivas para Canal A y Canal B preprocesados
                        let desc_a = if let Some(ref img) = self.preprocessed_a {
                            format!("Dimensiones: {} × {} px | Escala de grises", img.width(), img.height())
                        } else {
                            "909 px de ancho (pendiente)".to_string()
                        };
                        let desc_b = if let Some(ref img) = self.preprocessed_b {
                            format!("Dimensiones: {} × {} px | Escala de grises", img.width(), img.height())
                        } else {
                            "909 px de ancho (pendiente)".to_string()
                        };

                        if avail_w >= 680.0 {
                            ui.columns(2, |cols| {
                                intermediate_preview_card(
                                    &mut cols[0],
                                    theme,
                                    "Canal A Preprocesado (909 px)",
                                    &desc_a,
                                    self.path_preprocessed_a.as_deref(),
                                    self.preprocessed_a.is_some(),
                                    ChannelColor::Blue,
                                );
                                intermediate_preview_card(
                                    &mut cols[1],
                                    theme,
                                    "Canal B Preprocesado (909 px)",
                                    &desc_b,
                                    self.path_preprocessed_b.as_deref(),
                                    self.preprocessed_b.is_some(),
                                    ChannelColor::Green,
                                );
                            });
                        } else {
                            intermediate_preview_card(
                                ui,
                                theme,
                                "Canal A Preprocesado (909 px)",
                                &desc_a,
                                self.path_preprocessed_a.as_deref(),
                                self.preprocessed_a.is_some(),
                                ChannelColor::Blue,
                            );
                            ui.add_space(6.0);
                            intermediate_preview_card(
                                ui,
                                theme,
                                "Canal B Preprocesado (909 px)",
                                &desc_b,
                                self.path_preprocessed_b.as_deref(),
                                self.preprocessed_b.is_some(),
                                ChannelColor::Green,
                            );
                        }
                    });
                });

            ui.add_space(10.0);

            // ==========================================
            // FASE 2: CODIFICACIÓN A TRAMA APT (2080 px)
            // ==========================================
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Fase 2: Codificación a Trama Matricial APT (2080 px por línea)").strong().size(14.0).color(theme.text_primary()));
                            let status_text = if self.apt_image.is_some() { "LISTO" } else { "PENDIENTE" };
                            let status_color = if status_text == "LISTO" { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(156, 163, 175) };
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(status_text).strong().size(11.0).color(status_color));
                            });
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Ensambla la trama satelital completa a 2 líneas/segundo (2080 palabras/línea): Sincronismo A (39 px, pulsos cuadrados 1040 Hz) + Espacio A (47 px) + Canal A (909 px) + Telemetría A (45 px) + Sincronismo B (39 px) + Espacio B (47 px) + Canal B (909 px) + Telemetría B (45 px).")
                                .size(11.5)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            let can_encode = self.preprocessed_a.is_some() && self.preprocessed_b.is_some();
                            let encode_btn = egui::Button::new(
                                RichText::new("2. Codificar Trama Matricial APT (2080 px)").strong().color(Color32::WHITE),
                            )
                            .fill(if can_encode { theme.accent_blue() } else { theme.surface_subtle() })
                            .corner_radius(6.0);

                            if ui.add_enabled(can_encode, encode_btn).clicked() {
                                self.step_encode_apt(config);
                            }

                            if !can_encode {
                                ui.label(RichText::new("(Primero completa el preprocesamiento de la Fase 1)").size(11.5).color(theme.text_secondary()));
                            }
                        });

                        ui.add_space(8.0);

                        let apt_desc = if let Some(ref img) = self.apt_image {
                            let dur_sec = (img.height() as f32) * 0.5;
                            format!("Resolución: {} × {} px | Duración de escaneo: {:.1} segundos | Telemetría y marcas de sincronismo incrustadas", img.width(), img.height(), dur_sec)
                        } else {
                            "Resolución compuesta: 2080 px de ancho (pendiente)".to_string()
                        };

                        intermediate_preview_card(
                            ui,
                            theme,
                            "Imagen Compuesta APT (2080 px)",
                            &apt_desc,
                            self.path_apt_image.as_deref(),
                            self.apt_image.is_some(),
                            ChannelColor::Neutral,
                        );
                    });
                });

            ui.add_space(10.0);

            // ==========================================
            // FASE 3: MODULACIÓN EN SUBPORTADORA AM (2400 Hz)
            // ==========================================
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Fase 3: Modulación en Subportadora AM (2400 Hz a 11025 Hz)").strong().size(14.0).color(theme.text_primary()));
                            let status_text = if self.audio_am_samples.is_some() { "LISTO" } else { "PENDIENTE" };
                            let status_color = if status_text == "LISTO" { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(156, 163, 175) };
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(status_text).strong().size(11.0).color(status_color));
                            });
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Cada píxel modula en amplitud una subportadora senoidal pura de 2400 Hz. El estándar NOAA utiliza una frecuencia de muestreo de audio de 11,025 Hz (exactamente 4.59375 muestras/ciclo).")
                                .size(11.5)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            let can_mod = self.apt_image.is_some();
                            let mod_btn = egui::Button::new(
                                RichText::new("3. Modular Subportadora AM (2400 Hz)").strong().color(Color32::WHITE),
                            )
                            .fill(if can_mod { theme.accent_blue() } else { theme.surface_subtle() })
                            .corner_radius(6.0);

                            if ui.add_enabled(can_mod, mod_btn).clicked() {
                                self.step_modulate_am(config);
                            }

                            if !can_mod {
                                ui.label(RichText::new("(Primero genera la trama matricial en la Fase 2)").size(11.5).color(theme.text_secondary()));
                            }
                        });

                        if let Some(ref samples) = self.audio_am_samples {
                            ui.add_space(8.0);
                            let dur_sec = samples.len() as f32 / 11025.0;
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Métricas de Audio:").strong().size(12.0));
                                ui.label(format!("Tasa de muestreo: 11,025 Hz | Portadora AM: 2,400 Hz | Total: {} muestras PCM | Duración: {:.2} s", samples.len(), dur_sec));
                            });

                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                if let Some(ref wav_path) = self.path_audio_wav {
                                    if ui.button(RichText::new("▶ Reproducir / Abrir Audio APT (.wav)").strong()).clicked() {
                                        if let Some(p) = wav_path.to_str() {
                                            crate::utils::open_path_or_url(p);
                                        }
                                    }
                                }

                                let out_dir = config.get_pipeline_dir("noaa_apt");
                                if ui.button(RichText::new("📁 Abrir Carpeta de Salida").size(12.0)).clicked() {
                                    crate::utils::open_folder(&out_dir);
                                }
                            });
                        }
                    });
                });

            ui.add_space(10.0);

            // ==========================================
            // FASE 4: MODULACIÓN FM I/Q PARA SDR
            // ==========================================
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Fase 4: Modulación FM I/Q para SDR (17 kHz Desviación a 288 kHz)").strong().size(14.0).color(theme.text_primary()));
                            let status_text = if !self.iq_samples.is_empty() { "LISTO" } else { "PENDIENTE" };
                            let status_color = if status_text == "LISTO" { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(156, 163, 175) };
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(status_text).strong().size(11.0).color(status_color));
                            });
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Modula la subportadora AM en FM en banda ancha (WBFM) con 17 kHz de desviación y sobremuestreo a 288 kHz en banda base en cuadratura (I/Q) para transmisión SDR o exportación.")
                                .size(11.5)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            let can_mod_fm = self.audio_am_samples.is_some();
                            let fm_btn = egui::Button::new(
                                RichText::new("4. Modular Señal FM I/Q a 288 kHz").strong().color(Color32::WHITE),
                            )
                            .fill(if can_mod_fm { theme.accent_blue() } else { theme.surface_subtle() })
                            .corner_radius(6.0);

                            if ui.add_enabled(can_mod_fm, fm_btn).clicked() {
                                self.step_modulate_fm();
                            }

                            if !self.iq_samples.is_empty() {
                                let tx_btn = egui::Button::new(
                                    RichText::new("Continuar a Transmisión ->").strong().color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(34, 197, 94))
                                .corner_radius(6.0);

                                if ui.add(tx_btn).clicked() {
                                    self.current_step = 2;
                                }
                            }
                        });

                        if !self.iq_samples.is_empty() {
                            ui.add_space(8.0);
                            let dur_sec = self.iq_samples.len() as f32 / 288_000.0;
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Métricas de RF I/Q:").strong().size(12.0));
                                ui.label(format!("Tasa I/Q: 288,000 Hz | Desviación: 17,000 Hz | Total: {} muestras I/Q | Duración: {:.2} s", self.iq_samples.len(), dur_sec));
                            });
                        }
                    });
                });
        });
    }

    /// Paso 1: Preprocesamiento de Imagen A y B a 909 px escala de grises y recorte simétrico
    pub fn step_preprocess(&mut self, config: &AppConfig) {
        let img_a = self.img_a_path.as_ref().and_then(|p| ImageReader::open(p).ok()?.decode().ok())
            .unwrap_or_else(|| DynamicImage::ImageLuma8(GrayImage::new(CHANNEL_WIDTH, 200)));
        let img_b = self.img_b_path.as_ref().and_then(|p| ImageReader::open(p).ok()?.decode().ok())
            .unwrap_or_else(|| DynamicImage::ImageLuma8(GrayImage::new(CHANNEL_WIDTH, 200)));

        let mut pre_a = preprocess_image(&img_a, CHANNEL_WIDTH);
        let mut pre_b = preprocess_image(&img_b, CHANNEL_WIDTH);
        let _ = crop_to_matching_height(&mut pre_a, &mut pre_b);

        let out_dir = config.get_pipeline_dir("noaa_apt");
        let path_a = out_dir.join("preprocesado_a.png");
        let path_b = out_dir.join("preprocesado_b.png");
        let _ = pre_a.save(&path_a);
        let _ = pre_b.save(&path_b);

        self.preprocessed_a = Some(pre_a);
        self.preprocessed_b = Some(pre_b);
        self.path_preprocessed_a = Some(path_a);
        self.path_preprocessed_b = Some(path_b);

        self.status_message = "Fase 1 completada: Imágenes A y B preprocesadas a 909 px de ancho con recorte simétrico.".to_string();
    }

    /// Paso 2: Codificación a Trama Matricial APT (2080 px)
    pub fn step_encode_apt(&mut self, config: &AppConfig) {
        if self.preprocessed_a.is_none() || self.preprocessed_b.is_none() {
            self.step_preprocess(config);
        }

        if let (Some(ref a), Some(ref b)) = (&self.preprocessed_a, &self.preprocessed_b) {
            let apt = encode_apt(a, b);
            let out_dir = config.get_pipeline_dir("noaa_apt");
            let png_path = out_dir.join("apt_compuesta.png");
            let _ = apt.save(&png_path);

            self.apt_image = Some(apt);
            self.path_apt_image = Some(png_path);

            self.status_message = "Fase 2 completada: Trama APT continua de 2080 px por línea generada con sincronismo y telemetría.".to_string();
        }
    }

    /// Paso 3: Modulación AM a 2400 Hz (WAV a 11025 Hz)
    pub fn step_modulate_am(&mut self, config: &AppConfig) {
        if self.apt_image.is_none() {
            self.step_encode_apt(config);
        }

        if let Some(ref apt) = self.apt_image {
            let am_audio = modulate_apt(apt, 11025);
            let out_dir = config.get_pipeline_dir("noaa_apt");
            let wav_path = out_dir.join("audio_apt_2400hz.wav");

            // Guardar archivo WAV
            let _ = save_wav_file(&wav_path, &am_audio, 11025);

            self.audio_am_samples = Some(am_audio);
            self.path_audio_wav = Some(wav_path);

            self.status_message = "Fase 3 completada: Modulación en subportadora AM a 2400 Hz (11,025 Hz) lista y guardada en WAV.".to_string();
        }
    }

    /// Paso 4: Modulación FM I/Q a 288 kHz
    pub fn step_modulate_fm(&mut self) {
        if let Some(ref audio) = self.audio_am_samples {
            self.iq_samples = fm_modulate(audio, 11025, 288_000, 17_000.0);
            self.status_message = format!(
                "Fase 4 completada: Señal FM I/Q generada a 288 kHz ({} muestras complejas). ¡Listo para transmitir!",
                self.iq_samples.len()
            );
        }
    }
}

/// Guarda un buffer PCM i16 como archivo WAV mono usando Hound
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

