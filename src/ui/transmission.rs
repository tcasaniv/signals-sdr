use crate::config::AppConfig;
use crate::export::formats::{export_iq_file, IqExportFormat};
use crate::sdr::{
    detect_connected_radios, SdrDeviceOption, SdrHardwareType, SdrTransmissionController,
    TransmissionStatus, TxConfig,
};
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
    pub detected_radios: Vec<String>,
    // Estado Archivo I/Q
    pub export_format: IqExportFormat,
    pub output_filename: String,
    pub custom_output_dir: Option<PathBuf>,
    pub export_status_message: Option<(String, bool)>, // (Mensaje, es_error)
}

impl Default for TransmissionTabState {
    fn default() -> Self {
        let devices = SdrDeviceOption::all_supported();
        Self {
            destination: TransmissionDestination::HardwareSdr,
            tx_config: TxConfig::default(),
            available_devices: devices,
            selected_device_idx: 0,
            detected_radios: Vec::new(),
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
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Configuración de Transmisión SDR (Seify / SoapySDR)")
                            .strong()
                            .size(15.0)
                            .color(theme.accent_blue()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("⟳ Detectar SDRs Conectados").size(12.0)).clicked() {
                            state.detected_radios = detect_connected_radios();
                        }
                    });
                });

                if !state.detected_radios.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Radios detectados:").size(11.0).color(theme.text_secondary()));
                        for r in &state.detected_radios {
                            Frame::new()
                                .fill(Color32::from_rgb(22, 101, 52))
                                .corner_radius(4.0)
                                .inner_margin(egui::Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(format!("✓ {}", r)).size(11.0).color(Color32::WHITE));
                                });
                        }
                    });
                }

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
                                        state.tx_config.hw_type = dev.hw_type;
                                        state.tx_config.device_args = dev.default_args.clone();
                                    }
                                }
                            });
                    }
                });

                // Descripción didáctica del SDR seleccionado
                if state.selected_device_idx < state.available_devices.len() {
                    let desc = &state.available_devices[state.selected_device_idx].description;
                    ui.label(RichText::new(desc).size(11.0).color(theme.text_secondary()));
                }

                ui.add_space(8.0);

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
                    ui.label(RichText::new(format!("{} Hz ({:.2} MSps)", sample_rate, sample_rate as f32 / 1e6)).strong());
                });

                // Advertencia de Tasa de Muestreo si no es compatible con el SDR
                if (state.tx_config.hw_type == SdrHardwareType::HackRfSoapy || state.tx_config.hw_type == SdrHardwareType::HackRfSeify) && sample_rate < 1_000_000 {
                    ui.add_space(4.0);
                    Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(234, 179, 8, 30))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(234, 179, 8)))
                        .corner_radius(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("⚠️").size(14.0));
                                ui.label(
                                    RichText::new(format!(
                                        "Tasa de muestreo actual ({} Hz) no admitida por HackRF. Requiere al menos 1,000,000 Hz (1 MSps) en Soapy o 2,000,000 Hz en Seify. Ve al paso 2 y ajusta la tasa de muestreo del pipeline.",
                                        sample_rate
                                    ))
                                    .size(11.5)
                                    .color(Color32::from_rgb(250, 204, 21)),
                                );
                            });
                        });
                }

                ui.add_space(8.0);

                // --- CONTROLES ESPECÍFICOS SEGÚN EL SDR SELECCIONADO ---
                match state.tx_config.hw_type {
                    SdrHardwareType::HackRfSoapy | SdrHardwareType::HackRfSeify => {
                        ui.group(|ui| {
                            ui.label(RichText::new("Opciones de Hardware HackRF One").strong().size(12.0).color(theme.accent_blue()));
                            ui.add_space(4.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label("Ganancia TX (VGA):");
                                ui.add(egui::Slider::new(&mut state.tx_config.hackrf_vga_gain, 0.0..=47.0).text("dB (0-47)"));
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(&mut state.tx_config.hackrf_rf_amp, "Amplificador PA (+14 dB)");
                                ui.label(RichText::new("(¡Precaución! Conectar antena antes de activar)").size(11.0).color(theme.text_secondary()));
                            });

                            ui.add_space(2.0);
                            ui.label(RichText::new("Puerto de antena: TX/RX (Conector SMA principal)").size(11.0).color(theme.text_secondary()));
                        });
                    }
                    SdrHardwareType::UsrpSoapy => {
                        ui.group(|ui| {
                            ui.label(RichText::new("Opciones de Hardware USRP / UHD").strong().size(12.0).color(theme.accent_blue()));
                            ui.add_space(4.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label("Dirección IP o URI:");
                                ui.add(egui::TextEdit::singleline(&mut state.tx_config.usrp_ip).hint_text("ej: 192.168.10.2 (dejar vacío para USB/auto)"));
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Subdispositivo:");
                                ui.add(egui::TextEdit::singleline(&mut state.tx_config.usrp_subdev).hint_text("ej: A:0"));
                                ui.add_space(8.0);
                                ui.label("Antena TX:");
                                ui.add(egui::TextEdit::singleline(&mut state.tx_config.usrp_antenna).hint_text("TX/RX"));
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Ganancia TX (dB):");
                                ui.add(egui::Slider::new(&mut state.tx_config.usrp_gain, 0.0..=31.5).text("dB"));
                            });
                        });
                    }
                    SdrHardwareType::PlutoSdrSoapy => {
                        ui.group(|ui| {
                            ui.label(RichText::new("Opciones de Hardware PlutoSDR (ADALM-Pluto)").strong().size(12.0).color(theme.accent_blue()));
                            ui.add_space(4.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label("URI de Conexión:");
                                ui.add(egui::TextEdit::singleline(&mut state.tx_config.pluto_uri).hint_text("ej: ip:192.168.2.1 o usb: o vacío"));
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Atenuación TX (dB):");
                                ui.add(egui::Slider::new(&mut state.tx_config.pluto_attenuation, -89.75..=0.0).text("dB"));
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Puerto Antena TX:");
                                ui.add(egui::TextEdit::singleline(&mut state.tx_config.pluto_antenna).hint_text("A"));
                            });
                        });
                    }
                    SdrHardwareType::BladeRfSoapy => {
                        ui.group(|ui| {
                            ui.label(RichText::new("Opciones de Hardware bladeRF").strong().size(12.0).color(theme.accent_blue()));
                            ui.add_space(4.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label("Ganancia TX (dB):");
                                ui.add(egui::Slider::new(&mut state.tx_config.bladerf_tx_gain, 0.0..=66.0).text("dB"));
                            });
                        });
                    }
                    SdrHardwareType::LimeSdrSoapy => {
                        ui.group(|ui| {
                            ui.label(RichText::new("Opciones de Hardware LimeSDR").strong().size(12.0).color(theme.accent_blue()));
                            ui.add_space(4.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label("Ganancia TX (PAD):");
                                ui.add(egui::Slider::new(&mut state.tx_config.lime_gain, 0.0..=60.0).text("dB"));
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Antena TX:");
                                egui::ComboBox::from_id_salt("lime_ant_combo")
                                    .selected_text(&state.tx_config.lime_antenna)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut state.tx_config.lime_antenna, "BAND1".to_string(), "BAND1");
                                        ui.selectable_value(&mut state.tx_config.lime_antenna, "BAND2".to_string(), "BAND2");
                                    });
                            });
                        });
                    }
                    SdrHardwareType::Dummy => {
                        Frame::new()
                            .fill(theme.card_bg())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("ℹ️ Modo Simulación en Memoria").strong().size(12.0).color(theme.text_primary()));
                                ui.label(RichText::new("No requiere hardware SDR físico. Procesa y emite las muestras a la tasa configurada hacia un sumidero en memoria, ideal para probar la lógica sin radiar RF.").size(11.0).color(theme.text_secondary()));
                            });
                    }
                    SdrHardwareType::SoapyAuto => {
                        Frame::new()
                            .fill(theme.card_bg())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("🔍 Detección Automática de Radio").strong().size(12.0).color(theme.accent_blue()));
                                ui.label(RichText::new("Al pulsar 'Transmitir con SDR', Seify consultará los módulos instalados de SoapySDR y seleccionará automáticamente el primer transceptor de radio conectado (descartando tarjetas de sonido).").size(11.0).color(theme.text_secondary()));
                            });
                    }
                    SdrHardwareType::Custom => {
                        ui.group(|ui| {
                            ui.label(RichText::new("Argumentos de Conexión Manuales").strong().size(12.0).color(theme.accent_blue()));
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Cadena de argumentos:");
                                ui.add(egui::TextEdit::singleline(&mut state.tx_config.custom_args).hint_text("ej: driver=soapy,soapy_driver=hackrf"));
                            });
                        });
                    }
                }

                ui.add_space(6.0);
                ui.checkbox(&mut state.tx_config.repeat, "Repetir transmisión en bucle continuo");

                ui.add_space(10.0);

                // Estado y Botones de Transmisión
                let is_transmitting = sdr_controller.is_transmitting();
                let status = sdr_controller.status();

                ui.horizontal(|ui| {
                    if is_transmitting {
                        let stop_btn = egui::Button::new(
                            RichText::new("⏹ Detener Transmisión").strong().color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(220, 38, 38))
                        .corner_radius(6.0);

                        if ui.add(stop_btn).clicked() {
                            sdr_controller.stop();
                        }
                    } else {
                        let tx_btn = egui::Button::new(
                            RichText::new("⚡ Transmitir con SDR").strong().color(Color32::WHITE),
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

                ui.add_space(8.0);

                // Telemetría en vivo y Alertas Didácticas
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
                        Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(185, 28, 28, 25))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(220, 38, 38)))
                            .corner_radius(8.0)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("⛔ Error en Transmisión SDR").strong().size(13.0).color(Color32::from_rgb(248, 113, 113)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("Descartar").clicked() {
                                                sdr_controller.clear_error();
                                            }
                                        });
                                    });
                                    ui.add_space(4.0);
                                    ui.label(RichText::new(&err).size(12.0).color(Color32::from_rgb(254, 202, 202)));
                                    ui.add_space(4.0);

                                    // Sugerencia didáctica contextual
                                    if err.contains("incompatible") || err.contains("Tasa de muestreo") {
                                        ui.label(RichText::new("💡 Solución sugerida: El hardware HackRF One no soporta tasas de muestreo bajas. Ve al paso '2. Codificar' y genera la señal con una tasa de muestreo de al menos 1,000,000 Hz (1 MSps).").size(11.0).color(Color32::from_rgb(253, 230, 138)));
                                    } else if err.contains("No se encontró") {
                                        ui.label(RichText::new("💡 Solución sugerida: Conecta el cable USB del SDR a un puerto directo (evita concentradores sin alimentación) y presiona 'Detectar SDRs Conectados'. En Windows, asegúrate de haber instalado los controladores de SoapySDR o WinUSB/Zadig.").size(11.0).color(Color32::from_rgb(253, 230, 138)));
                                    } else if err.contains("utilizando") || err.contains("ocupado") {
                                        ui.label(RichText::new("💡 Solución sugerida: Cierra cualquier otro software que pueda estar accediendo al dispositivo (SDR#, GQRX, CubicSDR, GNU Radio, etc.) y reintenta.").size(11.0).color(Color32::from_rgb(253, 230, 138)));
                                    }
                                });
                            });
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
