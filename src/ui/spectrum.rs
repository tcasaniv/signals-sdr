use crate::theme::AppTheme;
use crate::ui::spectrogram::{
    draw_dashed_line_horizontal, draw_dashed_line_vertical, draw_line_horizontal,
    draw_line_segment, draw_line_vertical, draw_rect_filled, draw_rect_outline, draw_text_on_image,
};
use egui::{Color32, Frame, Pos2, RichText, Stroke, Ui, Vec2};

#[derive(Debug, Clone, PartialEq)]
pub struct ScopeControls {
    pub zoom_x: f32,  // 1.0 a 50_000.0x
    pub pan_x: f32,   // 0.0 a 1.0
    pub scale_y: f32, // 0.1 a 100.0x
    pub status_message: Option<(String, bool)>, // (mensaje, es_error)
}

impl Default for ScopeControls {
    fn default() -> Self {
        Self {
            zoom_x: 1.0,
            pan_x: 0.0,
            scale_y: 1.0,
            status_message: None,
        }
    }
}

/// Helper para calcular el rango de muestras visible
pub fn get_visible_window(total_len: usize, zoom_x: f32, pan_x: f32) -> (usize, usize) {
    if total_len == 0 {
        return (0, 0);
    }
    let z = zoom_x.max(1.0);
    let visible_count = ((total_len as f32) / z).max(16.0).min(total_len as f32) as usize;
    let max_start = if total_len > visible_count { total_len - visible_count } else { 0 };
    let start_idx = ((pan_x.clamp(0.0, 1.0) * max_start as f32).round() as usize).min(max_start);
    let end_idx = (start_idx + visible_count).min(total_len);
    (start_idx, end_idx)
}

