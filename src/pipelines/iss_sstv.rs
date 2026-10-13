use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::components::{image_input_card, info_card, step_bar};
use crate::ui::spectrum::render_signal_visualizer;
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui};
use fm_modulator::fm_modulate;
use image::{DynamicImage, ImageReader, RgbImage};
use sstv_encoder::{encode_sstv, Mode};

pub struct IssSstvPipeline {
    pub current_step: usize,
    pub img_path: Option<String>,
    pub url_input: String,
    pub selected_mode: Mode,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub transmission_state: TransmissionTabState,
}

impl Default for IssSstvPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            img_path: None,
            url_input: String::new(),
            selected_mode: Mode::Robot36,
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imagen para la transmisión desde la ISS.".to_string(),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl IssSstvPipeline {
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
                "ISS SSTV (Estación Espacial Internacional)",
                "Transmisiones conmemorativas de televisión de barrido lento (SSTV) emitidas periódicamente desde el módulo ruso \
                 de la Estación Espacial Internacional (ISS / ARISS) en la frecuencia oficial de 145.800 MHz FM. \
                 El modo estándar más habitual de la ISS es Robot 36 (36 segundos por imagen) o PD 120.",
            );

            ui.add_space(8.0);
            step_bar(ui, theme, &mut self.current_step, "2. Codificar");

            match self.current_step {
                0 => self.render_step_source(ui, theme),
                1 => self.render_step_encode(ui, theme),
                2 => render_transmission_tab(
                    ui,
                    theme,
                    config,
                    "iss_sstv",
                    145_800_000.0,
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
                RichText::new("Selecciona una imagen para transmitir vía SSTV:")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(8.0);

            image_input_card(
                ui,
                theme,
                "Imagen para ISS SSTV",
                &mut self.img_path,
                &mut self.url_input,
                crate::ui::components::ChannelColor::Blue,
            );

            ui.add_space(10.0);
            if self.img_path.is_some() {
                if ui.button(RichText::new("Continuar a Codificación").strong()).clicked() {
                    self.current_step = 1;
                }
            }
        });
    }

    fn render_step_encode(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Ajustes del Modo ISS SSTV").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Modo SSTV de la ISS:");
                            let mode_name = match self.selected_mode {
                                Mode::Robot36 => "Robot 36 (36s, 320x240 YUV - Estándar ISS)",
                                Mode::Pd120 => "PD 120 (126s, 640x496 alta resolución)",
                                Mode::Martin1 => "Martin 1 (114s, 320x256 RGB)",
                                _ => "Robot 36",
                            };

                            egui::ComboBox::from_id_salt("iss_sstv_mode_combo")
                                .selected_text(mode_name)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.selected_mode, Mode::Robot36, "Robot 36 (36s, 320x240 YUV)");
                                    ui.selectable_value(&mut self.selected_mode, Mode::Pd120, "PD 120 (126s, 640x496 YUV)");
                                    ui.selectable_value(&mut self.selected_mode, Mode::Martin1, "Martin 1 (114s, 320x256 RGB)");
                                });
                        });

                        ui.add_space(6.0);
                        ui.label("Frecuencia portadora de audio: 1200 Hz (Sincro) a 2300 Hz (Blanco) | Modulación FM 5 kHz dev.");

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            let start_btn = egui::Button::new(
                                RichText::new("Codificar Imagen y Modular en FM").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(start_btn).clicked() {
                                self.execute_pipeline();
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

    fn execute_pipeline(&mut self) {
        let img = self.img_path.as_ref()
            .and_then(|p| ImageReader::open(p).ok()?.decode().ok())
            .unwrap_or_else(|| DynamicImage::ImageRgb8(RgbImage::new(320, 240)));

        match encode_sstv(&img, self.selected_mode, 48_000) {
            Ok(samples_i16) => {
                let audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();
                // Modulación FM en banda estrecha (5 kHz desviación) para transmisión en VHF 145.8 MHz
                self.iq_samples = fm_modulate(&audio_f32, 48_000, 288_000, 5_000.0, 0.0);
                self.status_message = format!(
                    "Codificación exitosa en modo {:?}: {} muestras I/Q generadas a 288 kHz.",
                    self.selected_mode,
                    self.iq_samples.len()
                );
            }
            Err(e) => {
                self.status_message = format!("Error codificando SSTV: {}", e);
            }
        }
    }
}
