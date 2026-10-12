use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::components::{image_input_card, info_card, step_bar};
use crate::ui::spectrum::render_signal_visualizer;
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use apt_encoder::{crop_to_matching_height, encode_apt, fm_modulate, modulate_apt, preprocess_image, CHANNEL_WIDTH};
use egui::{Color32, Frame, ProgressBar, RichText, Stroke, Ui};
use image::{DynamicImage, GrayImage, ImageReader};
use std::path::PathBuf;

pub struct AptPipeline {
    pub current_step: usize,
    // Paso 1: Fuente
    pub img_a_path: Option<String>,
    pub url_a_input: String,
    pub img_b_path: Option<String>,
    pub url_b_input: String,
    // Paso 2: Fases de Codificación
    pub preprocessed_a: Option<GrayImage>,
    pub preprocessed_b: Option<GrayImage>,
    pub apt_image: Option<GrayImage>,
    pub audio_am_samples: Option<Vec<i16>>,
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
            apt_image: None,
            audio_am_samples: None,
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imágenes.".to_string(),
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
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Selecciona una o dos imágenes para codificar en los canales A y B:")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                image_input_card(
                    &mut cols[0],
                    theme,
                    "Imagen A (Canal Visible / Ch1)",
                    &mut self.img_a_path,
                    &mut self.url_a_input,
                );
                image_input_card(
                    &mut cols[1],
                    theme,
                    "Imagen B (Canal Infrarrojo / Ch2)",
                    &mut self.img_b_path,
                    &mut self.url_b_input,
                );
            });

            ui.add_space(10.0);
            if self.img_a_path.is_some() || self.img_b_path.is_some() {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Continuar a Codificación").strong()).clicked() {
                        self.current_step = 1;
                    }
                });
            }
        });
    }

    fn render_step_encode(&mut self, ui: &mut Ui, theme: &AppTheme, config: &AppConfig) {
        ui.vertical(|ui| {
            Frame::none()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.card_border()))
                .rounding(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Flujo de Codificación NOAA APT").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);

                        // Fase 1: Preprocesamiento
                        let pre_status = if self.preprocessed_a.is_some() {
                            "Listo (909 px de ancho, escala de grises, recorte simétrico)"
                        } else {
                            "Pendiente"
                        };
                        ui.label(format!("1. Preprocesamiento de imágenes: {}", pre_status));

                        // Fase 2: Codificación a trama APT
                        let apt_status = if self.apt_image.is_some() {
                            "Listo (Trama APT generada a 2080 px de ancho con sincro A/B)"
                        } else {
                            "Pendiente"
                        };
                        ui.label(format!("2. Codificación matricial APT: {}", apt_status));

                        // Fase 3: Modulación AM y FM I/Q
                        let mod_status = if !self.iq_samples.is_empty() {
                            format!("Listo ({} muestras I/Q generadas a 288 kHz)", self.iq_samples.len())
                        } else {
                            "Pendiente".to_string()
                        };
                        ui.label(format!("3. Modulación AM (2400 Hz) y FM I/Q (17 kHz dev): {}", mod_status));

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let btn_text = if self.iq_samples.is_empty() {
                                "Iniciar Codificación Completa APT"
                            } else {
                                "Regenerar Señal APT"
                            };

                            let start_btn = egui::Button::new(
                                RichText::new(btn_text).strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .rounding(6.0);

                            if ui.add(start_btn).clicked() {
                                self.execute_pipeline(config);
                            }

                            if !self.iq_samples.is_empty() {
                                if ui.button(RichText::new("Continuar a Transmisión").strong()).clicked() {
                                    self.current_step = 2;
                                }
                            }
                        });

                        ui.add_space(6.0);
                        ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                    });
                });

            ui.add_space(10.0);
            render_signal_visualizer(ui, theme, &self.iq_samples, 288_000);
        });
    }

    fn execute_pipeline(&mut self, config: &AppConfig) {
        // Cargar o crear imágenes sintéticas si no hay seleccionadas
        let img_a = self.img_a_path.as_ref().and_then(|p| ImageReader::open(p).ok()?.decode().ok())
            .unwrap_or_else(|| DynamicImage::ImageLuma8(GrayImage::new(909, 200)));
        let img_b = self.img_b_path.as_ref().and_then(|p| ImageReader::open(p).ok()?.decode().ok())
            .unwrap_or_else(|| DynamicImage::ImageLuma8(GrayImage::new(909, 200)));

        // 1. Preprocesar
        let mut pre_a = preprocess_image(&img_a, CHANNEL_WIDTH);
        let mut pre_b = preprocess_image(&img_b, CHANNEL_WIDTH);
        let _ = crop_to_matching_height(&mut pre_a, &mut pre_b);
        self.preprocessed_a = Some(pre_a.clone());
        self.preprocessed_b = Some(pre_b.clone());

        // 2. Codificar a APT
        let apt = encode_apt(&pre_a, &pre_b);
        // Guardar copia intermedia de PNG en la subcarpeta del pipeline
        let out_dir = config.get_pipeline_dir("noaa_apt");
        let png_path = out_dir.join("apt_compuesta.png");
        let _ = apt.save(&png_path);
        self.apt_image = Some(apt.clone());

        // 3. Modular en AM
        let am_audio = modulate_apt(&apt, 11025);
        self.audio_am_samples = Some(am_audio.clone());

        // 4. Modular en FM I/Q a 288 kHz
        self.iq_samples = fm_modulate(&am_audio, 11025, 288_000, 17_000.0);
        self.status_message = format!(
            "Codificación exitosa: Imagen guardada en {:?}, {} muestras I/Q generadas.",
            png_path.file_name().unwrap_or_default(),
            self.iq_samples.len()
        );
    }
}