/// Genera una imagen completa en alta definición (1600x800 px) del oscilograma con ejes X/Y, marcas, leyenda y cuadrícula
pub fn render_high_res_scope_chart(
    samples_i: &[f32],
    samples_q: Option<&[f32]>,
    sample_rate: u32,
    title: &str,
    zoom_x: f32,
    pan_x: f32,
    scale_y: f32,
) -> image::RgbaImage {
    let img_w = 1600u32;
    let img_h = 800u32;

    let bg_color = image::Rgba([13, 17, 23, 255]);
    let mut chart = image::RgbaImage::from_pixel(img_w, img_h, bg_color);

    let graph_x = 110u32;
    let graph_y = 90u32;
    let graph_w = 1440u32;
    let graph_h = 630u32;

    let (start_idx, end_idx) = get_visible_window(samples_i.len(), zoom_x, pan_x);
    let visible_i = if !samples_i.is_empty() {
        &samples_i[start_idx..end_idx]
    } else {
        &[]
    };
    let visible_len = visible_i.len();

    let t_start = if sample_rate > 0 {
        start_idx as f32 / sample_rate as f32
    } else {
        0.0
    };
    let t_end = if sample_rate > 0 {
        end_idx as f32 / sample_rate as f32
    } else {
        0.0
    };
    let t_span = t_end - t_start;
    let total_dur = if sample_rate > 0 {
        samples_i.len() as f32 / sample_rate as f32
    } else {
        0.0
    };

    // Fondo del área gráfica
    draw_rect_filled(&mut chart, graph_x, graph_y, graph_w, graph_h, [18, 24, 34, 255]);

    // Líneas de cuadrícula
    let center_y = graph_y + graph_h / 2;
    let half_h = graph_h as f32 * 0.44;

    let y_top = (center_y as f32 - (1.0 * scale_y).clamp(0.0, 1.0) * half_h).round() as u32;
    let y_bot = (center_y as f32 + (1.0 * scale_y).clamp(0.0, 1.0) * half_h).round() as u32;

    draw_dashed_line_horizontal(&mut chart, graph_x, graph_x + graph_w, y_top, [255, 255, 255, 45]);
    draw_line_horizontal(&mut chart, graph_x, graph_x + graph_w, center_y, [255, 255, 255, 100]);
    draw_dashed_line_horizontal(&mut chart, graph_x, graph_x + graph_w, y_bot, [255, 255, 255, 45]);

    draw_dashed_line_vertical(&mut chart, graph_x + graph_w / 4, graph_y, graph_y + graph_h, [255, 255, 255, 30]);
    draw_dashed_line_vertical(&mut chart, graph_x + graph_w / 2, graph_y, graph_y + graph_h, [255, 255, 255, 40]);
    draw_dashed_line_vertical(&mut chart, graph_x + (3 * graph_w) / 4, graph_y, graph_y + graph_h, [255, 255, 255, 30]);

    // Borde del gráfico
    draw_rect_outline(&mut chart, graph_x, graph_y, graph_w, graph_h, [60, 75, 95, 255]);

    // Encabezado y Metadatos
    let title_text = format!("SIGNALS SDR - {}", title.to_uppercase());
    draw_text_on_image(&mut chart, graph_x, 24, &title_text, [240, 246, 252, 255], 2);

    let meta_text = format!(
        "Fs: {} Hz | Duración Total: {:.3}s ({} muestras) | Ventana: [{:.4}s - {:.4}s] ({:.4}s, {} muestras) | Zoom X: {:.1}x | Escala Y: {:.1}x",
        sample_rate, total_dur, samples_i.len(), t_start, t_end, t_span, visible_len, zoom_x, scale_y
    );
    draw_text_on_image(&mut chart, graph_x, 54, &meta_text, [139, 148, 158, 255], 1);

    // Leyenda de Canales
    if samples_q.is_some() {
        draw_rect_filled(&mut chart, 1200, 28, 14, 14, [88, 166, 255, 255]);
        draw_text_on_image(&mut chart, 1222, 28, "Canal I (En fase)", [88, 166, 255, 255], 1);

        draw_rect_filled(&mut chart, 1380, 28, 14, 14, [245, 140, 30, 255]);
        draw_text_on_image(&mut chart, 1402, 28, "Canal Q (Cuadratura)", [245, 140, 30, 255], 1);
    } else {
        draw_rect_filled(&mut chart, 1320, 28, 14, 14, [88, 166, 255, 255]);
        draw_text_on_image(&mut chart, 1342, 28, "Señal de Audio", [88, 166, 255, 255], 1);
    }

    // Eje Y: Marcas de Amplitud
    let y_ticks = [
        (y_top, "+1.0"),
        (center_y, " 0.0"),
        (y_bot, "-1.0"),
    ];
    for (y_pos, lbl) in y_ticks {
        let y_clamped = y_pos.min(graph_y + graph_h - 1);
        draw_line_horizontal(&mut chart, graph_x - 8, graph_x, y_clamped, [160, 175, 195, 255]);
        let lbl_y = if y_pos >= 4 { y_pos - 4 } else { 0 };
        draw_text_on_image(&mut chart, 36, lbl_y, lbl, [201, 209, 217, 255], 1);
    }
    draw_text_on_image(&mut chart, 20, graph_y - 20, "AMPLITUD", [88, 166, 255, 255], 1);

    // Eje X: Marcas de Tiempo
    let x_ticks = [
        (graph_x, format!("{:.4}s", t_start)),
        (graph_x + graph_w / 4, format!("{:.4}s", t_start + t_span * 0.25)),
        (graph_x + graph_w / 2, format!("{:.4}s", t_start + t_span * 0.50)),
        (graph_x + (3 * graph_w) / 4, format!("{:.4}s", t_start + t_span * 0.75)),
        (graph_x + graph_w, format!("{:.4}s", t_end)),
    ];
    for (x_pos, lbl) in x_ticks {
        let x_clamped = x_pos.min(graph_x + graph_w - 1);
        draw_line_vertical(&mut chart, x_clamped, graph_y + graph_h, graph_y + graph_h + 8, [160, 175, 195, 255]);
        let lbl_x = if x_pos >= 26 { x_pos - 24 } else { 0 };
        draw_text_on_image(&mut chart, lbl_x, graph_y + graph_h + 14, &lbl, [201, 209, 217, 255], 1);
    }
    draw_text_on_image(&mut chart, graph_x + graph_w - 140, graph_y + graph_h + 34, "TIEMPO (s) ->", [88, 166, 255, 255], 1);

    // Trazo de Forma de Onda
    let col_i = [88, 166, 255, 255];
    let col_q = [245, 140, 30, 255];

    let visible_q = samples_q.map(|q| &q[start_idx..end_idx]);
    let spp = visible_len as f32 / graph_w as f32;

    if spp <= 2.5 {
        // Trazado de líneas de alta resolución punto a punto
        let mut prev_i: Option<(i32, i32)> = None;
        let mut prev_q: Option<(i32, i32)> = None;

        for k in 0..visible_len {
            let x_pos = graph_x as i32 + ((k as f32 / (visible_len - 1).max(1) as f32) * graph_w as f32).round() as i32;
            let val_i = visible_i[k];
            let y_i = center_y as i32 - ((val_i * scale_y).clamp(-1.0, 1.0) * half_h).round() as i32;

            if let Some((px, py)) = prev_i {
                draw_line_segment(&mut chart, px, py, x_pos, y_i, col_i);
                draw_line_segment(&mut chart, px, py + 1, x_pos, y_i + 1, col_i);
            }
            prev_i = Some((x_pos, y_i));

            if let Some(q_slice) = visible_q {
                let val_q = q_slice[k];
                let y_q = center_y as i32 - ((val_q * scale_y).clamp(-1.0, 1.0) * half_h).round() as i32;

                if let Some((px, py)) = prev_q {
                    draw_line_segment(&mut chart, px, py, x_pos, y_q, col_q);
                }
                prev_q = Some((x_pos, y_q));
            }
        }
    } else {
        // Modo envolvente Min/Max por columna
        for x_idx in 0..graph_w {
            let s_start = ((x_idx as f32) * spp) as usize;
            let s_end = (((x_idx + 1) as f32) * spp) as usize;
            let s_end_clamped = s_end.max(s_start + 1).min(visible_len);
            if s_start >= visible_len {
                break;
            }

            let mut min_i = 0.0_f32;
            let mut max_i = 0.0_f32;
            let mut min_q = 0.0_f32;
            let mut max_q = 0.0_f32;

            for idx in s_start..s_end_clamped {
                let vi = visible_i[idx];
                if vi < min_i { min_i = vi; }
                if vi > max_i { max_i = vi; }

                if let Some(q_slice) = visible_q {
                    let vq = q_slice[idx];
                    if vq < min_q { min_q = vq; }
                    if vq > max_q { max_q = vq; }
                }
            }

            let x_pixel = (graph_x + x_idx) as i32;
            let y_top_i = center_y as i32 - ((max_i * scale_y).clamp(-1.0, 1.0) * half_h).round() as i32;
            let y_bot_i = center_y as i32 - ((min_i * scale_y).clamp(-1.0, 1.0) * half_h).round() as i32;
            draw_line_segment(&mut chart, x_pixel, y_top_i, x_pixel, y_bot_i.max(y_top_i + 1), col_i);

            if visible_q.is_some() {
                let y_top_q = center_y as i32 - ((max_q * scale_y).clamp(-1.0, 1.0) * half_h).round() as i32;
                let y_bot_q = center_y as i32 - ((min_q * scale_y).clamp(-1.0, 1.0) * half_h).round() as i32;
                draw_line_segment(&mut chart, x_pixel, y_top_q, x_pixel, y_bot_q.max(y_top_q + 1), col_q);
            }
        }
    }

    chart
}

