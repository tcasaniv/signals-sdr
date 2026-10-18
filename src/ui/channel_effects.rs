use crate::channel::*;
use crate::config::AppConfig;
use crate::theme::AppTheme;
use crate::ui::components::info_card;
use crate::ui::spectrogram::render_spectrogram_widget;
use crate::ui::spectrum::render_signal_visualizer;
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};
use std::time::Instant;

/// Renderiza la subpestaña "3. Efectos de canal" para cualquier pipeline
pub fn render_channel_effects_tab(
    ui: &mut Ui,
    theme: &AppTheme,
    config: &AppConfig,
    pipeline_id: &str,
    sample_rate: u32,
    carrier_freq_hz: f64,
    original_iq: &[(f32, f32)],
    state: &mut ChannelEffectsState,
    mut on_continue_to_tx: impl FnMut(),
) {
    ui.vertical(|ui| {
        // 1. Selector Superior de Modo (Pills): Sin efectos vs Con efectos de canal
        ui.horizontal(|ui| {
            ui.label(RichText::new("Modo de Canal:").strong().size(13.5).color(theme.text_primary()));
            ui.add_space(8.0);

            let is_without = state.active_tab == ChannelEffectsTab::WithoutEffects;
            let is_with = state.active_tab == ChannelEffectsTab::WithEffects;

            let without_btn = egui::Button::new(
                RichText::new("  Sin efectos (Señal Pura / Laboratorio)  ")
                    .size(13.0)
                    .strong()
                    .color(if is_without { Color32::WHITE } else { theme.text_secondary() }),
            )
            .fill(if is_without { theme.accent_blue() } else { theme.surface_subtle() })
            .stroke(Stroke::new(1.0_f32, if is_without { Color32::TRANSPARENT } else { theme.card_border() }))
            .corner_radius(16.0);

            if ui.add(without_btn).clicked() {
                state.active_tab = ChannelEffectsTab::WithoutEffects;
            }

            let with_btn = egui::Button::new(
                RichText::new("  Con efectos de canal (Simulación Realista)  ")
                    .size(13.0)
                    .strong()
                    .color(if is_with { Color32::WHITE } else { theme.text_secondary() }),
            )
            .fill(if is_with { theme.accent_blue() } else { theme.surface_subtle() })
            .stroke(Stroke::new(1.0_f32, if is_with { Color32::TRANSPARENT } else { theme.card_border() }))
            .corner_radius(16.0);

            if ui.add(with_btn).clicked() {
                state.active_tab = ChannelEffectsTab::WithEffects;
                // Si aún no se han procesado muestras y está sucio, procesar
                if state.processed_iq.is_empty() && !original_iq.is_empty() {
                    let start = Instant::now();
                    state.processed_iq = apply_channel_effects(original_iq, sample_rate, &state.params);
                    state.last_processing_ms = start.elapsed().as_secs_f32() * 1000.0;
                    state.is_dirty = false;
                }
            }
        });

        ui.add_space(10.0);

        match state.active_tab {
            ChannelEffectsTab::WithoutEffects => {
                render_without_effects_view(
                    ui,
                    theme,
                    config,
                    pipeline_id,
                    sample_rate,
                    original_iq,
                    state,
                    &mut on_continue_to_tx,
                );
            }
            ChannelEffectsTab::WithEffects => {
                render_with_effects_view(
                    ui,
                    theme,
                    config,
                    pipeline_id,
                    sample_rate,
                    carrier_freq_hz,
                    original_iq,
                    state,
                    &mut on_continue_to_tx,
                );
            }
        }
    });
}

