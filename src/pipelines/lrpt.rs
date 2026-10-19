use crate::channel::ChannelEffectsState;
use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::channel_effects::render_channel_effects_tab;
use crate::ui::components::{accordion_section, image_input_card, info_card, step_bar, AccordionOptions};
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};
use lrpt_encoder::{
    ccsds, compression, convolutional, differential, imaging, modulation, MCUS_PER_ROW,
};

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
    pub phase1_ready: bool,
    pub phase2_ready: bool,
    pub mcu_rows_count: usize,
    pub source_packets: Vec<Vec<u8>>,
    pub vcdu_count: usize,
    pub encoded_cadus: Vec<Vec<u8>>,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub spectrogram_state: crate::ui::spectrogram::SpectrogramState,
    pub channel_effects_state: ChannelEffectsState,
    pub sdr_sample_rate: u32,
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
            phase1_ready: false,
            phase2_ready: false,
            mcu_rows_count: 0,
            source_packets: Vec::new(),
            vcdu_count: 0,
            encoded_cadus: Vec::new(),
            iq_samples: Vec::new(),
            status_message: "Listo para seleccionar imágenes de canales meteorológicos.".to_string(),
            spectrogram_state: crate::ui::spectrogram::SpectrogramState::default(),
            channel_effects_state: ChannelEffectsState::new_for_pipeline("meteor_lrpt", 137.9),
            sdr_sample_rate: 288_000,
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl LrptPipeline {
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
                2 => {
                    let mut next_step = false;
                    render_channel_effects_tab(
                        ui,
                        theme,
                        config,
                        "meteor_lrpt",
                        self.sdr_sample_rate,
                        137_900_000.0,
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
                        "meteor_lrpt",
                        137_900_000.0,
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
            if self.img1_path.is_some() || self.img2_path.is_some() || self.img3_path.is_some() {
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
        ui.vertical(|ui| {
            // Barra superior didáctica con botón para ejecutar todo en un clic
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(10.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Flujo Didáctico Paso a Paso Meteor LRPT").strong().size(15.0).color(theme.accent_blue()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let auto_btn = egui::Button::new(
                                RichText::new("Ejecutar Todo el Pipeline en un Clic").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(8.0)
                            .min_size(Vec2::new(0.0, 32.0));

                            if ui.add(auto_btn).clicked() {
                                self.execute_pipeline();
                            }
                        });
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                });

            ui.add_space(12.0);

            // ==========================================
            // FASE 1: COMPRESIÓN JPEG DE CANALES MCU
            // ==========================================
            let (badge_text_1, badge_color_1) = if self.phase1_ready {
                ("LISTO", Color32::from_rgb(34, 197, 94))
            } else {
                ("PENDIENTE", Color32::from_rgb(156, 163, 175))
            };

            accordion_section(
                ui,
                theme,
                "lrpt_phase_1",
                AccordionOptions::new("Fase 1: Compresión de Canales y Formato JPEG MCU (8x8 px)")
                    .with_badge(badge_text_1, badge_color_1)
                    .default_open(true),
                |ui| {
                    ui.label(
                        RichText::new("Meteor-M2 comprime cada canal óptico (1568 px de ancho, 196 bloques MCU por línea) en bloques de 8x8 píxeles mediante transformada discreta del coseno (DCT) y tablas de cuantificación de luminancia JPEG según el estándar CCSDS.")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label("Calidad de compresión JPEG (Factor Q):");
                        ui.add(egui::Slider::new(&mut self.quality, 20..=95));
                    });

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let btn1 = egui::Button::new(
                            RichText::new("1. Comprimir Canales en Bloques JPEG MCU").strong().color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add(btn1).clicked() {
                            self.step_1_compress_mcus();
                        }
                    });

                    if self.phase1_ready {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Métricas de Fase 1: {} filas MCU procesadas | {} paquetes de datos MSUMR generados (APID 64, 65, 66)",
                                self.mcu_rows_count,
                                self.source_packets.len()
                            ))
                            .size(12.0)
                            .color(theme.accent_green()),
                        );
                    }
                },
            );

            ui.add_space(10.0);

            // ==========================================
            // FASE 2: TRAMAS CCSDS (VCDU) Y FEC
            // ==========================================
            let (badge_text_2, badge_color_2) = if self.phase2_ready {
                ("LISTO", Color32::from_rgb(34, 197, 94))
            } else {
                ("PENDIENTE", Color32::from_rgb(156, 163, 175))
            };

            accordion_section(
                ui,
                theme,
                "lrpt_phase_2",
                AccordionOptions::new("Fase 2: Protocolo CCSDS y Corrección de Errores FEC (Reed-Solomon + Convolucional)")
                    .with_badge(badge_text_2, badge_color_2)
                    .default_open(true),
                |ui| {
                    ui.label(
                        RichText::new("Los paquetes espaciales se multiplexan en tramas de transferencia de canal virtual (VCDU) de 1024 bytes. Se calcula el código Reed-Solomon RS(255, 223) con entrelazado profundidad 4, se añade palabra sincro ASM 0x1ACFFC1D, codificación diferencial NRZ-M y codificador convolucional (K=7, R=1/2).")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        let can_fase2 = self.phase1_ready || self.img1_path.is_some();
                        let btn2 = egui::Button::new(
                            RichText::new("2. Estructurar Tramas CCSDS (VCDU) y FEC").strong().color(Color32::WHITE),
                        )
                        .fill(if can_fase2 { theme.accent_blue() } else { theme.surface_subtle() })
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add_enabled(can_fase2, btn2).clicked() {
                            self.step_2_build_ccsds_fec();
                        }

                        if !can_fase2 {
                            ui.label(RichText::new("(Primero ejecuta la compresión de la Fase 1)").size(11.5).color(theme.text_secondary()));
                        }
                    });

                    if self.phase2_ready {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Métricas de Fase 2: {} tramas VCDU de 1024 bytes | {} tramas CADU con corrección FEC (Tasa 1/2)",
                                self.vcdu_count,
                                self.encoded_cadus.len()
                            ))
                            .size(12.0)
                            .color(theme.accent_green()),
                        );
                    }
                },
            );

            ui.add_space(10.0);

            // ==========================================
            // FASE 3: MODULACIÓN DIGITAL OQPSK (288 kHz)
            // ==========================================
            let (badge_text_3, badge_color_3) = if !self.iq_samples.is_empty() {
                ("LISTO", Color32::from_rgb(34, 197, 94))
            } else {
                ("PENDIENTE", Color32::from_rgb(156, 163, 175))
            };

            accordion_section(
                ui,
                theme,
                "lrpt_phase_3",
                AccordionOptions::new("Fase 3: Modulación Digital OQPSK y Filtro RRC (288 kHz)")
                    .with_badge(badge_text_3, badge_color_3)
                    .default_open(true),
                |ui| {
                    ui.label(
                        RichText::new("Modulación en cuadratura con desplazamiento (Offset-QPSK) a 72 ksps con retardo de medio símbolo entre I y Q. La señal se conforma con filtro de coseno alzado (RRC, alfa=0.5) y se sobremuestrea a 288 kHz en banda base I/Q.")
                            .size(12.0)
                            .color(theme.text_secondary()),
                    );
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Tasa de Muestreo SDR I/Q:");
                        egui::ComboBox::from_id_salt("lrpt_sdr_rate_combo")
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
                        let can_fase3 = self.phase2_ready || self.phase1_ready || self.img1_path.is_some();
                        let btn_label = format!("3. Modular Señal Digital OQPSK ({} kHz)", self.sdr_sample_rate / 1000);
                        let btn3 = egui::Button::new(
                            RichText::new(btn_label).strong().color(Color32::WHITE),
                        )
                        .fill(if can_fase3 { theme.accent_blue() } else { theme.surface_subtle() })
                        .corner_radius(8.0)
                        .min_size(Vec2::new(0.0, 34.0));

                        if ui.add_enabled(can_fase3, btn3).clicked() {
                            self.step_3_modulate_oqpsk();
                        }

                        if !self.iq_samples.is_empty() {
                            let tx_btn = egui::Button::new(
                                RichText::new("Continuar a Efectos de Canal ->").strong().size(13.5).color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(34, 197, 94))
                            .corner_radius(8.0)
                            .min_size(Vec2::new(240.0, 38.0));

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
                                "Métricas de Señal I/Q: Tasa {} Sps | 72,000 símbolos/s | Total {} muestras I/Q | Duración: {:.2} s",
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
                            "lrpt_iq_osc",
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
                            "lrpt_iq_spec",
                            AccordionOptions::new("Espectrograma y Cascada de Radiofrecuencia (FFT en Vivo)").default_open(true),
                            |ui| {
                                let out_dir = config.get_pipeline_dir("meteor_lrpt");
                                crate::ui::spectrogram::render_spectrogram_widget(
                                    ui,
                                    theme,
                                    &mut self.spectrogram_state,
                                    &self.iq_samples,
                                    self.sdr_sample_rate,
                                    "meteor_lrpt",
                                    Some(&out_dir),
                                );
                            },
                        );
                    }
                },
            );
        });
    }

    fn get_channels_matrix(&self) -> [Vec<Vec<u8>>; 3] {
        let load_or_dummy = |path: &Option<String>| -> Vec<Vec<u8>> {
            path.as_ref()
                .and_then(|p| imaging::load_image(std::path::Path::new(p)).ok())
                .unwrap_or_else(|| vec![vec![128u8; 1568]; 64])
        };

        let ch1 = load_or_dummy(&self.img1_path);
        let ch2 = if self.num_channels >= 2 { load_or_dummy(&self.img2_path) } else { ch1.clone() };
        let ch3 = if self.num_channels >= 3 { load_or_dummy(&self.img3_path) } else { ch1.clone() };
        [ch1, ch2, ch3]
    }

    /// Paso 1: Extracción de MCU y compresión JPEG
    pub fn step_1_compress_mcus(&mut self) {
        let channels = self.get_channels_matrix();
        let apids = [64u16, 65, 66];
        let height = channels[0].len();
        let num_mcu_rows = imaging::mcu_row_count(height);
        self.mcu_rows_count = num_mcu_rows;

        let day = 24938u16;
        let ms_of_day = 43200000u32;
        let mut all_source_packets: Vec<Vec<u8>> = Vec::new();
        let mut seq_count: u16 = 0;
        let segments_per_line: usize = 14;
        let mcus_per_segment: usize = MCUS_PER_ROW / segments_per_line;

        for mcu_row_idx in 0..num_mcu_rows {
            let timestamp = ms_of_day.wrapping_add((mcu_row_idx as u32).wrapping_mul(500)) % 86_400_000;
            for ch in 0..3 {
                let mcus = imaging::extract_mcu_row(&channels[ch], mcu_row_idx);
                for seg_idx in 0..segments_per_line {
                    let start = seg_idx * mcus_per_segment;
                    let end = start + mcus_per_segment;
                    let compressed = compression::compress_mcu_row(&mcus[start..end], self.quality);
                    let packet = ccsds::build_msumr_segment_packet(
                        apids[ch],
                        seq_count,
                        day,
                        timestamp,
                        start as u8,
                        self.quality,
                        &compressed,
                    );
                    all_source_packets.push(packet);
                    seq_count = (seq_count + 1) & 0x3FFF;
                }
            }
            let telemetry = ccsds::build_msumr_telemetry_packet(
                seq_count,
                day,
                timestamp,
                4,
                None,
            );
            all_source_packets.push(telemetry);
            seq_count = (seq_count + 1) & 0x3FFF;
        }

        self.source_packets = all_source_packets;
        self.phase1_ready = true;
        self.status_message = format!(
            "Fase 1 completada: {} paquetes de segmentos MCU JPEG comprimidos (Factor Q={}).",
            self.source_packets.len(),
            self.quality
        );
    }

    /// Paso 2: Multiplexado VCDU, Reed-Solomon y codificación convolucional
    pub fn step_2_build_ccsds_fec(&mut self) {
        if !self.phase1_ready {
            self.step_1_compress_mcus();
        }

        let mut vcdu_builder = ccsds::VcduBuilder::new(57, 5);
        let vcdus = vcdu_builder.multiplex(&self.source_packets);

        let mut encoded_cadus: Vec<Vec<u8>> = Vec::with_capacity(vcdus.len());
        let mut conv_state: u8 = 0;
        let mut nrzm_last_bit: u8 = 0;

        for vcdu in &vcdus {
            let raw_cadu = ccsds::build_cadu(vcdu);
            let diff_cadu = differential::nrzm_encode_bits(&raw_cadu, &mut nrzm_last_bit);
            encoded_cadus.push(convolutional::encode_with_state(
                &diff_cadu,
                &mut conv_state,
            ));
        }

        self.vcdu_count = vcdus.len();
        self.encoded_cadus = encoded_cadus;
        self.phase2_ready = true;
        self.status_message = format!(
            "Fase 2 completada: {} tramas VCDU multiplexadas y {} tramas CADU codificadas con FEC (Reed-Solomon + Convolucional R=1/2).",
            self.vcdu_count,
            self.encoded_cadus.len()
        );
    }

    /// Paso 3: Modulación digital OQPSK a 72 ksps con tasa de muestreo SDR seleccionada
    pub fn step_3_modulate_oqpsk(&mut self) {
        if !self.phase2_ready {
            self.step_2_build_ccsds_fec();
        }

        let base_iq = modulation::modulate_cadu_stream(&self.encoded_cadus);
        if self.sdr_sample_rate == 288_000 {
            self.iq_samples = base_iq;
        } else {
            self.iq_samples = resample_iq(&base_iq, 288_000, self.sdr_sample_rate);
        }

        self.channel_effects_state.is_dirty = true;
        self.channel_effects_state.processed_iq.clear();

        self.status_message = format!(
            "Fase 3 completada: Modulación digital OQPSK a {} kHz generada con éxito ({} muestras I/Q). ¡Listo para transmitir!",
            self.sdr_sample_rate / 1000,
            self.iq_samples.len()
        );
    }

    pub fn execute_pipeline(&mut self) {
        self.step_1_compress_mcus();
        self.step_2_build_ccsds_fec();
        self.step_3_modulate_oqpsk();
    }
}

/// Remuestrea una señal I/Q mediante interpolación lineal de fase continua
fn resample_iq(samples: &[(f32, f32)], in_rate: u32, out_rate: u32) -> Vec<(f32, f32)> {
    if in_rate == out_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let duration = samples.len() as f64 / in_rate as f64;
    let n_out = (duration * out_rate as f64).round() as usize;
    if n_out == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n_out);
    let ratio = (samples.len() - 1) as f64 / ((n_out - 1).max(1) as f64);
    for j in 0..n_out {
        let src_pos = j as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        if idx + 1 < samples.len() {
            let i = samples[idx].0 * (1.0 - frac) + samples[idx + 1].0 * frac;
            let q = samples[idx].1 * (1.0 - frac) + samples[idx + 1].1 * frac;
            out.push((i, q));
        } else {
            out.push(samples[idx]);
        }
    }
    out
}
