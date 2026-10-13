use crate::config::AppConfig;
use crate::export::formats::{export_iq_file, IqExportFormat};
use crate::sdr::{SdrDeviceOption, SdrTransmissionController, TransmissionStatus, TxConfig};
use crate::theme::AppTheme;
use egui::{Color32, Frame, RichText, Stroke, Ui};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmissionDestination {
    HardwareSdr,
    IqFile,
}

#[derive(Debug, Clone)]
pub struct TransmissionTabState {
    pub destination: TransmissionDestination,
    // Estado SDR
    pub tx_config: TxConfig,
    pub available_devices: Vec<SdrDeviceOption>,
    pub selected_device_idx: usize,
    // Estado Archivo I/Q
    pub export_format: IqExportFormat,
    pub output_filename: String,
    pub custom_output_dir: Option<PathBuf>,
    pub export_status_message: Option<(String, bool)>, // (Mensaje, es_error)
}

impl Default for TransmissionTabState {
    fn default() -> Self {
        Self {
            destination: TransmissionDestination::HardwareSdr,
            tx_config: TxConfig::default(),
            available_devices: SdrDeviceOption::all_supported(),
            selected_device_idx: 0,
            export_format: IqExportFormat::WavIq16,
            output_filename: "senal_iq".to_string(),
            custom_output_dir: None,
            export_status_message: None,
        }
    }
}

impl TransmissionTabState {
    pub fn ensure_devices_probed(&mut self) {
        if self.available_devices.is_empty() {
            self.available_devices = SdrDeviceOption::all_supported();
        }
    }
}

/// Renderiza la subpestaña "3. Transmisión"
pub fn render_transmission_tab(
    ui: &mut Ui,
    theme: &AppTheme,
    config: &AppConfig,
    pipeline_slug: &str,
    default_rf_freq_hz: f64,
    state: &mut TransmissionTabState,
    sdr_controller: &mut SdrTransmissionController,
    samples: &[(f32, f32)],
    sample_rate: u32,
) {
    state.ensure_devices_probed();

    ui.vertical(|ui| {
        // Selector de destino: Hardware SDR vs Archivo IQ con soporte de wrap
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Destino de la señal:").strong().size(14.0).color(theme.text_primary()));
            ui.add_space(8.0);

            let sdr_selected = state.destination == TransmissionDestination::HardwareSdr;
            let file_selected = state.destination == TransmissionDestination::IqFile;

            if ui
                .selectable_label(sdr_selected, RichText::new("Transmitir con Hardware SDR").strong())
                .clicked()
            {
                state.destination = TransmissionDestination::HardwareSdr;
            }

            if ui
                .selectable_label(file_selected, RichText::new("Guardar como Archivo I/Q").strong())
                .clicked()
            {
                state.destination = TransmissionDestination::IqFile;
            }
        });

        ui.add_space(10.0);

        match state.destination {
            TransmissionDestination::HardwareSdr => {
                render_sdr_panel(
                    ui,
                    theme,
                    default_rf_freq_hz,
                    state,
                    sdr_controller,
                    samples,
                    sample_rate,
                );
            }
            TransmissionDestination::IqFile => {
                render_file_panel(
                    ui,
                    theme,
                    config,
                    pipeline_slug,
                    state,
                    samples,
                    sample_rate,
                );
            }
        }

        ui.add_space(10.0);
        crate::ui::spectrum::render_signal_visualizer(ui, theme, samples, sample_rate);
    });
}

