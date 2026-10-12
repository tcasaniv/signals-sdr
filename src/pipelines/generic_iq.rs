use crate::config::AppConfig;
use crate::export::formats::{read_iq_file, IqExportFormat};
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::components::{info_card, step_bar};
use crate::ui::spectrum::render_signal_visualizer;
use crate::ui::transmission::{render_transmission_tab, TransmissionTabState};
use egui::{Color32, Frame, RichText, Stroke, Ui};
use std::path::{Path, PathBuf};

pub struct GenericIqPipeline {
    pub current_step: usize,
    pub input_file_path: Option<PathBuf>,
    pub input_format: IqExportFormat,
    pub sample_rate: u32,
    pub raw_iq_samples: Vec<(f32, f32)>,
    pub processed_iq_samples: Vec<(f32, f32)>,
    // Ajustes de procesamiento
    pub gain_factor: f32,
    pub freq_shift_hz: f32,
    pub remove_dc: bool,
    pub status_message: String,
    pub transmission_state: TransmissionTabState,
}

impl Default for GenericIqPipeline {
    fn default() -> Self {
        Self {
            current_step: 0,
            input_file_path: None,
            input_format: IqExportFormat::WavIq16,
            sample_rate: 288_000,
            raw_iq_samples: Vec::new(),
            processed_iq_samples: Vec::new(),
            gain_factor: 1.0,
            freq_shift_hz: 0.0,
            remove_dc: true,
            status_message: "Listo para cargar archivo de muestras I/Q pregrabadas.".to_string(),
            transmission_state: TransmissionTabState::default(),
        }
    }
}

