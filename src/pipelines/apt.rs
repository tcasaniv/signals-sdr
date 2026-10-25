use crate::channel::ChannelEffectsState;
use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::channel_effects::render_channel_effects_tab;
use crate::ui::components::{accordion_section, image_input_card, info_card, intermediate_preview_card, step_bar, AccordionOptions, ChannelColor, ImageSlot};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use apt_encoder::{crop_to_matching_height, encode_apt, fm_modulate, modulate_apt, preprocess_image, CHANNEL_WIDTH};
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};
use image::{DynamicImage, GrayImage};
use std::path::PathBuf;

pub struct AptPipeline {
    pub current_step: usize,
    // Paso 1: Fuente
    pub img_a: ImageSlot,
    pub img_b: ImageSlot,
    // Paso 2: Fases Didácticas de Codificación
    pub preprocessed_a: Option<GrayImage>,
    pub preprocessed_b: Option<GrayImage>,
    pub path_preprocessed_a: Option<PathBuf>,
    pub path_preprocessed_b: Option<PathBuf>,
    pub apt_image: Option<GrayImage>,
    pub path_apt_image: Option<PathBuf>,
    pub audio_am_samples: Option<Vec<i16>>,
    pub path_audio_wav: Option<PathBuf>,
    pub bytes_preprocessed_a: Option<Vec<u8>>,
    pub bytes_preprocessed_b: Option<Vec<u8>>,
    pub bytes_apt_image: Option<Vec<u8>>,
    pub bytes_audio_wav: Option<Vec<u8>>,
    pub audio_player: crate::ui::audio_player::AudioPlayerWidget,
    pub image_modal: crate::ui::image_modal::ImageViewerModal,
    pub output_files_modal: crate::ui::output_files_modal::OutputFilesModal,
    pub is_modulating_fm: bool,
    pub is_playing_audio: bool,
    #[cfg(target_arch = "wasm32")]
    pub active_audio_blob_url: Option<String>,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    pub channel_effects_state: ChannelEffectsState,
    pub sdr_sample_rate: u32,
    // Paso 3: Transmisión
    pub transmission_state: TransmissionTabState,
}