fn render_without_effects_view(
    ui: &mut Ui,
    theme: &AppTheme,
    config: &AppConfig,
    pipeline_id: &str,
    sample_rate: u32,
    original_iq: &[(f32, f32)],
    state: &mut ChannelEffectsState,
    on_continue_to_tx: &mut impl FnMut(),
) {
    info_card(
        ui,
        theme,
        "Modo Sin Efectos de Canal (Señal Pura Ideal)",
        "Las muestras I/Q generadas por el modulador no sufren alteración alguna. La transmisión por SDR o exportación a archivo mantendrá la máxima pureza espectral teórica y cero distorsión de propagación.\n\n\
         ¿Cuándo utilizar este modo?\n\
         • Pruebas de RF con cable coaxial directo y atenuador fijo (evita saturación del frontend).\n\
         • Benchmarking y validación de algoritmos de codificación/modulación pura.\n\
         • Entornos controlados de laboratorio o calibración instrumental.",
    );

    ui.add_space(12.0);

    if !original_iq.is_empty() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Muestras I/Q disponibles: {} | Tasa: {} Sps | Duración: {:.2} s",
                    original_iq.len(),
                    sample_rate,
                    original_iq.len() as f32 / sample_rate as f32
                ))
                .size(12.5)
                .color(theme.accent_green()),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new("Continuar a Transmisión ->").strong().size(13.0)).clicked() {
                    on_continue_to_tx();
                }
            });
        });

        ui.add_space(8.0);
        render_signal_visualizer(ui, theme, original_iq, sample_rate);

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let spec_btn_text = if state.spectrogram_state.show {
                "Ocultar Espectro"
            } else {
                "Mostrar Espectro"
            };
            let spec_btn = egui::Button::new(RichText::new(spec_btn_text).strong().size(12.5))
                .fill(if state.spectrogram_state.show { theme.card_border() } else { theme.surface_subtle() })
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .min_size(Vec2::new(0.0, 32.0));

            if ui.add(spec_btn).clicked() {
                state.spectrogram_state.show = !state.spectrogram_state.show;
            }
        });

        if state.spectrogram_state.show {
            ui.add_space(8.0);
            let out_dir = config.get_pipeline_dir(pipeline_id);
            render_spectrogram_widget(
                ui,
                theme,
                &mut state.spectrogram_state,
                original_iq,
                sample_rate,
                pipeline_id,
                Some(&out_dir),
            );
        }
    } else {
        ui.colored_label(
            theme.text_secondary(),
            "No hay muestras I/Q generadas aún. Regrese a la subpestaña anterior para modular la señal.",
        );
    }
}