/// Visualizador gráfico de forma de onda (osciloscopio) de señal I/Q con Zoom X, Pan X y Escala Y interactiva
pub fn render_signal_visualizer(
    ui: &mut Ui,
    theme: &AppTheme,
    samples: &[(f32, f32)],
    sample_rate: u32,
) {
    let scope_id = ui.make_persistent_id("iq_scope_controls");
    let mut state = ui.data_mut(|d| d.get_temp_mut_or_default::<ScopeControls>(scope_id).clone());

    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Cabecera con título y métricas totales
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Visualizador de Señal I/Q (Osciloscopio en Cuadratura)")
                            .strong()
                            .size(14.0)
                            .color(theme.text_primary()),
                    );

                    let total_dur = if sample_rate > 0 {
                        samples.len() as f32 / sample_rate as f32
                    } else {
                        0.0
                    };

                    ui.label(
                        RichText::new(format!(
                            "Total: {} muestras | {:.2}s a {} Hz",
                            samples.len(),
                            total_dur,
                            sample_rate
                        ))
                        .size(11.5)
                        .color(theme.text_secondary()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_label = format!("{} Guardar Oscilograma (.png)", crate::icons::ICON_DOWNLOAD.codepoint);
                        let save_btn = egui::Button::new(
                            RichText::new(btn_label)
                                .strong()
                                .size(11.5)
                                .color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(6.0);

                        if ui.add(save_btn).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("oscilograma_iq.png")
                                .add_filter("Imagen PNG", &["png"])
                                .save_file()
                            {
                                if let Some(parent) = path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                let (samples_i, samples_q): (Vec<f32>, Vec<f32>) =
                                    samples.iter().cloned().unzip();
                                let chart = render_high_res_scope_chart(
                                    &samples_i,
                                    Some(&samples_q),
                                    sample_rate,
                                    "Oscilograma de Señal I/Q",
                                    state.zoom_x,
                                    state.pan_x,
                                    state.scale_y,
                                );
                                match chart.save(&path) {
                                    Ok(_) => {
                                        state.status_message = Some((
                                            format!("Oscilograma guardado con éxito: {}", path.display()),
                                            false,
                                        ));
                                    }
                                    Err(e) => {
                                        state.status_message = Some((
                                            format!("Error al guardar oscilograma: {}", e),
                                            true,
                                        ));
                                    }
                                }
                            }
                        }
                    });
                });

                if let Some((ref msg, is_err)) = state.status_message {
                    ui.add_space(4.0);
                    let col = if is_err {
                        Color32::from_rgb(239, 68, 68)
                    } else {
                        Color32::from_rgb(34, 197, 94)
                    };
                    ui.label(RichText::new(msg).size(11.5).color(col));
                }

                ui.add_space(6.0);

                // Barra de herramientas de Zoom y Escala interactiva
                render_scope_toolbar(ui, theme, &mut state, samples.len(), sample_rate);

                ui.add_space(6.0);

                let display_h = 135.0;
                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), display_h),
                    egui::Sense::click_and_drag(),
                );

                // Interacción con mouse en el osciloscopio: rueda para zoom X, arrastre para desplazamiento X
                handle_scope_mouse_interaction(&resp, ui, &mut state);

                let bg_fill = if *theme == AppTheme::Dark {
                    Color32::from_rgb(12, 16, 24)
                } else {
                    Color32::from_rgb(238, 244, 252)
                };
                ui.painter().rect_filled(rect, 4.0, bg_fill);
                ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);

                if samples.is_empty() {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Genera o carga una señal para visualizar su forma de onda en cuadratura",
                        egui::FontId::proportional(12.0),
                        theme.text_secondary(),
                    );
                    return;
                }

                // Calcular ventana de muestras visible según zoom X y pan X
                let (start_idx, end_idx) = get_visible_window(samples.len(), state.zoom_x, state.pan_x);
                let visible_samples = &samples[start_idx..end_idx];

                // Dibujar cuadrícula de referencia (líneas horizontales de nivel)
                let center_y = rect.center().y;
                let p = ui.painter();
                let grid_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(128, 128, 128, 45));
                let zero_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(160, 160, 160, 80));

                let y_top_ref = center_y - (1.0 * state.scale_y).clamp(0.0, 1.0) * (display_h * 0.44);
                let y_bot_ref = center_y + (1.0 * state.scale_y).clamp(0.0, 1.0) * (display_h * 0.44);
                p.line_segment([Pos2::new(rect.left(), y_top_ref), Pos2::new(rect.right(), y_top_ref)], grid_stroke);
                p.line_segment([Pos2::new(rect.left(), center_y), Pos2::new(rect.right(), center_y)], zero_stroke);
                p.line_segment([Pos2::new(rect.left(), y_bot_ref), Pos2::new(rect.right(), y_bot_ref)], grid_stroke);

                // Etiquetas de cuadrícula
                p.text(Pos2::new(rect.left() + 4.0, y_top_ref + 2.0), egui::Align2::LEFT_TOP, "+1.0", egui::FontId::proportional(9.5), theme.text_secondary());
                p.text(Pos2::new(rect.left() + 4.0, center_y + 2.0), egui::Align2::LEFT_TOP, " 0.0", egui::FontId::proportional(9.5), theme.text_secondary());
                p.text(Pos2::new(rect.left() + 4.0, y_bot_ref - 12.0), egui::Align2::LEFT_TOP, "-1.0", egui::FontId::proportional(9.5), theme.text_secondary());

                let i_color = theme.accent_blue();
                let q_color = Color32::from_rgb(245, 140, 30); // Naranja suave para Q

                let width_px = (rect.width() as usize).max(10);
                let visible_len = visible_samples.len();
                let samples_per_pixel = visible_len as f32 / width_px as f32;

                if samples_per_pixel <= 2.5 {
                    // Modo de Alta Resolución / Zoom Cercano: Trazado punto a punto con líneas continuas
                    let mut prev_i = None;
                    let mut prev_q = None;

                    for (k, &(i_val, q_val)) in visible_samples.iter().enumerate() {
                        let x_pos = rect.left() + (k as f32 / (visible_len - 1).max(1) as f32) * rect.width();
                        let y_i = center_y - (i_val * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);
                        let y_q = center_y - (q_val * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);

                        let curr_i = Pos2::new(x_pos, y_i);
                        let curr_q = Pos2::new(x_pos, y_q);

                        if let Some(prev) = prev_i {
                            p.line_segment([prev, curr_i], Stroke::new(1.3_f32, i_color));
                        }
                        if let Some(prev) = prev_q {
                            p.line_segment([prev, curr_q], Stroke::new(1.1_f32, q_color));
                        }

                        if visible_len < 120 {
                            p.circle_filled(curr_i, 1.8, i_color);
                            p.circle_filled(curr_q, 1.5, q_color);
                        }

                        prev_i = Some(curr_i);
                        prev_q = Some(curr_q);
                    }
                } else {
                    // Modo Envolvente Min/Max por columna de píxeles para evitar aliasing
                    for x_idx in 0..width_px {
                        let s_start = ((x_idx as f32) * samples_per_pixel) as usize;
                        let s_end = (((x_idx + 1) as f32) * samples_per_pixel) as usize;
                        let s_end_clamped = s_end.max(s_start + 1).min(visible_len);
                        if s_start >= visible_len {
                            break;
                        }

                        let mut min_i = 0.0_f32;
                        let mut max_i = 0.0_f32;
                        let mut min_q = 0.0_f32;
                        let mut max_q = 0.0_f32;

                        for &(iv, qv) in &visible_samples[s_start..s_end_clamped] {
                            if iv < min_i { min_i = iv; }
                            if iv > max_i { max_i = iv; }
                            if qv < min_q { min_q = qv; }
                            if qv > max_q { max_q = qv; }
                        }

                        let x_pos = rect.left() + (x_idx as f32 / width_px as f32) * rect.width();
                        let y_top_i = center_y - (max_i * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);
                        let y_bot_i = center_y - (min_i * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);
                        let y_top_q = center_y - (max_q * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);
                        let y_bot_q = center_y - (min_q * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);

                        p.line_segment([Pos2::new(x_pos, y_top_i), Pos2::new(x_pos, y_bot_i.max(y_top_i + 1.0))], Stroke::new(1.2_f32, i_color));
                        p.line_segment([Pos2::new(x_pos, y_top_q), Pos2::new(x_pos, y_bot_q.max(y_top_q + 1.0))], Stroke::new(1.0_f32, q_color));
                    }
                }

                // Leyenda inferior y ventana de tiempo actual
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Canal I (En fase)").strong().size(11.0).color(i_color));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Canal Q (Cuadratura)").strong().size(11.0).color(q_color));

                    if sample_rate > 0 {
                        let t_start = start_idx as f32 / sample_rate as f32;
                        let t_end = end_idx as f32 / sample_rate as f32;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Ventana: [{:.4}s - {:.4}s] | {:.4}s visible ({} muestras)",
                                    t_start,
                                    t_end,
                                    t_end - t_start,
                                    visible_len
                                ))
                                .size(10.5)
                                .color(theme.text_secondary()),
                            );
                        });
                    }
                });
            });
        });

    ui.data_mut(|d| *d.get_temp_mut_or_default::<ScopeControls>(scope_id) = state);
}

