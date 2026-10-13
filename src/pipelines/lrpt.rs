use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::components::{image_input_card, info_card, step_bar};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};
use lrpt_encoder::{encode_lrpt_channels, imaging, LrptConfig};

pub struct LrptPipeline {
    pub current_step: usize,
    pub num_channels: usize,
    pub img1_path: Option<String>,
    pub url1_input: String,
    pub img2_path: Option<String>,
    pub url2_input: String,
    pub img3_path: Option<String>,
    pub url3_input: String,
    pub quality: u8,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub transmission_state: TransmissionTabState,
}

impl Default for LrptPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            num_channels: 3,
            img1_path: None,
            url1_input: String::new(),
            img2_path: None,
            url2_input: String::new(),
            img3_path: None,
            url3_input: String::new(),
            quality: 80,
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imágenes de canales meteorológicos.".to_string(),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl LrptPipeline {
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
                "METEOR LRPT (Low Rate Picture Transmission)",
                "Protocolo digital de satélites meteorológicos rusos Meteor-M N2-x. \
                 Transmite 3 canales espectrales multiplexados con compresión JPEG DCT por MCU, \
                 paquetes CCSDS, protección Reed-Solomon (255,223), Scrambler y codificación convolucional FEC (r=1/2, K=7), \
                 modulados en OQPSK a 72 ksps sobre 137.1 / 137.9 MHz.",
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
                    "meteor_lrpt",
                    137_900_000.0,
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
            ui.horizontal(|ui| {
                ui.label(RichText::new("Número de canales a codificar:").size(13.0).color(theme.text_secondary()));
                egui::ComboBox::from_id_salt("lrpt_num_ch")
                    .selected_text(format!("{} canales", self.num_channels))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.num_channels, 1, "1 canal (duplicado)");
                        ui.selectable_value(&mut self.num_channels, 2, "2 canales");
                        ui.selectable_value(&mut self.num_channels, 3, "3 canales (completo)");
                    });
            });

            ui.add_space(8.0);

            let avail_w = ui.available_width();
            if avail_w >= 750.0 {
                let cols_count = self.num_channels;
                ui.columns(cols_count, |cols| {
                    image_input_card(
                        &mut cols[0],
                        theme,
                        "Canal 1 (APID 64)",
                        &mut self.img1_path,
                        &mut self.url1_input,
                        crate::ui::components::ChannelColor::Blue,
                    );
                    if self.num_channels >= 2 {
                        image_input_card(
                            &mut cols[1],
                            theme,
                            "Canal 2 (APID 65)",
                            &mut self.img2_path,
                            &mut self.url2_input,
                            crate::ui::components::ChannelColor::Green,
                        );
                    }
                    if self.num_channels >= 3 {
                        image_input_card(
                            &mut cols[2],
                            theme,
                            "Canal 3 (APID 66)",
                            &mut self.img3_path,
                            &mut self.url3_input,
                            crate::ui::components::ChannelColor::Neutral,
                        );
                    }
                });
            } else {
                image_input_card(
                    ui,
                    theme,
                    "Canal 1 (APID 64)",
                    &mut self.img1_path,
                    &mut self.url1_input,
                    crate::ui::components::ChannelColor::Blue,
                );
                if self.num_channels >= 2 {
                    ui.add_space(10.0);
                    image_input_card(
                        ui,
                        theme,
                        "Canal 2 (APID 65)",
                        &mut self.img2_path,
                        &mut self.url2_input,
                        crate::ui::components::ChannelColor::Green,
                    );
                }
                if self.num_channels >= 3 {
                    ui.add_space(10.0);
                    image_input_card(
                        ui,
                        theme,
                        "Canal 3 (APID 66)",
                        &mut self.img3_path,
                        &mut self.url3_input,
                        crate::ui::components::ChannelColor::Neutral,
                    );
                }
            }

            ui.add_space(16.0);
            if self.img1_path.is_some() {
                ui.horizontal(|ui| {
                    let next_btn = egui::Button::new(
                        RichText::new("Continuar a Codificación  ➜")
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

    fn render_step_encode(&mut self, ui: &mut Ui, theme: &AppTheme, _config: &AppConfig) {
        ui.vertical(|ui| {
            // Fase 1: Compresión JPEG de Canales
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Fase 1: Compresión de Canales y Formato JPEG MCU").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Meteor-M2 comprime cada canal óptico (1568 px de ancho) en bloques de 8x8 píxeles (MCU) usando tablas cuantificadas de luminancia JPEG según el estándar CCSDS.")
                                .size(11.5)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label("Calidad de compresión JPEG (Factor Q):");
                            ui.add(egui::Slider::new(&mut self.quality, 20..=95));
                        });
                    });
                });

            ui.add_space(10.0);

            // Fase 2: Tramas CCSDS y Codificación de Canal FEC
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Fase 2: Protocolo CCSDS y Corrección de Errores FEC").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Los paquetes de paquetes de datos se encapsulan en tramas de transferencia de canal virtual (VCDU) de 1024 bytes con codificación convolucional (K=7, R=1/2) y código Reed-Solomon RS(255, 223) con entrelazado.")
                                .size(11.5)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(4.0);
                        ui.label(format!("Canales activos: {} | APIDs: 64{}", self.num_channels, if self.num_channels >= 2 { ", 65" } else { "" }));
                    });
                });

            ui.add_space(10.0);

            // Fase 3: Modulación Digital OQPSK y Filtro de Conformado RRC
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Fase 3: Modulación Digital OQPSK y Filtro RRC (288 kHz)").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Modulación en cuadratura con desplazamiento (Offset-QPSK) a 72 ksps con un retardo de medio símbolo entre I y Q para minimizar las variaciones de envolvente, conformada por filtro de coseno alzado (RRC, alfa=0.5) y sobremuestreo a 288 kHz.")
                                .size(11.5)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            let start_btn = egui::Button::new(
                                RichText::new("Generar Señal OQPSK LRPT").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(start_btn).clicked() {
                                self.execute_pipeline();
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

                        ui.add_space(6.0);
                        ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));

                        if !self.iq_samples.is_empty() {
                            ui.add_space(4.0);
                            let dur_sec = self.iq_samples.len() as f32 / 288_000.0;
                            ui.label(format!("Total generado: {} muestras I/Q complejas a 288 kHz (Duración: {:.2} s)", self.iq_samples.len(), dur_sec));
                        }
                    });
                });
        });
    }

    fn execute_pipeline(&mut self) {
        let load_or_dummy = |path: &Option<String>| -> Vec<Vec<u8>> {
            path.as_ref()
                .and_then(|p| imaging::load_image(std::path::Path::new(p)).ok())
                .unwrap_or_else(|| vec![vec![128u8; 1568]; 64])
        };

        let ch1 = load_or_dummy(&self.img1_path);
        let ch2 = if self.num_channels >= 2 { load_or_dummy(&self.img2_path) } else { ch1.clone() };
        let ch3 = if self.num_channels >= 3 { load_or_dummy(&self.img3_path) } else { ch1.clone() };

        let channels = [ch1, ch2, ch3];

        let mut config = LrptConfig::default();
        config.quality = self.quality;

        let (_cadu, iq) = encode_lrpt_channels(&channels, &config);
        self.iq_samples = iq;
        self.status_message = format!(
            "Codificación completada exitosamente: {} muestras I/Q generadas a 288 kHz.",
            self.iq_samples.len()
        );
    }
}