fn render_sdr_panel(
    ui: &mut Ui,
    theme: &AppTheme,
    default_rf_freq_hz: f64,
    state: &mut TransmissionTabState,
    sdr_controller: &mut SdrTransmissionController,
    samples: &[(f32, f32)],
    sample_rate: u32,
) {
    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Configuración de Transmisión SDR (Seify)")
                        .strong()
                        .size(15.0)
                        .color(theme.accent_blue()),
                );
                ui.add_space(8.0);

                // Selector de dispositivo con soporte wrap
                ui.horizontal_wrapped(|ui| {
                    ui.label("Dispositivo SDR:");
                    if !state.available_devices.is_empty() {
                        let curr_name = &state.available_devices[state.selected_device_idx].display_name;
                        egui::ComboBox::from_id_salt("sdr_dev_combo")
                            .selected_text(curr_name)
                            .show_ui(ui, |ui| {
                                for (idx, dev) in state.available_devices.iter().enumerate() {
                                    if ui.selectable_value(&mut state.selected_device_idx, idx, &dev.display_name).clicked() {
                                        state.tx_config.device_args = dev.args.clone();
                                    }
                                }
                            });
                    }
                });

                ui.add_space(6.0);

                // Frecuencia RF en MHz
                if state.tx_config.frequency_hz == 0.0 || state.tx_config.frequency_hz == 137_100_000.0 {
                    state.tx_config.frequency_hz = default_rf_freq_hz;
                }

                let mut freq_mhz = state.tx_config.frequency_hz / 1_000_000.0;
                ui.horizontal_wrapped(|ui| {
                    ui.label("Frecuencia Central RF (MHz):");
                    if ui.add(egui::DragValue::new(&mut freq_mhz).speed(0.01).range(0.5..=6000.0)).changed() {
                        state.tx_config.frequency_hz = freq_mhz * 1_000_000.0;
                    }
                    if ui.button("Restablecer Frecuencia").clicked() {
                        state.tx_config.frequency_hz = default_rf_freq_hz;
                    }
                });

                ui.add_space(6.0);

                // Tasa de muestreo
                ui.horizontal_wrapped(|ui| {
                    ui.label("Tasa de Muestreo SDR (Hz):");
                    state.tx_config.sample_rate_hz = sample_rate;
                    ui.label(RichText::new(format!("{} Hz", sample_rate)).strong());
                });

                ui.add_space(6.0);

                // Ganancia TX
                ui.horizontal_wrapped(|ui| {
                    ui.label("Ganancia TX (dB):");
                    ui.add(egui::Slider::new(&mut state.tx_config.gain_db, 0.0..=47.0).text("dB"));
                });

                ui.add_space(6.0);

                // Interruptor de Amplificador de Potencia (PA) con texto conciso
                ui.horizontal_wrapped(|ui| {
                    ui.label("Amplificador PA:");
                    ui.checkbox(&mut state.tx_config.rf_amp, "Habilitar +14 dB (HackRF / hardware compatible)");
                });

                ui.add_space(10.0);

                // Estado y Botones de Transmisión
                let is_transmitting = sdr_controller.is_transmitting();
                let status = sdr_controller.status();

                ui.horizontal(|ui| {
                    if is_transmitting {
                        let stop_btn = egui::Button::new(
                            RichText::new("Detener Transmisión").strong().color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(220, 38, 38))
                        .corner_radius(6.0);

                        if ui.add(stop_btn).clicked() {
                            sdr_controller.stop();
                        }
                    } else {
                        let tx_btn = egui::Button::new(
                            RichText::new("Transmitir con SDR").strong().color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(6.0);

                        let enabled = !samples.is_empty();
                        if ui.add_enabled(enabled, tx_btn).clicked() {
                            sdr_controller.start(state.tx_config.clone(), samples.to_vec());
                        }

                        if samples.is_empty() {
                            ui.label(RichText::new("(Primero genera la señal en el paso 2)").size(12.0).color(theme.text_secondary()));
                        }
                    }
                });

                ui.add_space(6.0);

                // Telemetría en vivo de transmisión
                match status {
                    TransmissionStatus::Transmitting { samples_sent, total_samples, elapsed_sec } => {
                        let prog = (samples_sent as f32 / total_samples.max(1) as f32).clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(prog).show_percentage());
                        ui.label(
                            RichText::new(format!(
                                "Transmitiendo: {} / {} muestras ({:.1}s)",
                                samples_sent, total_samples, elapsed_sec
                            ))
                            .size(12.0)
                            .color(theme.accent_blue()),
                        );
                    }
                    TransmissionStatus::Finished { total_samples, elapsed_sec } => {
                        ui.label(
                            RichText::new(format!(
                                "Transmisión finalizada con éxito: {} muestras en {:.1}s",
                                total_samples, elapsed_sec
                            ))
                            .size(12.0)
                            .color(Color32::from_rgb(34, 197, 94)),
                        );
                    }
                    TransmissionStatus::Error(err) => {
                        ui.label(RichText::new(format!("Error en transmisión: {}", err)).size(12.0).color(Color32::from_rgb(239, 68, 68)));
                    }
                    TransmissionStatus::Idle => {}
                }
            });
        });
}

