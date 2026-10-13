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

pub struct GenericSstvPipeline {
    pub current_step: usize,
    pub img_path: Option<String>,
    pub url_input: String,
    pub selected_mode: Mode,
    pub fm_deviation_hz: f32,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
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
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imagen para modulación SSTV multiprotocolo.".to_string(),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl GenericSstvPipeline {
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
                1 => self.render_step_encode(ui, theme),
                2 => render_transmission_tab(
                    ui,
                    theme,
                    config,
                    "generic_sstv",
                    14_230_000.0, // 14.230 MHz (Frecuencia internacional SSTV en 20 metros USB/FM)
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

                ui.add_space(8.0);
                if ui
                    .button(RichText::new("Continuar a Selección de Modo ->").strong())
                    .clicked()
                {
                    self.current_step = 1;
                }
            } else {
                ui.label(
                    RichText::new("Seleccione una imagen o presione Continuar para usar un patrón de prueba sintético.")
                        .size(12.0)
                        .color(theme.text_secondary()),
                );
                ui.add_space(6.0);
                if ui
                    .button(RichText::new("Continuar con imagen sintética ->").strong())
                    .clicked()
                {
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

    fn render_step_encode(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Configuración del Modo SSTV").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Modo de Transmisión:");
                            let (current_name, current_res, current_dur) = Self::mode_info_label(self.selected_mode);

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

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("Desviación de frecuencia FM:");
                            ui.add(egui::Slider::new(&mut self.fm_deviation_hz, 2_500.0..=25_000.0).suffix(" Hz"));
                        });

                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Estructura de la señal: Tono de sincronismo 1200 Hz (línea y campo), Porche negro 1500 Hz, Escala analógica 1500-2300 Hz. Código VIS digital de 8 bits con paridad.")
                                .size(11.0)
                                .color(theme.text_secondary()),
                        );

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let start_btn = egui::Button::new(
                                RichText::new("Codificar SSTV y Modular en FM").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(start_btn).clicked() {
                                self.execute_pipeline();
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

            ui.add_space(10.0);
            render_signal_visualizer(ui, theme, &self.iq_samples, 288_000);
        });
    }

    fn execute_pipeline(&mut self) {
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
                let audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();
                self.iq_samples = fm_modulate(&audio_f32, 48_000, 288_000, self.fm_deviation_hz, 0.0);
                self.status_message = format!(
                    "Codificación exitosa en modo {} ({}): {} muestras I/Q generadas a 288 kHz.",
                    name,
                    res,
                    self.iq_samples.len()
                );
            }
            Err(e) => {
                self.status_message = format!("Error codificando SSTV: {}", e);
            }
        }
    }
}