fn render_with_effects_view(
    ui: &mut Ui,
    theme: &AppTheme,
    config: &AppConfig,
    pipeline_id: &str,
    sample_rate: u32,
    carrier_freq_hz: f64,
    original_iq: &[(f32, f32)],
    state: &mut ChannelEffectsState,
    on_continue_to_tx: &mut impl FnMut(),
) {
    let carrier_mhz = (carrier_freq_hz / 1e6) as f32;

    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Fila de selección de Presets contextuales
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Preset de Escenario Realista:").strong().size(13.5).color(theme.accent_blue()));
                    ui.add_space(6.0);

                    let available_presets = ChannelPreset::for_pipeline(pipeline_id);
                    let current_name = state.params.active_preset.display_name();

                    egui::ComboBox::from_id_salt("channel_preset_combo")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for preset in available_presets {
                                let is_selected = state.params.active_preset == *preset;
                                if ui.selectable_label(is_selected, preset.display_name()).clicked() {
                                    state.params = ChannelSimulationParams::from_preset(*preset, carrier_mhz);
                                    state.is_dirty = true;
                                }
                            }
                        });
                });

                ui.add_space(6.0);
                // Descripción didáctica del preset actual
                info_card(
                    ui,
                    theme,
                    state.params.active_preset.display_name(),
                    state.params.active_preset.description(),
                );

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(RichText::new("Parámetros Físicos del Canal:").strong().size(13.5).color(theme.text_primary()));
                ui.add_space(6.0);

                // --- 1. AWGN (Ruido Blanco Gaussiano Aditivo) ---
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let prev = state.params.enable_awgn;
                        if ui.checkbox(&mut state.params.enable_awgn, RichText::new("Ruido Blanco Gaussiano Térmico (AWGN)").strong()).changed() {
                            if prev != state.params.enable_awgn {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        }

                        if state.params.enable_awgn {
                            let snr = state.params.snr_db;
                            let (badge_txt, badge_color) = if snr >= 20.0 {
                                ("Excelente (> 20 dB)", theme.accent_green())
                            } else if snr >= 12.0 {
                                ("Buena (12-20 dB)", theme.accent_blue())
                            } else if snr >= 6.0 {
                                ("Aceptable (6-12 dB)", Color32::from_rgb(234, 179, 8))
                            } else {
                                ("Crítica / Marginal (< 6 dB)", Color32::from_rgb(239, 68, 68))
                            };
                            ui.label(RichText::new(format!("[ Calidad: {} ]", badge_txt)).size(11.5).color(badge_color));
                        }
                    });

                    ui.label(RichText::new(
                        "• Origen: Agitación térmica en antena receptora y etapa LNA.\n\
                         • Impacto: Añade piso de ruido constante. Causa 'nieve' en imágenes analógicas o dispersión de constelación y errores de bit en digitales."
                    ).size(11.5).color(theme.text_secondary()));

                    if state.params.enable_awgn {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Relación Señal/Ruido (SNR):");
                            if ui.add(egui::Slider::new(&mut state.params.snr_db, -5.0..=40.0).suffix(" dB")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });
                    }
                });

                ui.add_space(6.0);

                // --- 2. Efecto Doppler dinámico ---
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let prev = state.params.enable_doppler;
                        if ui.checkbox(&mut state.params.enable_doppler, RichText::new("Efecto Doppler Dinámico").strong()).changed() {
                            if prev != state.params.enable_doppler {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        }
                    });

                    ui.label(RichText::new(
                        "• Origen: Velocidad relativa entre el transmisor orbital o móvil y la estación receptora.\n\
                         • Impacto: Desplaza la portadora en el espectrograma (curva S). Requiere bucle de enganche AFC/PLL en el receptor para no perder sintonía."
                    ).size(11.5).color(theme.text_secondary()));

                    if state.params.enable_doppler {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Modo Doppler:");
                            let current_mode = match state.params.doppler_mode {
                                DopplerMode::LeoPass => "Pase Satelital LEO (Curva S)",
                                DopplerMode::LinearDrift => "Deriva Lineal Continua",
                                DopplerMode::StaticOffset => "Desplazamiento Estático (Offset)",
                            };
                            egui::ComboBox::from_id_salt("doppler_mode_combo")
                                .selected_text(current_mode)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(state.params.doppler_mode == DopplerMode::LeoPass, "Pase Satelital LEO (Curva S)").clicked() {
                                        state.params.doppler_mode = DopplerMode::LeoPass;
                                        state.params.active_preset = ChannelPreset::Custom;
                                        state.is_dirty = true;
                                    }
                                    if ui.selectable_label(state.params.doppler_mode == DopplerMode::LinearDrift, "Deriva Lineal Continua").clicked() {
                                        state.params.doppler_mode = DopplerMode::LinearDrift;
                                        state.params.active_preset = ChannelPreset::Custom;
                                        state.is_dirty = true;
                                    }
                                    if ui.selectable_label(state.params.doppler_mode == DopplerMode::StaticOffset, "Desplazamiento Estático (Offset)").clicked() {
                                        state.params.doppler_mode = DopplerMode::StaticOffset;
                                        state.params.active_preset = ChannelPreset::Custom;
                                        state.is_dirty = true;
                                    }
                                });

                            ui.add_space(10.0);
                            ui.label("Desviación Máxima (Delta f):");
                            if ui.add(egui::Slider::new(&mut state.params.max_doppler_hz, 0.0..=50000.0).suffix(" Hz")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });
                    }
                });

                ui.add_space(6.0);

                // --- 3. Pérdida por Trayectoria (FSPL) y Atenuación ---
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let prev = state.params.enable_path_loss;
                        if ui.checkbox(&mut state.params.enable_path_loss, RichText::new("Atenuación Geométrica (FSPL) y Atmosférica").strong()).changed() {
                            if prev != state.params.enable_path_loss {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        }

                        if state.params.enable_path_loss {
                            // Cálculo informativo de FSPL
                            let fspl_db = 20.0 * state.params.distance_km.log10() + 20.0 * state.params.carrier_freq_mhz.max(1.0).log10() + 32.44;
                            ui.label(RichText::new(format!("[ FSPL teórico: {:.1} dB ]", fspl_db)).size(11.5).color(theme.text_secondary()));
                        }
                    });

                    ui.label(RichText::new(
                        "• Origen: Dispersión geométrica tridimensional de energía en el espacio y absorción por gases y lluvia.\n\
                         • Impacto: Reduce la amplitud de la señal recibida según la distancia y frecuencia portadora."
                    ).size(11.5).color(theme.text_secondary()));

                    if state.params.enable_path_loss {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Distancia de Enlace:");
                            if ui.add(egui::Slider::new(&mut state.params.distance_km, 1.0..=3000.0).suffix(" km")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }

                            ui.add_space(10.0);
                            ui.label("Pérdida Atmosférica:");
                            if ui.add(egui::Slider::new(&mut state.params.atmospheric_loss_db, 0.0..=10.0).suffix(" dB")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Atenuación Directa Normalizada:");
                            if ui.add(egui::Slider::new(&mut state.params.direct_attenuation_db, -40.0..=0.0).suffix(" dB")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });
                    }
                });

                ui.add_space(6.0);

                // --- 4. Desvanecimiento por Multitrayecto (Rician / Rayleigh) ---
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let prev = state.params.enable_fading;
                        if ui.checkbox(&mut state.params.enable_fading, RichText::new("Desvanecimiento por Multitrayecto (Rician / Rayleigh)").strong()).changed() {
                            if prev != state.params.enable_fading {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        }

                        if state.params.enable_fading {
                            let k = state.params.rician_k_db;
                            let mode_str = if k <= 0.5 {
                                "Rayleigh Puro (Sin línea de vista NLOS)"
                            } else if k >= 15.0 {
                                "Rician Fuerte (Visión directa dominante LOS)"
                            } else {
                                "Rician Moderado"
                            };
                            ui.label(RichText::new(format!("[ {} ]", mode_str)).size(11.5).color(theme.text_secondary()));
                        }
                    });

                    ui.label(RichText::new(
                        "• Origen: Ondas que rebotan en edificios, terreno o ionosfera interfiriendo en fase aleatoria.\n\
                         • Impacto: Produce caídas periódicas profundas en la envolvente ('fades') o nulos de portadora."
                    ).size(11.5).color(theme.text_secondary()));

                    if state.params.enable_fading {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Factor K de Rician:");
                            if ui.add(egui::Slider::new(&mut state.params.rician_k_db, 0.0..=25.0).suffix(" dB")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }

                            ui.add_space(10.0);
                            ui.label("Tasa de Fading (fm):");
                            if ui.add(egui::Slider::new(&mut state.params.fading_rate_hz, 0.1..=30.0).suffix(" Hz")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });
                    }
                });

                ui.add_space(6.0);

                // --- 5. Margen de Ruido (Pre y Post Padding) ---
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let prev = state.params.enable_noise_padding;
                        if ui.checkbox(&mut state.params.enable_noise_padding, RichText::new("Margen de Ruido de Canal (Pre y Post)").strong()).changed() {
                            if prev != state.params.enable_noise_padding {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        }
                    });

                    ui.label(RichText::new(
                        "• Origen: Ruido ambiental presente antes de encender la portadora y tras apagarse la emisión.\n\
                         • Impacto: Añade segundos de ruido blanco con transición suave (Hann). Ideal para probar silenciamiento (squelch)."
                    ).size(11.5).color(theme.text_secondary()));

                    if state.params.enable_noise_padding {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Ruido Previo (Apertura):");
                            if ui.add(egui::Slider::new(&mut state.params.pre_padding_sec, 0.0..=5.0).suffix(" s")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }

                            ui.add_space(10.0);
                            ui.label("Ruido Posterior (Cierre):");
                            if ui.add(egui::Slider::new(&mut state.params.post_padding_sec, 0.0..=5.0).suffix(" s")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });
                    }
                });

                ui.add_space(6.0);

                // --- 6. Imperfecciones de Front-End RF (Opcional) ---
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let prev = state.params.enable_hardware_imp;
                        if ui.checkbox(&mut state.params.enable_hardware_imp, RichText::new("Imperfecciones de Front-End SDR (Desbalance I/Q y Fuga DC)").strong()).changed() {
                            if prev != state.params.enable_hardware_imp {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        }
                    });

                    ui.label(RichText::new(
                        "• Origen: Desbalance analógico en mezcladores en cuadratura y fuga de oscilador local (LO leakage).\n\
                         • Impacto: Genera una imagen especular no deseada en -f y un pico parásito en el centro exacto (DC spike)."
                    ).size(11.5).color(theme.text_secondary()));

                    if state.params.enable_hardware_imp {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Desbalance I/Q:");
                            if ui.add(egui::Slider::new(&mut state.params.iq_imbalance_db, -2.0..=2.0).suffix(" dB")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }

                            ui.add_space(10.0);
                            ui.label("Error de Fase:");
                            if ui.add(egui::Slider::new(&mut state.params.phase_imbalance_deg, -10.0..=10.0).suffix(" °")).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Offset DC (I):");
                            if ui.add(egui::Slider::new(&mut state.params.dc_offset_i, -0.05..=0.05)).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }

                            ui.add_space(10.0);
                            ui.label("Offset DC (Q):");
                            if ui.add(egui::Slider::new(&mut state.params.dc_offset_q, -0.05..=0.05)).changed() {
                                state.params.active_preset = ChannelPreset::Custom;
                                state.is_dirty = true;
                            }
                        });
                    }
                });

                ui.add_space(12.0);

                // Botón de cálculo y feedback
                ui.horizontal(|ui| {
                    let apply_btn = egui::Button::new(RichText::new("  ⟳ Aplicar Efectos de Canal  ").strong().size(13.5).color(Color32::WHITE))
                        .fill(theme.accent_blue())
                        .corner_radius(6.0)
                        .min_size(Vec2::new(0.0, 32.0));

                    if ui.add(apply_btn).clicked() {
                        let start = Instant::now();
                        state.processed_iq = apply_channel_effects(original_iq, sample_rate, &state.params);
                        state.last_processing_ms = start.elapsed().as_secs_f32() * 1000.0;
                        state.is_dirty = false;
                    }

                    ui.add_space(8.0);
                    if state.is_dirty {
                        ui.label(RichText::new("⚠ Hay cambios de canal pendientes de aplicar.").color(Color32::from_rgb(234, 179, 8)).strong());
                    } else if !state.processed_iq.is_empty() {
                        ui.label(
                            RichText::new(format!(
                                "✓ Efectos aplicados en {:.1} ms (Muestras totales: {})",
                                state.last_processing_ms,
                                state.processed_iq.len()
                            ))
                            .color(theme.accent_green())
                            .strong(),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Continuar a Transmisión ->").strong().size(13.0)).clicked() {
                            // Si está sucio o vacío, asegurarse de computar antes de ir a tx
                            if state.processed_iq.is_empty() || state.is_dirty {
                                let start = Instant::now();
                                state.processed_iq = apply_channel_effects(original_iq, sample_rate, &state.params);
                                state.last_processing_ms = start.elapsed().as_secs_f32() * 1000.0;
                                state.is_dirty = false;
                            }
                            on_continue_to_tx();
                        }
                    });
                });
            });
        });

    // 3. Visualizador de la señal resultante con efectos
    let effective_samples = if !state.processed_iq.is_empty() {
        &state.processed_iq
    } else {
        original_iq
    };

    if !effective_samples.is_empty() {
        ui.add_space(10.0);
        render_signal_visualizer(ui, theme, effective_samples, sample_rate);

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let spec_btn_text = if state.spectrogram_state.show {
                "Ocultar Espectro"
            } else {
                "Mostrar Espectro"
            };
            let spec_btn = egui::Button::new(RichText::new(spec_btn_text).strong().size(12.5))
                .fill(if state.spectrogram_state.show { theme.card_border() } else { theme.surface_subtle() })
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .min_size(Vec2::new(0.0, 32.0));

            if ui.add(spec_btn).clicked() {
                state.spectrogram_state.show = !state.spectrogram_state.show;
            }
        });

        if state.spectrogram_state.show {
            ui.add_space(8.0);
            let out_dir = config.get_pipeline_dir(pipeline_id);
            render_spectrogram_widget(
                ui,
                theme,
                &mut state.spectrogram_state,
                effective_samples,
                sample_rate,
                pipeline_id,
                Some(&out_dir),
            );
        }
    }
}