/// Visualizador gráfico de forma de onda de audio con Zoom X, Pan X y Escala Y interactiva y botón de exportar imagen
pub fn render_audio_waveform(
    ui: &mut Ui,
    theme: &AppTheme,
    samples: &[f32],
    sample_rate: u32,
    title: &str,
) {
    let scope_id = ui.make_persistent_id(format!("audio_scope_{}", title));
    let mut state = ui.data_mut(|d| d.get_temp_mut_or_default::<ScopeControls>(scope_id).clone());

    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(title)
                            .strong()
                            .size(13.5)
                            .color(theme.text_primary()),
                    );

                    let total_dur = if sample_rate > 0 {
                        samples.len() as f32 / sample_rate as f32
                    } else {
                        0.0
                    };

                    ui.label(
                        RichText::new(format!(
                            "Total: {} muestras | {:.2}s a {} Hz",
                            samples.len(),
                            total_dur,
                            sample_rate
                        ))
                        .size(11.5)
                        .color(theme.text_secondary()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_label = format!("{} Guardar Oscilograma (.png)", crate::icons::ICON_DOWNLOAD.codepoint);
                        let save_btn = egui::Button::new(
                            RichText::new(btn_label)
                                .strong()
                                .size(11.5)
                                .color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(6.0);

                        if ui.add(save_btn).clicked() {
                            let default_fn = format!("{}.png", title.to_lowercase().replace(' ', "_"));
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(&default_fn)
                                .add_filter("Imagen PNG", &["png"])
                                .save_file()
                            {
                                if let Some(parent) = path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                let chart = render_high_res_scope_chart(
                                    samples,
                                    None,
                                    sample_rate,
                                    title,
                                    state.zoom_x,
                                    state.pan_x,
                                    state.scale_y,
                                );
                                match chart.save(&path) {
                                    Ok(_) => {
                                        state.status_message = Some((
                                            format!("Oscilograma guardado con éxito: {}", path.display()),
                                            false,
                                        ));
                                    }
                                    Err(e) => {
                                        state.status_message = Some((
                                            format!("Error al guardar oscilograma: {}", e),
                                            true,
                                        ));
                                    }
                                }
                            }
                        }
                    });
                });

                if let Some((ref msg, is_err)) = state.status_message {
                    ui.add_space(4.0);
                    let col = if is_err {
                        Color32::from_rgb(239, 68, 68)
                    } else {
                        Color32::from_rgb(34, 197, 94)
                    };
                    ui.label(RichText::new(msg).size(11.5).color(col));
                }

                ui.add_space(6.0);

                // Barra de herramientas de Zoom y Escala interactiva
                render_scope_toolbar(ui, theme, &mut state, samples.len(), sample_rate);

                ui.add_space(6.0);

                let display_h = 115.0;
                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), display_h),
                    egui::Sense::click_and_drag(),
                );

                handle_scope_mouse_interaction(&resp, ui, &mut state);

                let bg_fill = if *theme == AppTheme::Dark {
                    Color32::from_rgb(12, 16, 24)
                } else {
                    Color32::from_rgb(238, 244, 252)
                };
                ui.painter().rect_filled(rect, 4.0, bg_fill);
                ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);

                if samples.is_empty() {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Cargue o genere una señal de audio para visualizar su forma de onda",
                        egui::FontId::proportional(11.5),
                        theme.text_secondary(),
                    );
                    return;
                }

                let (start_idx, end_idx) = get_visible_window(samples.len(), state.zoom_x, state.pan_x);
                let visible_samples = &samples[start_idx..end_idx];

                let center_y = rect.center().y;
                let p = ui.painter();
                let grid_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(128, 128, 128, 45));
                let zero_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(160, 160, 160, 80));

                let y_top_ref = center_y - (1.0 * state.scale_y).clamp(0.0, 1.0) * (display_h * 0.44);
                let y_bot_ref = center_y + (1.0 * state.scale_y).clamp(0.0, 1.0) * (display_h * 0.44);
                p.line_segment([Pos2::new(rect.left(), y_top_ref), Pos2::new(rect.right(), y_top_ref)], grid_stroke);
                p.line_segment([Pos2::new(rect.left(), center_y), Pos2::new(rect.right(), center_y)], zero_stroke);
                p.line_segment([Pos2::new(rect.left(), y_bot_ref), Pos2::new(rect.right(), y_bot_ref)], grid_stroke);

                let wave_color = theme.accent_blue();
                let width_px = (rect.width() as usize).max(10);
                let visible_len = visible_samples.len();
                let samples_per_pixel = visible_len as f32 / width_px as f32;

                if samples_per_pixel <= 2.5 {
                    // Zoom cercano: Dibujar trazo continuo senoidal
                    let mut prev_pt = None;
                    for (k, &s) in visible_samples.iter().enumerate() {
                        let x_pos = rect.left() + (k as f32 / (visible_len - 1).max(1) as f32) * rect.width();
                        let y_pos = center_y - (s * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);
                        let curr = Pos2::new(x_pos, y_pos);

                        if let Some(prev) = prev_pt {
                            p.line_segment([prev, curr], Stroke::new(1.4_f32, wave_color));
                        }
                        if visible_len < 100 {
                            p.circle_filled(curr, 1.8, wave_color);
                        }
                        prev_pt = Some(curr);
                    }
                } else {
                    // Zoom lejano: Envolvente de picos min/max
                    for x_idx in 0..width_px {
                        let s_start = ((x_idx as f32) * samples_per_pixel) as usize;
                        let s_end = (((x_idx + 1) as f32) * samples_per_pixel) as usize;
                        let s_end_clamped = s_end.max(s_start + 1).min(visible_len);
                        if s_start >= visible_len {
                            break;
                        }

                        let mut min_val = 0.0_f32;
                        let mut max_val = 0.0_f32;
                        for &s in &visible_samples[s_start..s_end_clamped] {
                            if s < min_val { min_val = s; }
                            if s > max_val { max_val = s; }
                        }

                        let x_pos = rect.left() + (x_idx as f32 / width_px as f32) * rect.width();
                        let y_top = center_y - (max_val * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);
                        let y_bot = center_y - (min_val * state.scale_y).clamp(-1.0, 1.0) * (display_h * 0.44);

                        p.line_segment(
                            [Pos2::new(x_pos, y_top), Pos2::new(x_pos, y_bot.max(y_top + 1.0))],
                            Stroke::new(1.2_f32, wave_color),
                        );
                    }
                }

                // Información de rango visible al pie del osciloscopio
                if sample_rate > 0 {
                    ui.add_space(4.0);
                    let t_start = start_idx as f32 / sample_rate as f32;
                    let t_end = end_idx as f32 / sample_rate as f32;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Ventana visible: [{:.4}s - {:.4}s] | {:.4}s mostrados ({} muestras)",
                                t_start,
                                t_end,
                                t_end - t_start,
                                visible_len
                            ))
                            .size(10.5)
                            .color(theme.text_secondary()),
                        );
                    });
                }
            });
        });

    ui.data_mut(|d| *d.get_temp_mut_or_default::<ScopeControls>(scope_id) = state);
}

