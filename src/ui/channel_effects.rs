use crate::channel::*;
use crate::config::AppConfig;
use crate::theme::AppTheme;
use crate::ui::components::{accordion_section, info_card, AccordionOptions};
use crate::ui::spectrogram::render_spectrogram_widget;
use crate::ui::spectrum::render_signal_visualizer;
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use web_time::Instant;

/// Calcula efectos de canal en background sin bloquear la interfaz.
/// En Desktop corre sobre un hilo secundario del SO; en WASM mediante tarea asíncrona spawn_local.
/// No trunca ni reduce la cantidad de muestras de la señal original.
fn dispatch_compute_channel_iq(
    original: &[(f32, f32)],
    sample_rate: u32,
    params: &ChannelSimulationParams,
    result_arc: Arc<Mutex<Option<(Vec<(f32, f32)>, f32)>>>,
    ctx: egui::Context,
) {
    if original.is_empty() {
        if let Ok(mut g) = result_arc.lock() {
            *g = Some((Vec::new(), 0.0));
        }
        return;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let slice: Vec<(f32, f32)> = original.to_vec();
        let params_owned = params.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let t0 = js_sys::Date::now();
            let out = apply_channel_effects(&slice, sample_rate, &params_owned);
            let elapsed_ms = (js_sys::Date::now() - t0) as f32;
            if let Ok(mut g) = result_arc.lock() {
                *g = Some((out, elapsed_ms));
            }
            ctx.request_repaint();
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let slice: Vec<(f32, f32)> = original.to_vec();
        let params_owned = params.clone();
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let out = apply_channel_effects(&slice, sample_rate, &params_owned);
            let elapsed = t0.elapsed().as_secs_f32() * 1000.0;
            if let Ok(mut g) = result_arc.lock() {
                *g = Some((out, elapsed));
            }
            ctx.request_repaint();
        });
    }
}

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
    // Recoger resultado del cómputo asíncrono si ya terminó
    if state.is_computing {
        if state.poll_compute_result() {
            if state.pending_continue_to_tx {
                state.pending_continue_to_tx = false;
                on_continue_to_tx();
            }
        }
        ui.ctx().request_repaint();
    }

    ui.vertical(|ui| {
        // 1. Selector Superior de Modo (Pills): Sin efectos vs Con efectos de canal
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Modo de Canal:")
                    .strong()
                    .size(13.5)
                    .color(theme.text_primary()),
            );
            ui.add_space(8.0);

            let is_without = state.active_tab == ChannelEffectsTab::WithoutEffects;
            let is_with = state.active_tab == ChannelEffectsTab::WithEffects;

            let without_btn =
                egui::Button::new(RichText::new("  Sin efectos  ").size(13.0).strong().color(
                    if is_without {
                        Color32::WHITE
                    } else {
                        theme.text_secondary()
                    },
                ))
                .fill(if is_without {
                    theme.accent_blue()
                } else {
                    theme.surface_subtle()
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if is_without {
                        Color32::TRANSPARENT
                    } else {
                        theme.card_border()
                    },
                ))
                .corner_radius(16.0);

            if ui.add(without_btn).clicked() {
                state.active_tab = ChannelEffectsTab::WithoutEffects;
            }

            let with_btn = egui::Button::new(
                RichText::new("  Con efectos de canal  ")
                    .size(13.0)
                    .strong()
                    .color(if is_with {
                        Color32::WHITE
                    } else {
                        theme.text_secondary()
                    }),
            )
            .fill(if is_with {
                theme.accent_blue()
            } else {
                theme.surface_subtle()
            })
            .stroke(Stroke::new(
                1.0_f32,
                if is_with {
                    Color32::TRANSPARENT
                } else {
                    theme.card_border()
                },
            ))
            .corner_radius(16.0);

            if ui.add(with_btn).clicked() {
                state.active_tab = ChannelEffectsTab::WithEffects;
                // Disparar cómputo asíncrono si no hay muestras procesadas
                if state.processed_iq.is_empty() && !original_iq.is_empty() && !state.is_computing {
                    state.is_computing = true;
                    state.is_dirty = false;
                    dispatch_compute_channel_iq(
                        original_iq,
                        sample_rate,
                        &state.params,
                        Arc::clone(&state.compute_result),
                        ui.ctx().clone(),
                    );
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
        "Modo Sin Efectos de Canal",
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
                if ui
                    .button(
                        RichText::new("Continuar a Transmisión ->")
                            .strong()
                            .size(13.0),
                    )
                    .clicked()
                {
                    on_continue_to_tx();
                }
            });
        });

        ui.add_space(8.0);
        accordion_section(
            ui,
            theme,
            "without_eff_osc",
            AccordionOptions::new("Osciloscopio I/Q (Dominio del Tiempo)").default_open(false),
            |ui| {
                render_signal_visualizer(ui, theme, original_iq, sample_rate);
            },
        );

        ui.add_space(8.0);
        accordion_section(
            ui,
            theme,
            "without_eff_spec",
            AccordionOptions::new("Espectrograma / Cascada RF").default_open(false),
            |ui| {
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
            },
        );
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
                // Fila de selección de Presets contextuales (adaptable para pantallas estrechas)
                let available_presets = ChannelPreset::for_pipeline(pipeline_id);
                let current_name = state.params.active_preset.display_name();

                if ui.available_width() < 460.0 {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Preset de Escenario Realista:")
                                .strong()
                                .size(13.0)
                                .color(theme.accent_blue()),
                        );
                        ui.add_space(2.0);
                        egui::ComboBox::from_id_salt("channel_preset_combo")
                            .selected_text(current_name)
                            .width(ui.available_width() - 8.0)
                            .show_ui(ui, |ui| {
                                for preset in available_presets {
                                    let is_selected = state.params.active_preset == *preset;
                                    if ui
                                        .selectable_label(is_selected, preset.display_name())
                                        .clicked()
                                    {
                                        state.params =
                                            ChannelSimulationParams::from_preset(*preset, carrier_mhz);
                                        state.is_dirty = true;
                                    }
                                }
                            });
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Preset de Escenario Realista:")
                                .strong()
                                .size(13.5)
                                .color(theme.accent_blue()),
                        );
                        ui.add_space(6.0);

                        egui::ComboBox::from_id_salt("channel_preset_combo")
                            .selected_text(current_name)
                            .show_ui(ui, |ui| {
                                for preset in available_presets {
                                    let is_selected = state.params.active_preset == *preset;
                                    if ui
                                        .selectable_label(is_selected, preset.display_name())
                                        .clicked()
                                    {
                                        state.params =
                                            ChannelSimulationParams::from_preset(*preset, carrier_mhz);
                                        state.is_dirty = true;
                                    }
                                }
                            });
                    });
                }

                ui.add_space(6.0);
                // Descripción didáctica del preset actual
                info_card(
                    ui,
                    theme,
                    state.params.active_preset.display_name(),
                    state.params.active_preset.description(),
                );
            });
        });

    ui.add_space(8.0);

    ui.label(
        RichText::new("Efectos Individuales del Canal de Transmisión:")
            .strong()
            .size(13.5)
            .color(theme.text_primary()),
    );
    ui.add_space(6.0);

    // --- 1. AWGN (Ruido Blanco Gaussiano Aditivo) ---
    let prev_awgn = state.params.enable_awgn;
    let awgn_badge = if state.params.enable_awgn {
        let snr = state.params.snr_db;
        if snr >= 20.0 {
            Some(("Excelente (> 20 dB)", theme.accent_green()))
        } else if snr >= 12.0 {
            Some(("Buena (12-20 dB)", theme.accent_blue()))
        } else if snr >= 6.0 {
            Some(("Aceptable (6-12 dB)", Color32::from_rgb(234, 179, 8)))
        } else {
            Some(("Crítica (< 6 dB)", Color32::from_rgb(239, 68, 68)))
        }
    } else {
        None
    };

    let mut awgn_opt = AccordionOptions::new("Ruido Blanco Gaussiano Térmico (AWGN)")
        .with_toggle(&mut state.params.enable_awgn)
        .default_open(false);
    if let Some((b_text, b_col)) = awgn_badge {
        awgn_opt = awgn_opt.with_badge(b_text, b_col);
    }

    accordion_section(ui, theme, "ch_eff_awgn", awgn_opt, |ui| {
        ui.label(RichText::new(
                        "• Origen: Agitación térmica en antena receptora y etapa LNA.\n\
                         • Impacto: Añade piso de ruido constante. Causa 'nieve' en imágenes analógicas o dispersión de constelación y errores de bit en digitales."
                    ).size(11.5).color(theme.text_secondary()));

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Relación Señal/Ruido (SNR):");
            if ui
                .add(egui::Slider::new(&mut state.params.snr_db, -5.0..=40.0).suffix(" dB"))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });
    });

    if prev_awgn != state.params.enable_awgn {
        state.params.active_preset = ChannelPreset::Custom;
        state.is_dirty = true;
    }

    ui.add_space(6.0);

    // --- 2. Efecto Doppler dinámico ---
    let prev_doppler = state.params.enable_doppler;
    let doppler_mode_str = match state.params.doppler_mode {
        DopplerMode::LeoPass => "Pase LEO (Curva S)",
        DopplerMode::LinearDrift => "Deriva Lineal",
        DopplerMode::StaticOffset => "Offset Estático",
    };
    let doppler_badge = if state.params.enable_doppler {
        Some((doppler_mode_str, theme.accent_blue()))
    } else {
        None
    };

    let mut doppler_opt = AccordionOptions::new("Efecto Doppler Dinámico")
        .with_toggle(&mut state.params.enable_doppler)
        .default_open(false);
    if let Some((b_text, b_col)) = doppler_badge {
        doppler_opt = doppler_opt.with_badge(b_text, b_col);
    }

    accordion_section(ui, theme, "ch_eff_doppler", doppler_opt, |ui| {
        ui.label(RichText::new(
                        "• Origen: Velocidad relativa entre el transmisor orbital o móvil y la estación receptora.\n\
                         • Impacto: Desplaza la portadora en el espectrograma (curva S). Requiere bucle de enganche AFC/PLL en el receptor para no perder sintonía."
                    ).size(11.5).color(theme.text_secondary()));

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Modo Doppler:");
            let current_mode = match state.params.doppler_mode {
                DopplerMode::LeoPass => "Pase Satelital LEO (Curva S)",
                DopplerMode::LinearDrift => "Deriva Lineal Continua",
                DopplerMode::StaticOffset => "Desplazamiento Estático (Offset)",
            };
            egui::ComboBox::from_id_salt("doppler_mode_combo")
                .selected_text(current_mode)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            state.params.doppler_mode == DopplerMode::LeoPass,
                            "Pase Satelital LEO (Curva S)",
                        )
                        .clicked()
                    {
                        state.params.doppler_mode = DopplerMode::LeoPass;
                        state.params.active_preset = ChannelPreset::Custom;
                        state.is_dirty = true;
                    }
                    if ui
                        .selectable_label(
                            state.params.doppler_mode == DopplerMode::LinearDrift,
                            "Deriva Lineal Continua",
                        )
                        .clicked()
                    {
                        state.params.doppler_mode = DopplerMode::LinearDrift;
                        state.params.active_preset = ChannelPreset::Custom;
                        state.is_dirty = true;
                    }
                    if ui
                        .selectable_label(
                            state.params.doppler_mode == DopplerMode::StaticOffset,
                            "Desplazamiento Estático (Offset)",
                        )
                        .clicked()
                    {
                        state.params.doppler_mode = DopplerMode::StaticOffset;
                        state.params.active_preset = ChannelPreset::Custom;
                        state.is_dirty = true;
                    }
                });
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Desviación Máxima (Delta f):");
            if ui
                .add(
                    egui::Slider::new(&mut state.params.max_doppler_hz, 0.0..=50000.0)
                        .suffix(" Hz"),
                )
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });
    });

    if prev_doppler != state.params.enable_doppler {
        state.params.active_preset = ChannelPreset::Custom;
        state.is_dirty = true;
    }

    ui.add_space(6.0);

    // --- 3. Pérdida por Trayectoria (FSPL) y Atenuación ---
    let prev_path_loss = state.params.enable_path_loss;
    let fspl_db = 20.0 * state.params.distance_km.log10()
        + 20.0 * state.params.carrier_freq_mhz.max(1.0).log10()
        + 32.44;
    let fspl_str = format!("FSPL: {:.1} dB", fspl_db);
    let path_loss_badge = if state.params.enable_path_loss {
        Some((fspl_str.as_str(), theme.text_secondary()))
    } else {
        None
    };

    let mut path_loss_opt = AccordionOptions::new("Atenuación Geométrica (FSPL) y Atmosférica")
        .with_toggle(&mut state.params.enable_path_loss)
        .default_open(false);
    if let Some((b_text, b_col)) = path_loss_badge {
        path_loss_opt = path_loss_opt.with_badge(b_text, b_col);
    }

    accordion_section(ui, theme, "ch_eff_path_loss", path_loss_opt, |ui| {
        ui.label(RichText::new(
                        "• Origen: Dispersión geométrica tridimensional de energía en el espacio y absorción por gases y lluvia.\n\
                         • Impacto: Reduce la amplitud de la señal recibida según la distancia y frecuencia portadora."
                    ).size(11.5).color(theme.text_secondary()));

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Distancia de Enlace:");
            if ui
                .add(egui::Slider::new(&mut state.params.distance_km, 1.0..=3000.0).suffix(" km"))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Pérdida Atmosférica:");
            if ui
                .add(
                    egui::Slider::new(&mut state.params.atmospheric_loss_db, 0.0..=10.0)
                        .suffix(" dB"),
                )
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Atenuación Directa Normalizada:");
            if ui
                .add(
                    egui::Slider::new(&mut state.params.direct_attenuation_db, -40.0..=0.0)
                        .suffix(" dB"),
                )
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });
    });

    if prev_path_loss != state.params.enable_path_loss {
        state.params.active_preset = ChannelPreset::Custom;
        state.is_dirty = true;
    }

    ui.add_space(6.0);

    // --- 4. Desvanecimiento por Multitrayecto (Rician / Rayleigh) ---
    let prev_fading = state.params.enable_fading;
    let fading_str = if state.params.rician_k_db <= 0.5 {
        "Rayleigh (NLOS)"
    } else if state.params.rician_k_db >= 15.0 {
        "Rician (LOS Dominante)"
    } else {
        "Rician Moderado"
    };
    let fading_badge = if state.params.enable_fading {
        Some((fading_str, theme.accent_blue()))
    } else {
        None
    };

    let mut fading_opt =
        AccordionOptions::new("Desvanecimiento por Multitrayecto (Rician / Rayleigh)")
            .with_toggle(&mut state.params.enable_fading)
            .default_open(false);
    if let Some((b_text, b_col)) = fading_badge {
        fading_opt = fading_opt.with_badge(b_text, b_col);
    }

    accordion_section(ui, theme, "ch_eff_fading", fading_opt, |ui| {
        ui.label(RichText::new(
                        "• Origen: Ondas que rebotan en edificios, terreno o ionosfera interfiriendo en fase aleatoria.\n\
                         • Impacto: Produce caídas periódicas profundas en la envolvente ('fades') o nulos de portadora."
                    ).size(11.5).color(theme.text_secondary()));

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Factor K de Rician:");
            if ui
                .add(egui::Slider::new(&mut state.params.rician_k_db, 0.0..=25.0).suffix(" dB"))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Tasa de Fading (fm):");
            if ui
                .add(egui::Slider::new(&mut state.params.fading_rate_hz, 0.1..=30.0).suffix(" Hz"))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });
    });

    if prev_fading != state.params.enable_fading {
        state.params.active_preset = ChannelPreset::Custom;
        state.is_dirty = true;
    }

    ui.add_space(6.0);

    // --- 5. Margen de Ruido (Pre y Post Padding) ---
    let prev_padding = state.params.enable_noise_padding;
    let padding_badge = if state.params.enable_noise_padding {
        Some(("Activo", theme.accent_green()))
    } else {
        None
    };

    let mut padding_opt = AccordionOptions::new("Margen de Ruido de Canal (Pre y Post Padding)")
        .with_toggle(&mut state.params.enable_noise_padding)
        .default_open(false);
    if let Some((b_text, b_col)) = padding_badge {
        padding_opt = padding_opt.with_badge(b_text, b_col);
    }

    accordion_section(ui, theme, "ch_eff_padding", padding_opt, |ui| {
        ui.label(RichText::new(
                        "• Origen: Ruido ambiental presente antes de encender la portadora y tras apagarse la emisión.\n\
                         • Impacto: Añade segundos de ruido blanco con transición suave (Hann). Ideal para probar silenciamiento (squelch)."
                    ).size(11.5).color(theme.text_secondary()));

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Ruido Previo (Apertura):");
            if ui
                .add(egui::Slider::new(&mut state.params.pre_padding_sec, 0.0..=5.0).suffix(" s"))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Ruido Posterior (Cierre):");
            if ui
                .add(egui::Slider::new(&mut state.params.post_padding_sec, 0.0..=5.0).suffix(" s"))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });
    });

    if prev_padding != state.params.enable_noise_padding {
        state.params.active_preset = ChannelPreset::Custom;
        state.is_dirty = true;
    }

    ui.add_space(6.0);

    // --- 6. Imperfecciones de Front-End RF (Opcional) ---
    let prev_hw = state.params.enable_hardware_imp;
    let hw_badge = if state.params.enable_hardware_imp {
        Some(("Desbalance I/Q y DC", theme.accent_blue()))
    } else {
        None
    };

    let mut hw_opt =
        AccordionOptions::new("Imperfecciones de Front-End SDR (Desbalance I/Q y Fuga DC)")
            .with_toggle(&mut state.params.enable_hardware_imp)
            .default_open(false);
    if let Some((b_text, b_col)) = hw_badge {
        hw_opt = hw_opt.with_badge(b_text, b_col);
    }

    accordion_section(ui, theme, "ch_eff_hw", hw_opt, |ui| {
        ui.label(RichText::new(
                        "• Origen: Desbalance analógico en mezcladores en cuadratura y fuga de oscilador local (LO leakage).\n\
                         • Impacto: Genera una imagen especular no deseada en -f y un pico parásito en el centro exacto (DC spike)."
                    ).size(11.5).color(theme.text_secondary()));

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Desbalance I/Q:");
            if ui
                .add(egui::Slider::new(&mut state.params.iq_imbalance_db, -2.0..=2.0).suffix(" dB"))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Error de Fase:");
            if ui
                .add(
                    egui::Slider::new(&mut state.params.phase_imbalance_deg, -10.0..=10.0)
                        .suffix(" °"),
                )
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Offset DC (I):");
            if ui
                .add(egui::Slider::new(
                    &mut state.params.dc_offset_i,
                    -0.05..=0.05,
                ))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Offset DC (Q):");
            if ui
                .add(egui::Slider::new(
                    &mut state.params.dc_offset_q,
                    -0.05..=0.05,
                ))
                .changed()
            {
                state.params.active_preset = ChannelPreset::Custom;
                state.is_dirty = true;
            }
        });
    });

    if prev_hw != state.params.enable_hardware_imp {
        state.params.active_preset = ChannelPreset::Custom;
        state.is_dirty = true;
    }

    ui.add_space(12.0);

    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            // Botón de cálculo y feedback
            ui.horizontal_wrapped(|ui| {
                let is_computing = state.is_computing;

                // Botón Aplicar — deshabilitado mientras computa
                let apply_label = if is_computing {
                    format!("  {} Calculando...  ", crate::icons::ICON_REFRESH.codepoint)
                } else {
                    format!("  {} Aplicar Efectos de Canal  ", crate::icons::ICON_REFRESH.codepoint)
                };
                let apply_btn = egui::Button::new(
                    RichText::new(apply_label)
                        .strong()
                        .size(13.5)
                        .color(Color32::WHITE),
                )
                .fill(if is_computing { theme.surface_subtle() } else { theme.accent_blue() })
                .corner_radius(6.0)
                .min_size(Vec2::new(0.0, 32.0));

                if ui.add_enabled(!is_computing, apply_btn).clicked() {
                    state.is_computing = true;
                    state.is_dirty = false;
                    dispatch_compute_channel_iq(
                        original_iq,
                        sample_rate,
                        &state.params,
                        Arc::clone(&state.compute_result),
                        ui.ctx().clone(),
                    );
                }

                ui.add_space(8.0);
                if is_computing {
                    ui.label(
                        RichText::new(format!(
                            "{} Calculando efectos de canal en segundo plano...",
                            crate::icons::ICON_REFRESH.codepoint
                        ))
                        .color(theme.accent_blue())
                        .strong(),
                    );
                } else if state.is_dirty {
                    ui.label(
                        RichText::new(format!(
                            "{} Hay cambios de canal pendientes de aplicar.",
                            crate::icons::ICON_WARNING.codepoint
                        ))
                        .color(Color32::from_rgb(234, 179, 8))
                        .strong(),
                    );
                } else if !state.processed_iq.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "{} Efectos aplicados en {:.1} ms (Muestras procesadas: {})",
                            crate::icons::ICON_CHECK_CIRCLE.codepoint,
                            state.last_processing_ms,
                            state.processed_iq.len()
                        ))
                        .color(theme.accent_green())
                        .strong(),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let next_label = format!(
                        "Continuar a Transmisión {}",
                        crate::icons::ICON_ARROW_FORWARD.codepoint
                    );
                    let btn_text = if state.is_computing {
                        format!("{} Calculando...", crate::icons::ICON_REFRESH.codepoint)
                    } else {
                        next_label
                    };
                    if ui
                        .add_enabled(
                            !state.is_computing,
                            egui::Button::new(
                                RichText::new(btn_text)
                                    .strong()
                                    .size(13.0),
                            ),
                        )
                        .clicked()
                    {
                        // Si está sucio o vacío, disparar cómputo íntegro en segundo plano
                        if (state.processed_iq.is_empty() || state.is_dirty) && !state.is_computing {
                            state.is_computing = true;
                            state.is_dirty = false;
                            state.pending_continue_to_tx = true;
                            dispatch_compute_channel_iq(
                                original_iq,
                                sample_rate,
                                &state.params,
                                Arc::clone(&state.compute_result),
                                ui.ctx().clone(),
                            );
                        } else if !state.is_computing {
                            on_continue_to_tx();
                        }
                    }
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
        accordion_section(
            ui,
            theme,
            "with_eff_osc",
            AccordionOptions::new("Osciloscopio I/Q (Dominio del Tiempo)").default_open(false),
            |ui| {
                render_signal_visualizer(ui, theme, effective_samples, sample_rate);
            },
        );

        ui.add_space(8.0);
        accordion_section(
            ui,
            theme,
            "with_eff_spec",
            AccordionOptions::new("Espectrograma / Cascada RF").default_open(false),
            |ui| {
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
            },
        );
    }
}
