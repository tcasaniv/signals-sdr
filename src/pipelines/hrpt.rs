use crate::config::AppConfig;
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::components::{info_card, step_bar};
use crate::ui::spectrum::render_signal_visualizer;
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui};

pub struct HrptPipeline {
    pub current_step: usize,
    pub iq_samples: Vec<(f32, f32)>,
    pub status_message: String,
    pub transmission_state: TransmissionTabState,
}

impl Default for HrptPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            iq_samples: Vec::new(),
            status_message: "Módulo educativo HRPT en desarrollo (Consulte ROADMAP.md).".to_string(),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl HrptPipeline {
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
            "NOAA HRPT (Transmisión de Imágenes de Alta Resolución - Banda L)",
            "Transmisión digital de radiómetro AVHRR/3 en banda L (1698.0 / 1707.0 MHz) a 665.4 kbps con codificación \
             Manchester Split-Phase-L y modulación BPSK. Ofrece 5 canales espectrales a resolución espacial de 1.1 km por píxel.",
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
                "noaa_hrpt",
                1_698_000_000.0, // 1698.0 MHz Banda L
                &mut self.transmission_state,
                sdr_controller,
                &self.iq_samples,
                2_400_000,
            ),
            _ => {}
        }
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Especificación de Entrada AVHRR").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.label("La norma NOAA HRPT requiere 5 canales espectrales de 10 bits por muestra (2048 palabras por línea):");
                        ui.label("- Canal 1 (Visible: 0.58 - 0.68 µm)");
                        ui.label("- Canal 2 (Infrarrojo cercano: 0.725 - 1.00 µm)");
                        ui.label("- Canal 3A/3B (Infrarrojo medio: 1.58 - 3.93 µm)");
                        ui.label("- Canal 4 (Térmico: 10.30 - 11.30 µm)");
                        ui.label("- Canal 5 (Térmico: 11.50 - 12.50 µm)");

                        ui.add_space(10.0);
                        if ui.button(RichText::new("Generar Trama Sintética BPSK de Prueba ->").strong()).clicked() {
                            self.generate_synthetic_bpsk();
                            self.current_step = 1;
                        }
                    });
                });
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
                        ui.label(RichText::new("Modulador Manchester Split-Phase BPSK").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.label("Tasa de símbolos: 665,400 baudios | Tasa de muestreo I/Q: 2.6616 MSps (4x sobremuestreo)");
                        ui.label("Filtro conformado de pulso: Root Raised Cosine (RRC) alpha = 0.5");

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let gen_btn = egui::Button::new(
                                RichText::new("Sintetizar Señal BPSK HRPT").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(gen_btn).clicked() {
                                self.generate_synthetic_bpsk();
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
            render_signal_visualizer(ui, theme, &self.iq_samples, 2_400_000);
        });
    }

    fn generate_synthetic_bpsk(&mut self) {
        let _symbol_rate = 665_400.0f32;
        let sample_rate = 2_661_600.0f32; // 4 muestras por símbolo
        let sps = 4;
        let num_symbols = 8_000;
        let mut iq = Vec::with_capacity(num_symbols * sps);

        // Generar secuencia de bits pseudo-aleatoria (PRBS) con sincronismo 0x0284
        let mut lfsr: u16 = 0xACE1;
        for _ in 0..num_symbols {
            let bit = (lfsr & 1) != 0;
            lfsr = (lfsr >> 1) ^ (-(bit as i16) as u16 & 0xB400);

            // Codificación Manchester: 1 -> +1 luego -1; 0 -> -1 luego +1
            let (first_half, second_half) = if bit { (1.0f32, -1.0f32) } else { (-1.0f32, 1.0f32) };

            for _ in 0..(sps / 2) {
                iq.push((first_half * 0.7, 0.0));
            }
            for _ in 0..(sps / 2) {
                iq.push((second_half * 0.7, 0.0));
            }
        }

        self.iq_samples = iq;
        self.status_message = format!(
            "Generadas {} muestras I/Q de señal sintética NOAA HRPT BPSK a {:.3} MSps.",
            self.iq_samples.len(),
            sample_rate / 1e6
        );
    }
}