/// Renderiza la barra de control interactiva para los osciloscopios
fn render_scope_toolbar(
    ui: &mut Ui,
    theme: &AppTheme,
    state: &mut ScopeControls,
    _total_len: usize,
    _sample_rate: u32,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        ui.label(RichText::new("Zoom Tiempo (X):").size(11.5).color(theme.text_secondary()));
        ui.add(
            egui::Slider::new(&mut state.zoom_x, 1.0..=50_000.0)
                .logarithmic(true)
                .suffix("x")
                .custom_formatter(|n, _| format!("{:.1}x", n)),
        );

        if state.zoom_x > 1.01 {
            ui.label(RichText::new("Posición X:").size(11.5).color(theme.text_secondary()));
            ui.add(
                egui::Slider::new(&mut state.pan_x, 0.0..=1.0)
                    .show_value(false),
            );
        }

        ui.label(RichText::new("Escala Amplitud (Y):").size(11.5).color(theme.text_secondary()));
        ui.add(
            egui::Slider::new(&mut state.scale_y, 0.1..=100.0)
                .logarithmic(true)
                .suffix("x")
                .custom_formatter(|n, _| format!("{:.1}x", n)),
        );

        let is_custom = state.zoom_x != 1.0 || state.scale_y != 1.0 || state.pan_x != 0.0;
        if is_custom {
            if ui.button(RichText::new("Restablecer").size(11.0)).clicked() {
                state.zoom_x = 1.0;
                state.pan_x = 0.0;
                state.scale_y = 1.0;
            }
        }

        ui.label(
            RichText::new("(Arrastre el mouse o use la rueda para zoom/scroll)")
                .size(10.5)
                .color(theme.text_secondary()),
        );
    });
}