impl GenericIqPipeline {
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
            "Generic IQ (Gestor y Transmisor de Archivos I/Q)",
            "Permite cargar archivos de señal I/Q existentes en formatos estándar (WAV I/Q estéreo, RAW SC16 bladeRF, \
             RAW int8 HackRF, RAW float32), inspeccionar la envolvente en el osciloscopio y analizador de espectro, \
             aplicar corrección de DC/frecuencia, y transmitirlos directamente con SDR o convertirlos a otro formato.",
        );

        ui.add_space(8.0);
        step_bar(ui, theme, &mut self.current_step, "2. Procesamiento");

        match self.current_step {
            0 => self.render_step_source(ui, theme),
            1 => self.render_step_processing(ui, theme),
            2 => {
                let samples_to_send = if !self.processed_iq_samples.is_empty() {
                    &self.processed_iq_samples
                } else {
                    &self.raw_iq_samples
                };

                render_transmission_tab(
                    ui,
                    theme,
                    config,
                    "generic_iq",
                    144_800_000.0,
                    &mut self.transmission_state,
                    sdr_controller,
                    samples_to_send,
                    self.sample_rate,
                );
            }
            _ => {}
        }
    }

    fn render_step_source(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            Frame::none()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.card_border()))
                .rounding(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Seleccionar Archivo de Muestras I/Q").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            let pick_btn = egui::Button::new("Examinar Archivo I/Q...")
                                .fill(theme.surface_subtle())
                                .stroke(Stroke::new(1.0, theme.card_border()));

                            if ui.add(pick_btn).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Muestras I/Q o WAV", &["wav", "raw", "iq", "bin", "sc16", "dat"])
                                    .pick_file()
                                {
                                    self.input_file_path = Some(path.clone());
                                    // Auto-detección por extensión
                                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                        if ext.eq_ignore_ascii_case("wav") {
                                            self.input_format = IqExportFormat::WavIq16;
                                        }
                                    }
                                    self.load_selected_file();
                                }
                            }

                            if ui.button("Generar Tono I/Q Sintético").clicked() {
                                self.generate_synthetic_iq();
                            }
                        });

                        ui.add_space(8.0);
                        if let Some(path) = &self.input_file_path {
                            ui.label(RichText::new(format!("Archivo: {}", path.display())).monospace().size(12.0));
                        }

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Formato de origen:");
                            egui::ComboBox::from_id_salt("iq_input_format_combo")
                                .selected_text(self.input_format.label())
                                .show_ui(ui, |ui| {
                                    let formats = [
                                        IqExportFormat::WavIq16,
                                        IqExportFormat::RawSc16Q11,
                                        IqExportFormat::RawSc16Full,
                                        IqExportFormat::RawInt8,
                                        IqExportFormat::RawFloat32,
                                        IqExportFormat::WavAudioMono,
                                    ];
                                    for fmt in formats {
                                        if ui.selectable_value(&mut self.input_format, fmt, fmt.label()).clicked() {
                                            if self.input_file_path.is_some() {
                                                self.load_selected_file();
                                            }
                                        }
                                    }
                                });
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Tasa de muestreo (Hz):");
                            ui.add(egui::DragValue::new(&mut self.sample_rate).speed(1000).range(8_000..=20_000_000));
                        });

                        ui.add_space(8.0);
                        ui.label(RichText::new(&self.status_message).size(12.0).color(theme.text_secondary()));
                    });
                });

            ui.add_space(10.0);
            if !self.raw_iq_samples.is_empty() {
                if ui
                    .button(RichText::new("Continuar a Procesamiento y Análisis ->").strong())
                    .clicked()
                {
                    self.process_samples();
                    self.current_step = 1;
                }
            }
        });
    }

    fn render_step_processing(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            Frame::none()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.card_border()))
                .rounding(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Ajustes de Acondicionamiento I/Q").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label("Factor de Escala / Ganancia:");
                            if ui.add(egui::Slider::new(&mut self.gain_factor, 0.1..=5.0).text("x")).changed() {
                                self.process_samples();
                            }
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Desplazamiento de Frecuencia (Shift Hz):");
                            if ui.add(egui::Slider::new(&mut self.freq_shift_hz, -100_000.0..=100_000.0).suffix(" Hz")).changed() {
                                self.process_samples();
                            }
                        });

                        ui.add_space(6.0);
                        if ui.checkbox(&mut self.remove_dc, "Eliminar componente continua (Cancelación DC Offset)").changed() {
                            self.process_samples();
                        }

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let apply_btn = egui::Button::new(
                                RichText::new("Aplicar Procesamiento").strong().color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .rounding(6.0);

                            if ui.add(apply_btn).clicked() {
                                self.process_samples();
                            }

                            if !self.processed_iq_samples.is_empty() {
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
            render_signal_visualizer(ui, theme, &self.processed_iq_samples, self.sample_rate);
        });
    }

    fn load_selected_file(&mut self) {
        if let Some(path) = &self.input_file_path {
            match read_iq_file(path, self.input_format) {
                Ok((samples, detected_sr)) => {
                    self.sample_rate = detected_sr;
                    let count = samples.len();
                    self.raw_iq_samples = samples;
                    self.process_samples();
                    self.status_message = format!(
                        "Archivo cargado exitosamente: {} muestras a {} Hz ({:.2} seg).",
                        count,
                        self.sample_rate,
                        count as f32 / self.sample_rate as f32
                    );
                }
                Err(e) => {
                    self.status_message = format!("Error al cargar archivo I/Q: {}", e);
                }
            }
        }
    }

    fn generate_synthetic_iq(&mut self) {
        let sr = 288_000;
        let count = sr * 2; // 2 segundos
        let mut samples = Vec::with_capacity(count as usize);

        for n in 0..count {
            let t = n as f32 / sr as f32;
            let phase = 2.0 * std::f32::consts::PI * 10_000.0 * t; // Tono a +10 kHz
            let i = phase.cos() * 0.7;
            let q = phase.sin() * 0.7;
            samples.push((i, q));
        }

        self.sample_rate = sr;
        self.input_file_path = Some(PathBuf::from("Sintético (Portadora compleja +10 kHz)"));
        self.raw_iq_samples = samples;
        self.process_samples();
        self.status_message = "Generadas 576,000 muestras I/Q de tono sintético a +10 kHz.".to_string();
    }

    fn process_samples(&mut self) {
        if self.raw_iq_samples.is_empty() {
            self.processed_iq_samples.clear();
            return;
        }

        let mut dc_i = 0.0f32;
        let mut dc_q = 0.0f32;

        if self.remove_dc {
            let count = self.raw_iq_samples.len() as f32;
            let sum_i: f32 = self.raw_iq_samples.iter().map(|s| s.0).sum();
            let sum_q: f32 = self.raw_iq_samples.iter().map(|s| s.1).sum();
            dc_i = sum_i / count;
            dc_q = sum_q / count;
        }

        let shift_angular = 2.0 * std::f32::consts::PI * self.freq_shift_hz / (self.sample_rate as f32);
        let mut processed = Vec::with_capacity(self.raw_iq_samples.len());

        for (idx, &(raw_i, raw_q)) in self.raw_iq_samples.iter().enumerate() {
            let mut i = (raw_i - dc_i) * self.gain_factor;
            let mut q = (raw_q - dc_q) * self.gain_factor;

            if self.freq_shift_hz.abs() > 0.01 {
                let angle = shift_angular * (idx as f32);
                let cos_val = angle.cos();
                let sin_val = angle.sin();
                let rot_i = i * cos_val - q * sin_val;
                let rot_q = i * sin_val + q * cos_val;
                i = rot_i;
                q = rot_q;
            }

            // Normalización suave para evitar saturación del DAC
            processed.push((i.clamp(-1.0, 1.0), q.clamp(-1.0, 1.0)));
        }

        self.processed_iq_samples = processed;
        self.status_message = format!(
            "Procesamiento aplicado: Ganancia {:.2}x, Shift {:.1} Hz, DC Offset {}. Muestras listas: {}",
            self.gain_factor,
            self.freq_shift_hz,
            if self.remove_dc { "eliminado" } else { "mantenido" },
            self.processed_iq_samples.len()
        );
    }
}