fn render_file_panel(
    ui: &mut Ui,
    theme: &AppTheme,
    config: &AppConfig,
    pipeline_slug: &str,
    state: &mut TransmissionTabState,
    samples: &[(f32, f32)],
    sample_rate: u32,
) {
    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Exportación a Archivo I/Q")
                        .strong()
                        .size(15.0)
                        .color(theme.accent_blue()),
                );
                ui.add_space(8.0);

                // Formato de salida
                ui.horizontal_wrapped(|ui| {
                    ui.label("Formato de salida I/Q:");
                    egui::ComboBox::from_id_salt("export_format_combo")
                        .selected_text(state.export_format.name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.export_format, IqExportFormat::WavIq16, IqExportFormat::WavIq16.name());
                            ui.selectable_value(&mut state.export_format, IqExportFormat::RawSc16Q11, IqExportFormat::RawSc16Q11.name());
                            ui.selectable_value(&mut state.export_format, IqExportFormat::RawSc16Full, IqExportFormat::RawSc16Full.name());
                            ui.selectable_value(&mut state.export_format, IqExportFormat::RawInt8, IqExportFormat::RawInt8.name());
                            ui.selectable_value(&mut state.export_format, IqExportFormat::RawFloat32, IqExportFormat::RawFloat32.name());
                            ui.selectable_value(&mut state.export_format, IqExportFormat::WavAudioMono, IqExportFormat::WavAudioMono.name());
                        });
                });

                ui.add_space(6.0);

                // Nombre del archivo
                ui.horizontal_wrapped(|ui| {
                    ui.label("Nombre del archivo:");
                    ui.text_edit_singleline(&mut state.output_filename);
                    ui.label(format!(".{}", state.export_format.default_extension()));
                });

                ui.add_space(6.0);

                // Carpeta destino (usando la carpeta del pipeline en Ajustes por defecto)
                let target_dir = state
                    .custom_output_dir
                    .clone()
                    .unwrap_or_else(|| config.get_pipeline_dir(pipeline_slug));

                ui.horizontal_wrapped(|ui| {
                    ui.label("Carpeta destino:");
                    ui.add(
                        egui::Label::new(
                            RichText::new(target_dir.display().to_string())
                                .monospace()
                                .size(11.0)
                                .color(theme.text_secondary()),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(4.0);
                if ui.button("Cambiar Carpeta Destino...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        state.custom_output_dir = Some(folder);
                    }
                }

                ui.add_space(10.0);

                // Botón Guardar Archivo I/Q
                let save_btn = egui::Button::new(
                    RichText::new("Guardar Archivo I/Q").strong().color(Color32::WHITE),
                )
                .fill(theme.accent_blue())
                .corner_radius(6.0);

                let can_save = !samples.is_empty();
                if ui.add_enabled(can_save, save_btn).clicked() {
                    let file_path = target_dir.join(format!("{}.{}", state.output_filename, state.export_format.default_extension()));
                    match export_iq_file(&file_path, samples, sample_rate, state.export_format) {
                        Ok(_) => {
                            state.export_status_message = Some((
                                format!("Archivo guardado exitosamente en: {}", file_path.display()),
                                false,
                            ));
                        }
                        Err(e) => {
                            state.export_status_message = Some((
                                format!("Error guardando archivo: {}", e),
                                true,
                            ));
                        }
                    }
                }

                if samples.is_empty() {
                    ui.label(RichText::new("(Primero genera la señal en el paso 2)").size(12.0).color(theme.text_secondary()));
                }

                if let Some((ref msg, is_err)) = state.export_status_message {
                    ui.add_space(6.0);
                    let color = if is_err {
                        Color32::from_rgb(239, 68, 68)
                    } else {
                        Color32::from_rgb(34, 197, 94)
                    };
                    ui.label(RichText::new(msg).size(12.0).color(color));
                }
            });
        });
}