/// Maneja la interacción con mouse (rueda para zoom y arrastre para pan)
fn handle_scope_mouse_interaction(resp: &egui::Response, ui: &mut Ui, state: &mut ScopeControls) {
    if resp.dragged() {
        let delta = resp.drag_delta();
        let span = (resp.rect.width() * state.zoom_x).max(1.0);
        state.pan_x = (state.pan_x - delta.x / span).clamp(0.0, 1.0);
    }

    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll > 0.0 {
            state.zoom_x = (state.zoom_x * 1.25).min(50_000.0);
        } else if scroll < 0.0 {
            state.zoom_x = (state.zoom_x / 1.25).max(1.0);
            if state.zoom_x <= 1.01 {
                state.pan_x = 0.0;
            }
        }
    }

    if resp.double_clicked() {
        state.zoom_x = 1.0;
        state.pan_x = 0.0;
        state.scale_y = 1.0;
    }
}

/// Helper para visualizar buffers PCM i16 en el dominio del tiempo
pub fn render_audio_waveform_i16(
    ui: &mut Ui,
    theme: &AppTheme,
    samples: &[i16],
    sample_rate: u32,
    title: &str,
) {
    let float_samples: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();
    render_audio_waveform(ui, theme, &float_samples, sample_rate, title);
}