impl Default for AptPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            img_a: ImageSlot::new(),
            img_b: ImageSlot::new(),
            preprocessed_a: None,
            preprocessed_b: None,
            path_preprocessed_a: None,
            path_preprocessed_b: None,
            bytes_preprocessed_a: None,
            bytes_preprocessed_b: None,
            apt_image: None,
            path_apt_image: None,
            bytes_apt_image: None,
            audio_am_samples: None,
            path_audio_wav: None,
            bytes_audio_wav: None,
            audio_player: crate::ui::audio_player::AudioPlayerWidget::new(),
            image_modal: crate::ui::image_modal::ImageViewerModal::new(),
            output_files_modal: crate::ui::output_files_modal::OutputFilesModal::new(),
            is_modulating_fm: false,
            is_playing_audio: false,
            #[cfg(target_arch = "wasm32")]
            active_audio_blob_url: None,
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imágenes e iniciar el proceso.".to_string(),
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            channel_effects_state: ChannelEffectsState::new_for_pipeline("noaa_apt", 137.1),
            sdr_sample_rate: 288_000,
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl AptPipeline {
    pub fn release_heavy_memory(&mut self) {
        self.iq_samples.clear();
        self.iq_samples.shrink_to_fit();
        self.spectrogram_state.release_memory();
        self.channel_effects_state.release_memory();
        self.transmission_state.release_memory();
        self.audio_player.stop();
    }

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
                "NOAA APT (Automatic Picture Transmission)",
                "Sistema de transmisión analógica satelital utilizado por los satélites polares NOAA (NOAA-15, 18, 19). \
                 Transmite dos imágenes (visible e infrarroja) a 2 líneas por segundo usando una subportadora AM a 2400 Hz modulada en FM a 137.1 / 137.62 MHz / 137.9125 MHz.",
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
                        "noaa_apt",
                        self.sdr_sample_rate,
                        137_100_000.0,
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
                        "noaa_apt",
                        137_100_000.0,
                        &mut self.transmission_state,
                        sdr_controller,
                        effective_iq,
                        self.sdr_sample_rate,
                    );
                }
                _ => {}
            }

            // Renderizado de modales interactivos (Overlay a pantalla completa)
            self.output_files_modal.show(ui.ctx(), theme, &mut self.image_modal);
            self.image_modal.show(ui.ctx(), theme);
        });
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Selecciona una o dos imágenes para codificar en los canales A y B:")
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
                        &mut self.img_a,
                        ChannelColor::Blue,
                    );
                    image_input_card(
                        &mut cols[1],
                        theme,
                        "Imagen B",
                        &mut self.img_b,
                        ChannelColor::Green,
                    );
                });
            } else {
                image_input_card(
                    ui,
                    theme,
                    "Imagen A",
                    &mut self.img_a,
                    ChannelColor::Blue,
                );
                ui.add_space(10.0);
                image_input_card(
                    ui,
                    theme,
                    "Imagen B",
                    &mut self.img_b,
                    ChannelColor::Green,
                );
            }

            ui.add_space(16.0);
            if self.img_a.is_some() || self.img_b.is_some() {
                ui.horizontal(|ui| {
                    let next_btn = egui::Button::new(
                        RichText::new("Continuar a Codificación ->")
                            .strong()
                            .size(13.5)
                            .color(Color32::WHITE),
                    )
                    .fill(theme.accent_blue())
                    .corner_radius(8.0)
                    .min_size(Vec2::new(240.0, 38.0));

                    if ui.add(next_btn).clicked() {
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
                .corner_radius(10.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Flujo Paso a Paso de Codificación NOAA APT").strong().size(15.0).color(theme.accent_blue()));
                        ui.add_space(8.0);
                        let auto_btn = egui::Button::new(RichText::new("Ejecutar todo el Proceso").strong().size(12.5))
                            .min_size(Vec2::new(0.0, 32.0));
                        if ui.add(auto_btn).clicked() {
                            self.step_preprocess(config);
                            self.step_encode_apt(config);
                            self.step_modulate_am(config);
                            self.step_modulate_fm();
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(&self.status_message).size(12.5).color(theme.text_secondary()));
                });

            ui.add_space(14.0);

            // ==========================================
            // FASE 1: PREPROCESAMIENTO DE IMÁGENES
            // ==========================================
            let status_text_1 = if self.preprocessed_a.is_some() && self.preprocessed_b.is_some() {
                "LISTO"
            } else {
                "PENDIENTE"
            };
            let status_color_1 = if status_text_1 == "LISTO" {
                Color32::from_rgb(34, 197, 94)
            } else {
                Color32::from_rgb(156, 163, 175)
            };

            accordion_section(
                ui,
                theme,
                "apt_phase_1",
                AccordionOptions::new("Fase 1: Preprocesamiento de Imágenes (Canales A y B)")
                    .with_badge(status_text_1, status_color_1)
                    .default_open(false),
                |ui| {
                    ui.label(
                        RichText::new("Cada canal del sensor AVHRR satelital requiere exactamente 909 píxeles de ancho en escala de grises de 8 bits (0-255). Además, ambas imágenes deben recortarse simétricamente a la misma altura para sincronizar el barrido orbital.")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(10.0);

                    ui.horizontal_wrapped(|ui| {
                        let pre_btn = egui::Button::new(
                            RichText::new("1. Preprocesar Imágenes A y B (909 px)").strong().color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

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
                        format!("Dimensiones: {} x {} px | Escala de grises", img.width(), img.height())
                    } else {
                        "909 px de ancho (pendiente)".to_string()
                    };

                    let color_a = self.preprocessed_a.as_ref().map(crate::ui::components::gray_image_to_color_image);
                    let color_b = self.preprocessed_b.as_ref().map(crate::ui::components::gray_image_to_color_image);

                    let mut action_a = crate::ui::components::PreviewCardResponse::default();
                    let mut action_b = crate::ui::components::PreviewCardResponse::default();

                    if avail_w >= 680.0 {
                        ui.columns(2, |cols| {
                            action_a = intermediate_preview_card(
                                &mut cols[0],
                                theme,
                                "Canal A Preprocesado (909 px)",
                                &desc_a,
                                self.path_preprocessed_a.as_deref(),
                                self.preprocessed_a.is_some(),
                                ChannelColor::Blue,
                                color_a.as_ref(),
                            );
                            action_b = intermediate_preview_card(
                                &mut cols[1],
                                theme,
                                "Canal B Preprocesado (909 px)",
                                &desc_b,
                                self.path_preprocessed_b.as_deref(),
                                self.preprocessed_b.is_some(),
                                ChannelColor::Green,
                                color_b.as_ref(),
                            );
                        });
                    } else {
                        action_a = intermediate_preview_card(
                            ui,
                            theme,
                            "Canal A Preprocesado (909 px)",
                            &desc_a,
                            self.path_preprocessed_a.as_deref(),
                            self.preprocessed_a.is_some(),
                            ChannelColor::Blue,
                            color_a.as_ref(),
                        );
                        ui.add_space(6.0);
                        action_b = intermediate_preview_card(
                            ui,
                            theme,
                            "Canal B Preprocesado (909 px)",
                            &desc_b,
                            self.path_preprocessed_b.as_deref(),
                            self.preprocessed_b.is_some(),
                            ChannelColor::Green,
                            color_b.as_ref(),
                        );
                    }

                    if action_a.view_fullscreen {
                        if let Some(ref ci) = color_a {
                            self.image_modal.open_with_color_image(
                                "Canal A Preprocesado (909 px)",
                                ci.clone(),
                                "preprocesado_a.png",
                                self.bytes_preprocessed_a.clone(),
                            );
                        }
                    }
                    if action_a.download {
                        if let Some(ref bytes) = self.bytes_preprocessed_a {
                            #[cfg(target_arch = "wasm32")]
                            let _ = crate::web_utils::trigger_browser_download("preprocesado_a.png", bytes, "image/png");
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                                let _ = std::fs::write(dir.join("preprocesado_a.png"), bytes);
                            }
                        }
                    }

                    if action_b.view_fullscreen {
                        if let Some(ref ci) = color_b {
                            self.image_modal.open_with_color_image(
                                "Canal B Preprocesado (909 px)",
                                ci.clone(),
                                "preprocesado_b.png",
                                self.bytes_preprocessed_b.clone(),
                            );
                        }
                    }
                    if action_b.download {
                        if let Some(ref bytes) = self.bytes_preprocessed_b {
                            #[cfg(target_arch = "wasm32")]
                            let _ = crate::web_utils::trigger_browser_download("preprocesado_b.png", bytes, "image/png");
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                                let _ = std::fs::write(dir.join("preprocesado_b.png"), bytes);
                            }
                        }
                    }
                },
            );

            ui.add_space(10.0);

            // ==========================================
            // FASE 2: CODIFICACIÓN A TRAMA APT (2080 px)
            // ==========================================
            let status_text_2 = if self.apt_image.is_some() { "LISTO" } else { "PENDIENTE" };
            let status_color_2 = if status_text_2 == "LISTO" { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(156, 163, 175) };

            accordion_section(
                ui,
                theme,
                "apt_phase_2",
                AccordionOptions::new("Fase 2: Codificación a Trama Matricial APT (2080 px por línea)")
                    .with_badge(status_text_2, status_color_2)
                    .default_open(false),
                |ui| {
                    ui.label(
                        RichText::new("Ensambla la trama satelital completa a 2 líneas/segundo (2080 palabras/línea): ")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.label(
                        RichText::new("Sincronismo A (39 px, pulsos cuadrados 1040 Hz) + Espacio A (47 px) + Canal A (909 px) + Telemetría A (45 px) + Sincronismo B (39 px) + Espacio B (47 px) + Canal B (909 px) + Telemetría B (45 px).")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(10.0);

                    ui.horizontal_wrapped(|ui| {
                        let can_encode = self.img_a.is_some() || self.img_b.is_some() || self.preprocessed_a.is_some();
                        let encode_btn = egui::Button::new(
                            RichText::new("2. Codificar Trama Matricial APT (2080 px)").strong().color(Color32::WHITE),
                        )
                        .fill(if can_encode { theme.accent_blue() } else { theme.surface_subtle() })
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add_enabled(can_encode, encode_btn).clicked() {
                            self.step_encode_apt(config);
                        }

                        if !can_encode {
                            ui.label(RichText::new("(Primero selecciona al menos una imagen en el Paso 1)").size(11.5).color(theme.text_secondary()));
                        }
                    });

                    ui.add_space(10.0);

                    let apt_desc = if let Some(ref img) = self.apt_image {
                        let dur_sec = (img.height() as f32) * 0.5;
                        format!("Resolución: {} x {} px | Duración de escaneo: {:.1} segundos | Telemetría y marcas de sincronismo incrustadas", img.width(), img.height(), dur_sec)
                    } else {
                        "Resolución compuesta: 2080 px de ancho (pendiente)".to_string()
                    };

                    let color_apt = self.apt_image.as_ref().map(crate::ui::components::gray_image_to_color_image);

                    let action_apt = intermediate_preview_card(
                        ui,
                        theme,
                        "Imagen Compuesta APT (2080 px)",
                        &apt_desc,
                        self.path_apt_image.as_deref(),
                        self.apt_image.is_some(),
                        ChannelColor::Neutral,
                        color_apt.as_ref(),
                    );

                    if action_apt.view_fullscreen {
                        if let Some(ref ci) = color_apt {
                            self.image_modal.open_with_color_image(
                                "Imagen Compuesta APT (2080 px)",
                                ci.clone(),
                                "apt_compuesta.png",
                                self.bytes_apt_image.clone(),
                            );
                        }
                    }
                    if action_apt.download {
                        if let Some(ref bytes) = self.bytes_apt_image {
                            #[cfg(target_arch = "wasm32")]
                            let _ = crate::web_utils::trigger_browser_download("apt_compuesta.png", bytes, "image/png");
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                                let _ = std::fs::write(dir.join("apt_compuesta.png"), bytes);
                            }
                        }
                    }
                },
            );

            ui.add_space(10.0);

            // ==========================================
            // FASE 3: MODULACIÓN EN SUBPORTADORA AM (2400 Hz)
            // ==========================================
            let status_text_3 = if self.audio_am_samples.is_some() { "LISTO" } else { "PENDIENTE" };
            let status_color_3 = if status_text_3 == "LISTO" { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(156, 163, 175) };

            accordion_section(
                ui,
                theme,
                "apt_phase_3",
                AccordionOptions::new("Fase 3: Modulación en Subportadora AM (2400 Hz a 11025 Hz)")
                    .with_badge(status_text_3, status_color_3)
                    .default_open(false),
                |ui| {
                    ui.label(
                        RichText::new("Cada píxel modula en amplitud una subportadora senoidal pura de 2400 Hz. El estándar NOAA utiliza una frecuencia de muestreo de audio de 11,025 Hz (exactamente 4.59375 muestras/ciclo).")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(10.0);

                    ui.horizontal_wrapped(|ui| {
                        let can_mod = self.apt_image.is_some() || self.img_a.is_some();
                        let mod_btn = egui::Button::new(
                            RichText::new("3. Modular Subportadora AM (2400 Hz)").strong().color(Color32::WHITE),
                        )
                        .fill(if can_mod { theme.accent_blue() } else { theme.surface_subtle() })
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add_enabled(can_mod, mod_btn).clicked() {
                            self.step_modulate_am(config);
                        }

                        if !can_mod {
                            ui.label(RichText::new("(Primero genera la trama matricial en la Fase 2)").size(11.5).color(theme.text_secondary()));
                        }
                    });

                    if let Some(ref samples) = self.audio_am_samples {
                        ui.add_space(10.0);
                        let dur_sec = samples.len() as f32 / 11025.0;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Métricas de Audio:").strong().size(12.5));
                            ui.label(format!("Tasa de muestreo: 11,025 Hz | Portadora AM: 2,400 Hz | Total: {} muestras PCM | Duración: {:.2} s", samples.len(), dur_sec));
                        });

                        ui.add_space(8.0);
                        accordion_section(
                            ui,
                            theme,
                            "apt_am_osc",
                            AccordionOptions::new("Osciloscopio de Subportadora AM NOAA APT (11,025 Hz)").default_open(false),
                            |ui| {
                                crate::ui::spectrum::render_audio_waveform_i16(
                                    ui,
                                    theme,
                                    samples,
                                    11025,
                                    "Osciloscopio de Subportadora AM NOAA APT (11,025 Hz)",
                                );
                            },
                        );

                        ui.add_space(8.0);
                        self.audio_player.render(ui, theme, None);

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            // Botón Descargar WAV
                            if self.bytes_audio_wav.is_some() || self.path_audio_wav.is_some() {
                                let dl_label = format!("{} Descargar Audio (.wav)", crate::icons::ICON_DOWNLOAD.codepoint);
                                let dl_wav_btn = egui::Button::new(RichText::new(dl_label).size(12.5))
                                    .fill(theme.surface_subtle())
                                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                    .corner_radius(6.0)
                                    .min_size(Vec2::new(0.0, 32.0));

                                if ui.add(dl_wav_btn).clicked() {
                                    if let Some(ref bytes) = self.bytes_audio_wav {
                                        #[cfg(target_arch = "wasm32")]
                                        let _ = crate::web_utils::trigger_browser_download("audio_apt_2400hz.wav", bytes, "audio/wav");
                                        #[cfg(not(target_arch = "wasm32"))]
                                        if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                                            let _ = std::fs::write(dir.join("audio_apt_2400hz.wav"), bytes);
                                        }
                                    }
                                }
                            }

                            // Botón Archivos Generados
                            let files_label = format!("{} Archivos Generados ({})", crate::icons::ICON_FOLDER_OPEN.codepoint, self.output_files_modal.files.len());
                            let files_btn = egui::Button::new(
                                RichText::new(files_label).size(12.5)
                            )
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(0.0, 32.0));

                            if ui.add(files_btn).clicked() {
                                self.output_files_modal.open = true;
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let out_dir = config.get_pipeline_dir("noaa_apt");
                                    crate::utils::open_folder(&out_dir);
                                }
                            }
                        });
                    }
                },
            );

            ui.add_space(10.0);

            // ==========================================
            // FASE 4: MODULACIÓN FM I/Q PARA SDR
            // ==========================================
            let status_text_4 = if !self.iq_samples.is_empty() { "LISTO" } else { "PENDIENTE" };
            let status_color_4 = if status_text_4 == "LISTO" { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(156, 163, 175) };

            accordion_section(
                ui,
                theme,
                "apt_phase_4",
                AccordionOptions::new("Fase 4: Modulación FM I/Q para SDR (17 kHz Desviación a 288 kHz)")
                    .with_badge(status_text_4, status_color_4)
                    .default_open(false),
                |ui| {
                    ui.label(
                        RichText::new("Modula la subportadora AM en FM en banda ancha (WBFM) con 17 kHz de desviación y sobremuestreo a 288 kHz en banda base en cuadratura (I/Q) para transmisión SDR o exportación.")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Tasa de Muestreo SDR I/Q:");
                        egui::ComboBox::from_id_salt("apt_sdr_rate_combo")
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
                        let can_mod_fm = self.audio_am_samples.is_some();
                        let btn_label = format!("4. Modular Señal FM I/Q a {} kHz", self.sdr_sample_rate / 1000);
                        let fm_btn = egui::Button::new(
                            RichText::new(btn_label).strong().color(Color32::WHITE),
                        )
                        .fill(if can_mod_fm { theme.accent_blue() } else { theme.surface_subtle() })
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add_enabled(can_mod_fm, fm_btn).clicked() {
                            self.step_modulate_fm();
                        }

                        if !self.iq_samples.is_empty() {
                            let tx_btn = egui::Button::new(
                                RichText::new("Continuar a Efectos de Canal ->").strong().size(13.5).color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(34, 197, 94))
                            .corner_radius(8.0)
                            .min_size(Vec2::new(0.0, 36.0));

                            if ui.add(tx_btn).clicked() {
                                self.current_step = 2;
                            }
                        }
                    });

                    if !self.iq_samples.is_empty() {
                        ui.add_space(10.0);
                        let dur_sec = self.iq_samples.len() as f32 / self.sdr_sample_rate as f32;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Métricas de RF I/Q:").strong().size(12.5));
                            ui.label(format!("Tasa I/Q: {} Hz | Desviación: 17,000 Hz | Total: {} muestras I/Q | Duración: {:.2} s", self.sdr_sample_rate, self.iq_samples.len(), dur_sec));
                        });

                        ui.add_space(8.0);
                        accordion_section(
                            ui,
                            theme,
                            "apt_iq_osc",
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
                            "apt_iq_spec",
                            AccordionOptions::new("Espectrograma / Cascada RF").default_open(false),
                            |ui| {
                                let out_dir = config.get_pipeline_dir("noaa_apt");
                                crate::ui::spectrogram::render_spectrogram_widget(
                                    ui,
                                    theme,
                                    &mut self.spectrogram_state,
                                    &self.iq_samples,
                                    self.sdr_sample_rate,
                                    "noaa_apt",
                                    Some(&out_dir),
                                );
                            },
                        );
                    }
                },
            );
        });
    }

    /// Paso 1: Preprocesamiento de Imagen A y B a 909 px escala de grises y recorte simétrico
    pub fn step_preprocess(&mut self, config: &AppConfig) {
        let img_a = self.img_a.get_image().cloned()
            .unwrap_or_else(|| DynamicImage::ImageLuma8(GrayImage::new(CHANNEL_WIDTH, 200)));
        let img_b = self.img_b.get_image().cloned()
            .unwrap_or_else(|| DynamicImage::ImageLuma8(GrayImage::new(CHANNEL_WIDTH, 200)));

        let mut pre_a = preprocess_image(&img_a, CHANNEL_WIDTH);
        let mut pre_b = preprocess_image(&img_b, CHANNEL_WIDTH);
        let _ = crop_to_matching_height(&mut pre_a, &mut pre_b);

        let out_dir = config.get_pipeline_dir("noaa_apt");
        let path_a = out_dir.join("preprocesado_a.png");
        let path_b = out_dir.join("preprocesado_b.png");
        let _ = pre_a.save(&path_a);
        let _ = pre_b.save(&path_b);

        self.bytes_preprocessed_a = encode_gray_to_png(&pre_a);
        self.bytes_preprocessed_b = encode_gray_to_png(&pre_b);

        if let Some(ref b) = self.bytes_preprocessed_a {
            self.output_files_modal.push_file(crate::ui::output_files_modal::OutputFile::image(
                "preprocesado_a.png",
                b.clone(),
                "Canal A visible normalizado a 909 px de ancho",
            ));
        }
        if let Some(ref b) = self.bytes_preprocessed_b {
            self.output_files_modal.push_file(crate::ui::output_files_modal::OutputFile::image(
                "preprocesado_b.png",
                b.clone(),
                "Canal B infrarrojo normalizado a 909 px de ancho",
            ));
        }

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

            self.bytes_apt_image = encode_gray_to_png(&apt);
            if let Some(ref b) = self.bytes_apt_image {
                self.output_files_modal.push_file(crate::ui::output_files_modal::OutputFile::image(
                    "apt_compuesta.png",
                    b.clone(),
                    "Trama matricial continua NOAA APT (2080 px)",
                ));
            }

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

            // Guardar archivo WAV en disco y en memoria
            let _ = save_wav_file(&wav_path, &am_audio, 11025);
            self.bytes_audio_wav = encode_wav_to_bytes(&am_audio, 11025);

            if let Some(ref b) = self.bytes_audio_wav {
                self.audio_player.load_bytes("audio_apt_2400hz.wav", b.clone());
                self.output_files_modal.push_file(crate::ui::output_files_modal::OutputFile::audio(
                    "audio_apt_2400hz.wav",
                    b.clone(),
                    "Subportadora AM a 2400 Hz (11,025 Hz mono)",
                ));
            }

            self.audio_am_samples = Some(am_audio);
            self.path_audio_wav = Some(wav_path);

            self.status_message = "Fase 3 completada: Modulación en subportadora AM a 2400 Hz (11,025 Hz) lista y guardada en WAV.".to_string();
        }
    }

    /// Paso 4: Modulación FM I/Q con tasa SDR seleccionada (optimizado sin congelar WASM)
    pub fn step_modulate_fm(&mut self) {
        if let Some(ref audio) = self.audio_am_samples {
            #[cfg(target_arch = "wasm32")]
            {
                // En Web: A tasas de 1.0 MHz o superiores (como 2.0 MHz para HackRF),
                // modular una ventana representativa óptima para previsualizar cascada y oscilograma
                // sin desbordar el heap de WebAssembly ni congelar el hilo de JavaScript.
                let max_audio_samples = if self.sdr_sample_rate >= 2_000_000 {
                    (2.5 * 11025.0) as usize
                } else if self.sdr_sample_rate >= 1_000_000 {
                    (4.0 * 11025.0) as usize
                } else {
                    audio.len()
                };

                let slice = if audio.len() > max_audio_samples {
                    &audio[..max_audio_samples]
                } else {
                    audio
                };

                self.iq_samples = fm_modulate(slice, 11025, self.sdr_sample_rate, 17_000.0);
                self.channel_effects_state.is_dirty = true;
                self.channel_effects_state.processed_iq.clear();

                let dur = self.iq_samples.len() as f32 / self.sdr_sample_rate as f32;
                if audio.len() > max_audio_samples {
                    self.status_message = format!(
                        "Fase 4 completada: Señal FM I/Q generada a {} kHz ({} muestras complejas, {:.1}s representativos para rendimiento web). ¡Listo para transmitir!",
                        self.sdr_sample_rate / 1000,
                        self.iq_samples.len(),
                        dur
                    );
                } else {
                    self.status_message = format!(
                        "Fase 4 completada: Señal FM I/Q generada a {} kHz ({} muestras complejas). ¡Listo para transmitir!",
                        self.sdr_sample_rate / 1000,
                        self.iq_samples.len()
                    );
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                self.iq_samples = fm_modulate(audio, 11025, self.sdr_sample_rate, 17_000.0);
                self.channel_effects_state.is_dirty = true;
                self.channel_effects_state.processed_iq.clear();
                self.status_message = format!(
                    "Fase 4 completada: Señal FM I/Q generada a {} kHz ({} muestras complejas). ¡Listo para transmitir!",
                    self.sdr_sample_rate / 1000,
                    self.iq_samples.len()
                );
            }
        }
    }

    /// Reproduce el audio de subportadora AM generado de forma integrada
    pub fn play_audio(&mut self) {
        if self.is_playing_audio {
            self.stop_audio();
            return;
        }

        if let Some(ref bytes) = self.bytes_audio_wav {
            self.is_playing_audio = true;
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(old_url) = self.active_audio_blob_url.take() {
                    let _ = web_sys::Url::revoke_object_url(&old_url);
                }
                if let Ok(url) = crate::web_utils::create_audio_blob_url(bytes) {
                    let script = format!(
                        r#"(function(){{var a=document.getElementById('_apt_audio');if(a){{a.pause();a.remove();}}var el=document.createElement('audio');el.id='_apt_audio';el.src='{}';el.play();}})()"#,
                        url
                    );
                    let _ = js_sys::eval(&script);
                    self.active_audio_blob_url = Some(url);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = bytes;
                if let Some(ref p) = self.path_audio_wav {
                    if let Some(p_str) = p.to_str() {
                        crate::utils::open_path_or_url(p_str);
                    }
                }
            }
        }
    }

    /// Detiene la reproducción de audio
    pub fn stop_audio(&mut self) {
        self.is_playing_audio = false;
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(url) = self.active_audio_blob_url.take() {
                let _ = web_sys::Url::revoke_object_url(&url);
            }
            let _ = js_sys::eval(r#"(function(){var a=document.getElementById('_apt_audio');if(a){a.pause();a.remove();}})()"#);
        }
    }
}

/// Serializa una imagen GrayImage en bytes PNG usando Cursor
fn encode_gray_to_png(img: &GrayImage) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    if DynamicImage::ImageLuma8(img.clone()).write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
        Some(bytes)
    } else {
        None
    }
}

/// Serializa un buffer PCM i16 en bytes WAV usando Hound y Cursor
fn encode_wav_to_bytes(samples: &[i16], sample_rate: u32) -> Option<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).ok()?;
        for &s in samples {
            writer.write_sample(s).ok()?;
        }
        writer.finalize().ok()?;
    }
    Some(cursor.into_inner())
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

