use crate::theme::AppTheme;
use egui::{
    Color32, ColorImage, Frame, Pos2, Rect, RichText, Stroke, TextureHandle, TextureOptions, Ui,
    Vec2,
};
use num_complex::Complex32;
use std::f32::consts::PI;
use std::path::PathBuf;

pub struct SpectrogramState {
    pub show: bool,
    pub texture: Option<TextureHandle>,
    pub cached_samples_len: usize,
    pub cached_sample_rate: u32,
    pub cached_zoom_x: f32,
    pub cached_pan_x: f32,
    pub cached_zoom_y: f32,
    pub cached_pan_y: f32,
    pub zoom_x: f32,  // 1.0 a 50_000.0x
    pub pan_x: f32,   // 0.0 a 1.0
    pub zoom_y: f32,  // 1.0 a 50.0x
    pub pan_y: f32,   // -1.0 a 1.0 (0.0 = 0 Hz en el centro)
    pub raw_image: Option<image::RgbaImage>,
    pub status_message: Option<(String, bool)>, // (mensaje, es_error)
}

impl Default for SpectrogramState {
    fn default() -> Self {
        Self {
            show: false,
            texture: None,
            cached_samples_len: 0,
            cached_sample_rate: 0,
            cached_zoom_x: 1.0,
            cached_pan_x: 0.0,
            cached_zoom_y: 1.0,
            cached_pan_y: 0.0,
            zoom_x: 1.0,
            pan_x: 0.0,
            zoom_y: 1.0,
            pan_y: 0.0,
            raw_image: None,
            status_message: None,
        }
    }
}

const FFT_SIZE: usize = 256;
const TIME_COLUMNS: usize = 512;

/// Helper para calcular la ventana de muestras en el tiempo según zoom X y pan X
pub fn get_time_window(total_len: usize, zoom_x: f32, pan_x: f32) -> (usize, usize) {
    if total_len == 0 {
        return (0, 0);
    }
    let z = zoom_x.max(1.0);
    let visible_count = ((total_len as f32) / z).max(32.0).min(total_len as f32) as usize;
    let max_start = if total_len > visible_count { total_len - visible_count } else { 0 };
    let start_idx = ((pan_x.clamp(0.0, 1.0) * max_start as f32).round() as usize).min(max_start);
    let end_idx = (start_idx + visible_count).min(total_len);
    (start_idx, end_idx)
}

/// Helper para calcular las filas de FFT visibles según zoom Y y pan Y
pub fn get_freq_window(fft_h: usize, zoom_y: f32, pan_y: f32) -> (f32, f32) {
    let z_y = zoom_y.max(1.0);
    let visible_rows = (fft_h as f32 / z_y).clamp(4.0, fft_h as f32);
    let max_pan = (fft_h as f32 - visible_rows) / 2.0;
    // pan_y en [-1.0, 1.0]: 0.0 es centro (0 Hz). +1.0 va hacia frecuencias positivas (fila 0), -1.0 hacia negativas (fila H-1)
    let center_row = (fft_h as f32 / 2.0) - pan_y.clamp(-1.0, 1.0) * max_pan;
    let row_start = (center_row - visible_rows / 2.0).clamp(0.0, fft_h as f32 - visible_rows);
    let row_end = (row_start + visible_rows).min(fft_h as f32);
    (row_start, row_end)
}

/// Calcula el espectrograma (cascada tiempo vs frecuencia) con zoom y pan para la visualización en la interfaz
pub fn generate_spectrogram_image(
    samples: &[(f32, f32)],
    _sample_rate: u32,
    zoom_x: f32,
    pan_x: f32,
    zoom_y: f32,
    pan_y: f32,
) -> (ColorImage, image::RgbaImage) {
    let w = TIME_COLUMNS;
    let h = FFT_SIZE;

    if samples.is_empty() {
        let pixels = vec![0u8; w * h * 4];
        let color_img = ColorImage::from_rgba_unmultiplied([w, h], &pixels);
        let raw_img = image::RgbaImage::from_raw(w as u32, h as u32, pixels).unwrap();
        return (color_img, raw_img);
    }

    let (start_idx, end_idx) = get_time_window(samples.len(), zoom_x, pan_x);
    let visible_samples = &samples[start_idx..end_idx];
    let visible_len = visible_samples.len();

    // Ventana Hann precalculada
    let mut hann = vec![0.0f32; h];
    for i in 0..h {
        hann[i] = 0.5 * (1.0 - (2.0 * PI * i as f32 / h as f32).cos());
    }

    let mut db_matrix = vec![vec![0.0f32; h]; w];
    let max_offset = if visible_len > h { visible_len - h } else { 0 };
    let mut max_db = -120.0f32;

    for x in 0..w {
        let offset = if visible_len > h {
            (x * max_offset) / (w - 1)
        } else {
            0
        };

        let mut fft_buf = vec![Complex32::new(0.0, 0.0); h];
        for k in 0..h {
            if offset + k < visible_len {
                let (i_val, q_val) = visible_samples[offset + k];
                let win = hann[k];
                fft_buf[k] = Complex32::new(i_val * win, q_val * win);
            }
        }

        fft_radix2(&mut fft_buf);

        // FFT Shift & mapeo a filas: Top (y=0) es +Fs/2, Centro (y=h/2) es 0 Hz, Abajo (y=h-1) es -Fs/2
        for y in 0..h {
            let shifted_idx = h - 1 - y;
            let raw_k = (shifted_idx + h / 2) % h;
            let mag_sq = fft_buf[raw_k].norm_sqr();
            let db = 10.0 * (mag_sq + 1e-12).log10();
            db_matrix[x][y] = db;
            if db > max_db {
                max_db = db;
            }
        }
    }

    let min_db = max_db - 55.0; // Rango dinámico RF de 55 dB
    let range = (max_db - min_db).max(1.0);

    let (row_start, row_end) = get_freq_window(h, zoom_y, pan_y);

    let mut pixels = vec![0u8; w * h * 4];
    for y in 0..h {
        let row_f = row_start + (y as f32 / (h - 1).max(1) as f32) * (row_end - row_start);
        let fft_y = (row_f.round() as usize).min(h - 1);

        for x in 0..w {
            let val_db = db_matrix[x][fft_y];
            let norm = ((val_db - min_db) / range).clamp(0.0, 1.0);
            let color = rf_waterfall_colormap(norm);

            let idx = (y * w + x) * 4;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = color[3];
        }
    }

    let raw_img = image::RgbaImage::from_raw(w as u32, h as u32, pixels.clone())
        .unwrap_or_else(|| image::RgbaImage::new(w as u32, h as u32));
    let color_img = ColorImage::from_rgba_unmultiplied([w, h], &pixels);

    (color_img, raw_img)
}

/// Genera una imagen completa en alta definición (1600x900 px) reflejando exactamente el zoom y posición aplicados
pub fn render_high_res_spectrogram_chart(
    samples: &[(f32, f32)],
    sample_rate: u32,
    pipeline_name: &str,
    zoom_x: f32,
    pan_x: f32,
    zoom_y: f32,
    pan_y: f32,
) -> image::RgbaImage {
    let img_w = 1600u32;
    let img_h = 900u32;

    // Fondo oscuro profesional estilo instrumentación de laboratorio SDR
    let bg_color = image::Rgba([13, 17, 23, 255]);
    let mut chart = image::RgbaImage::from_pixel(img_w, img_h, bg_color);

    // Dimensiones de la cascada dentro del lienzo
    let wf_x = 150u32;
    let wf_y = 90u32;
    let wf_w = 1300u32;
    let wf_h = 720u32;

    let bar_x = 1485u32;
    let bar_w = 25u32;

    let (start_idx, end_idx) = get_time_window(samples.len(), zoom_x, pan_x);
    let visible_samples = if !samples.is_empty() {
        &samples[start_idx..end_idx]
    } else {
        &[]
    };
    let visible_len = visible_samples.len();

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
    let dur_sec = t_end - t_start;

    let fft_h = 512usize;
    let time_cols = wf_w as usize;

    let (row_start, row_end) = get_freq_window(fft_h, zoom_y, pan_y);

    let half_fs = sample_rate as f32 / 2.0;
    let f_top = half_fs * (1.0 - 2.0 * row_start / fft_h as f32);
    let f_bot = half_fs * (1.0 - 2.0 * row_end / fft_h as f32);
    let f_span = (f_top - f_bot).max(1.0);

    // 1. Calcular FFT de alta definición para la cascada visible
    if !visible_samples.is_empty() {
        let mut hann = vec![0.0f32; fft_h];
        for i in 0..fft_h {
            hann[i] = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_h as f32).cos());
        }

        let mut db_matrix = vec![vec![0.0f32; fft_h]; time_cols];
        let max_offset = if visible_len > fft_h { visible_len - fft_h } else { 0 };
        let mut max_db = -120.0f32;

        for x in 0..time_cols {
            let offset = if visible_len > fft_h {
                (x * max_offset) / (time_cols - 1)
            } else {
                0
            };

            let mut fft_buf = vec![Complex32::new(0.0, 0.0); fft_h];
            for k in 0..fft_h {
                if offset + k < visible_len {
                    let (i_val, q_val) = visible_samples[offset + k];
                    let win = hann[k];
                    fft_buf[k] = Complex32::new(i_val * win, q_val * win);
                }
            }

            fft_radix2(&mut fft_buf);

            for y in 0..fft_h {
                let shifted_idx = fft_h - 1 - y;
                let raw_k = (shifted_idx + fft_h / 2) % fft_h;
                let mag_sq = fft_buf[raw_k].norm_sqr();
                let db = 10.0 * (mag_sq + 1e-12).log10();
                db_matrix[x][y] = db;
                if db > max_db {
                    max_db = db;
                }
            }
        }

        let min_db = max_db - 55.0;
        let range = (max_db - min_db).max(1.0);

        // Dibujar pixeles de la cascada
        for y in 0..wf_h {
            let row_f = row_start + (y as f32 / (wf_h - 1).max(1) as f32) * (row_end - row_start);
            let fft_row = (row_f.round() as usize).min(fft_h - 1);

            for x in 0..wf_w {
                let db_val = db_matrix[x as usize][fft_row];
                let norm = ((db_val - min_db) / range).clamp(0.0, 1.0);
                let col = rf_waterfall_colormap(norm);
                chart.put_pixel(wf_x + x, wf_y + y, image::Rgba(col));
            }
        }
    } else {
        draw_rect_filled(&mut chart, wf_x, wf_y, wf_w, wf_h, [18, 24, 34, 255]);
    }

    // 2. Líneas de referencia y cuadrícula
    // Si 0 Hz (portadora DC) está dentro del rango visible, dibujarla
    if f_bot <= 0.0 && f_top >= 0.0 {
        let t_dc = (f_top - 0.0) / f_span;
        let dc_y = wf_y + (t_dc * wf_h as f32).round() as u32;
        if dc_y >= wf_y && dc_y < wf_y + wf_h {
            draw_line_horizontal(&mut chart, wf_x, wf_x + wf_w, dc_y, [255, 255, 255, 130]);
        }
    }

    // Líneas guías horizontales en 25% y 75%
    draw_dashed_line_horizontal(&mut chart, wf_x, wf_x + wf_w, wf_y + wf_h / 4, [255, 255, 255, 40]);
    draw_dashed_line_horizontal(&mut chart, wf_x, wf_x + wf_w, wf_y + (3 * wf_h) / 4, [255, 255, 255, 40]);

    // Líneas guías verticales en 25%, 50%, 75% de tiempo
    draw_dashed_line_vertical(&mut chart, wf_x + wf_w / 4, wf_y, wf_y + wf_h, [255, 255, 255, 30]);
    draw_dashed_line_vertical(&mut chart, wf_x + wf_w / 2, wf_y, wf_y + wf_h, [255, 255, 255, 40]);
    draw_dashed_line_vertical(&mut chart, wf_x + (3 * wf_w) / 4, wf_y, wf_y + wf_h, [255, 255, 255, 30]);

    // Borde exterior de la cascada
    draw_rect_outline(&mut chart, wf_x, wf_y, wf_w, wf_h, [60, 75, 95, 255]);

    // 3. Barra de Color (Potencia / Heatmap dB)
    for y in 0..wf_h {
        let t = 1.0 - (y as f32 / wf_h as f32);
        let col = rf_waterfall_colormap(t);
        for x in 0..bar_w {
            chart.put_pixel(bar_x + x, wf_y + y, image::Rgba(col));
        }
    }
    draw_rect_outline(&mut chart, bar_x, wf_y, bar_w, wf_h, [60, 75, 95, 255]);

    // 4. Encabezado y Metadatos
    let title_text = format!("SIGNALS SDR - ESPECTROGRAMA RF [{}]", pipeline_name.to_uppercase());
    draw_text_on_image(&mut chart, wf_x, 24, &title_text, [240, 246, 252, 255], 2);

    let format_freq_meta = |f: f32| -> String {
        if f.abs() >= 1000.0 {
            format!("{:.1} kHz", f / 1000.0)
        } else {
            format!("{:.0} Hz", f)
        }
    };

    let meta_text = format!(
        "Fs: {} Hz | Ventana: [{:.4}s - {:.4}s] ({:.4}s) | Banda: [{} a {}] | Zoom: X={:.1}x, Y={:.1}x | Muestras: {}",
        sample_rate,
        t_start,
        t_end,
        dur_sec,
        format_freq_meta(f_bot),
        format_freq_meta(f_top),
        zoom_x,
        zoom_y,
        visible_len
    );
    draw_text_on_image(&mut chart, wf_x, 54, &meta_text, [139, 148, 158, 255], 1);

    // 5. EJE Y: Frecuencia (kHz / Hz)
    let format_freq_tick = |f: f32| -> String {
        if f.abs() >= 1000.0 {
            format!("{:+7.1} kHz", f / 1000.0)
        } else {
            format!("{:+7.0} Hz ", f)
        }
    };

    let y_ticks = [
        (wf_y, format_freq_tick(f_top)),
        (wf_y + wf_h / 4, format_freq_tick(f_top - 0.25 * f_span)),
        (wf_y + wf_h / 2, format_freq_tick(f_top - 0.50 * f_span)),
        (wf_y + (3 * wf_h) / 4, format_freq_tick(f_top - 0.75 * f_span)),
        (wf_y + wf_h, format_freq_tick(f_bot)),
    ];

    for (y_pos, label) in y_ticks {
        let y_clamped = y_pos.min(wf_y + wf_h - 1);
        draw_line_horizontal(&mut chart, wf_x - 8, wf_x, y_clamped, [160, 175, 195, 255]);
        let lbl_y = if y_pos >= 4 { y_pos - 4 } else { 0 };
        draw_text_on_image(&mut chart, 20, lbl_y, &label, [201, 209, 217, 255], 1);
    }
    draw_text_on_image(&mut chart, 20, wf_y - 20, "FRECUENCIA", [88, 166, 255, 255], 1);

    // 6. EJE X: Tiempo (segundos)
    let x_ticks = [
        (wf_x, format!("{:.3}s", t_start)),
        (wf_x + wf_w / 4, format!("{:.3}s", t_start + dur_sec * 0.25)),
        (wf_x + wf_w / 2, format!("{:.3}s", t_start + dur_sec * 0.50)),
        (wf_x + (3 * wf_w) / 4, format!("{:.3}s", t_start + dur_sec * 0.75)),
        (wf_x + wf_w, format!("{:.3}s", t_end)),
    ];

    for (x_pos, label) in x_ticks {
        let x_clamped = x_pos.min(wf_x + wf_w - 1);
        draw_line_vertical(&mut chart, x_clamped, wf_y + wf_h, wf_y + wf_h + 8, [160, 175, 195, 255]);
        let lbl_x = if x_pos >= 24 { x_pos - 22 } else { 0 };
        draw_text_on_image(&mut chart, lbl_x, wf_y + wf_h + 14, &label, [201, 209, 217, 255], 1);
    }
    draw_text_on_image(&mut chart, wf_x + wf_w - 140, wf_y + wf_h + 34, "TIEMPO (s) ->", [88, 166, 255, 255], 1);

    // 7. Etiquetas de la Barra de Color (Potencia dB)
    draw_text_on_image(&mut chart, bar_x - 10, wf_y - 20, "POTENCIA", [88, 166, 255, 255], 1);

    let db_ticks = [
        (wf_y, " 0 dB (Pico)".to_string()),
        (wf_y + wf_h / 4, "-14 dB".to_string()),
        (wf_y + wf_h / 2, "-28 dB".to_string()),
        (wf_y + (3 * wf_h) / 4, "-42 dB".to_string()),
        (wf_y + wf_h - 1, "-55 dB (Piso)".to_string()),
    ];

    for (y_pos, label) in db_ticks {
        let y_clamped = y_pos.min(wf_y + wf_h - 1);
        draw_line_horizontal(&mut chart, bar_x + bar_w, bar_x + bar_w + 6, y_clamped, [160, 175, 195, 255]);
        let lbl_y = if y_pos >= 4 { y_pos - 4 } else { 0 };
        draw_text_on_image(&mut chart, bar_x + bar_w + 10, lbl_y, &label, [201, 209, 217, 255], 1);
    }

    chart
}

/// Renderiza el widget de espectrograma completo con controles de zoom/pan interactivos y ejes de tiempo y frecuencia
pub fn render_spectrogram_widget(
    ui: &mut Ui,
    theme: &AppTheme,
    state: &mut SpectrogramState,
    samples: &[(f32, f32)],
    sample_rate: u32,
    pipeline_name: &str,
    target_output_dir: Option<&PathBuf>,
) {
    if samples.is_empty() {
        ui.label(
            RichText::new("Sin muestras I/Q para generar espectrograma.")
                .size(12.0)
                .color(theme.text_secondary()),
        );
        return;
    }

    // Actualizar textura si las muestras cambiaron o los controles de zoom/pan se modificaron
    let needs_update = state.texture.is_none()
        || state.cached_samples_len != samples.len()
        || state.cached_sample_rate != sample_rate
        || (state.cached_zoom_x - state.zoom_x).abs() > 0.001
        || (state.cached_pan_x - state.pan_x).abs() > 0.001
        || (state.cached_zoom_y - state.zoom_y).abs() > 0.001
        || (state.cached_pan_y - state.pan_y).abs() > 0.001;

    if needs_update {
        let (color_img, raw_img) = generate_spectrogram_image(
            samples,
            sample_rate,
            state.zoom_x,
            state.pan_x,
            state.zoom_y,
            state.pan_y,
        );
        state.texture = Some(ui.ctx().load_texture(
            format!("spectrogram_{}", pipeline_name),
            color_img,
            TextureOptions::LINEAR,
        ));
        state.raw_image = Some(raw_img);
        state.cached_samples_len = samples.len();
        state.cached_sample_rate = sample_rate;
        state.cached_zoom_x = state.zoom_x;
        state.cached_pan_x = state.pan_x;
        state.cached_zoom_y = state.zoom_y;
        state.cached_pan_y = state.pan_y;
    }

    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Encabezado con título, métricas y botón de guardar imagen
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Espectrograma RF / Cascada (Tiempo vs Frecuencia)")
                            .strong()
                            .size(14.0)
                            .color(theme.text_primary()),
                    );

                    let dur_sec = samples.len() as f32 / sample_rate as f32;
                    ui.label(
                        RichText::new(format!(
                            "Duración: {:.2}s | Tasa: {} Hz | Eje X: Tiempo, Eje Y: Frecuencia",
                            dur_sec, sample_rate
                        ))
                        .size(12.0)
                        .color(theme.text_secondary()),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Botón Guardar Espectrograma con letras blancas para contraste impecable
                        let save_btn = egui::Button::new(
                            RichText::new("Guardar Espectrograma (.png)")
                                .strong()
                                .size(12.5)
                                .color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue())
                        .corner_radius(6.0);

                        if ui.add(save_btn).clicked() {
                            let default_filename = format!("espectrograma_{}.png", pipeline_name);
                            let save_dest = rfd::FileDialog::new()
                                .set_file_name(&default_filename)
                                .add_filter("Imagen PNG", &["png"])
                                .save_file()
                                .or_else(|| {
                                    target_output_dir.map(|dir| dir.join(&default_filename))
                                });

                            if let Some(path) = save_dest {
                                if let Some(parent) = path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                // Generar gráfico de alta resolución (1600x900) con el zoom y la posición actuales
                                let high_res_chart = render_high_res_spectrogram_chart(
                                    samples,
                                    sample_rate,
                                    pipeline_name,
                                    state.zoom_x,
                                    state.pan_x,
                                    state.zoom_y,
                                    state.pan_y,
                                );
                                match high_res_chart.save(&path) {
                                    Ok(_) => {
                                        state.status_message = Some((
                                            format!("Espectrograma en alta resolución guardado: {}", path.display()),
                                            false,
                                        ));
                                    }
                                    Err(e) => {
                                        state.status_message = Some((
                                            format!("Error al guardar imagen: {}", e),
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

                // Barra de herramientas de Zoom y Pan interactivos para el espectrograma
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
                        ui.add(egui::Slider::new(&mut state.pan_x, 0.0..=1.0).show_value(false));
                    }

                    ui.label(RichText::new("Zoom Freq (Y):").size(11.5).color(theme.text_secondary()));
                    ui.add(
                        egui::Slider::new(&mut state.zoom_y, 1.0..=50.0)
                            .logarithmic(true)
                            .suffix("x")
                            .custom_formatter(|n, _| format!("{:.1}x", n)),
                    );

                    if state.zoom_y > 1.01 {
                        ui.label(RichText::new("Centro Y:").size(11.5).color(theme.text_secondary()));
                        ui.add(egui::Slider::new(&mut state.pan_y, -1.0..=1.0).show_value(false));
                    }

                    let is_custom = state.zoom_x != 1.0 || state.pan_x != 0.0 || state.zoom_y != 1.0 || state.pan_y != 0.0;
                    if is_custom {
                        if ui.button(RichText::new("Restablecer").size(11.0)).clicked() {
                            state.zoom_x = 1.0;
                            state.pan_x = 0.0;
                            state.zoom_y = 1.0;
                            state.pan_y = 0.0;
                        }
                    }

                    ui.label(
                        RichText::new("(Arrastre sobre la cascada para mover; rueda o Shift+rueda para zoom)")
                            .size(10.5)
                            .color(theme.text_secondary()),
                    );
                });

                ui.add_space(8.0);

                // Área visual del Espectrograma con Ejes X/Y y Barra de Color
                let total_w = ui.available_width().max(320.0);
                let axis_y_w = 68.0;
                let colorbar_w = 42.0;
                let spectro_w = (total_w - axis_y_w - colorbar_w - 20.0).max(180.0);
                let spectro_h = 210.0;
                let axis_x_h = 24.0;

                let (full_rect, _) = ui.allocate_exact_size(
                    Vec2::new(total_w, spectro_h + axis_x_h),
                    egui::Sense::hover(),
                );

                // Rectángulo central del Espectrograma
                let img_rect = Rect::from_min_size(
                    Pos2::new(full_rect.left() + axis_y_w, full_rect.top()),
                    Vec2::new(spectro_w, spectro_h),
                );

                // Interacción con mouse sobre el lienzo del espectrograma
                let img_response = ui.interact(img_rect, ui.make_persistent_id("spectro_canvas_interact"), egui::Sense::click_and_drag());
                if img_response.dragged() {
                    let delta = img_response.drag_delta();
                    let span_x = (img_rect.width() * state.zoom_x).max(1.0);
                    state.pan_x = (state.pan_x - delta.x / span_x).clamp(0.0, 1.0);

                    let span_y = (img_rect.height() * state.zoom_y).max(1.0);
                    state.pan_y = (state.pan_y + delta.y / span_y).clamp(-1.0, 1.0);
                }
                if img_response.hovered() {
                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    let is_shift = ui.input(|i| i.modifiers.shift);
                    if scroll > 0.0 {
                        if is_shift {
                            state.zoom_y = (state.zoom_y * 1.2).min(50.0);
                        } else {
                            state.zoom_x = (state.zoom_x * 1.25).min(50_000.0);
                        }
                    } else if scroll < 0.0 {
                        if is_shift {
                            state.zoom_y = (state.zoom_y / 1.2).max(1.0);
                            if state.zoom_y <= 1.01 {
                                state.pan_y = 0.0;
                            }
                        } else {
                            state.zoom_x = (state.zoom_x / 1.25).max(1.0);
                            if state.zoom_x <= 1.01 {
                                state.pan_x = 0.0;
                            }
                        }
                    }
                }
                if img_response.double_clicked() {
                    state.zoom_x = 1.0;
                    state.pan_x = 0.0;
                    state.zoom_y = 1.0;
                    state.pan_y = 0.0;
                }

                let p = ui.painter();

                // Dibujar textura del espectrograma
                if let Some(ref tex) = state.texture {
                    p.image(
                        tex.id(),
                        img_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    p.rect_filled(img_rect, 0.0, Color32::from_rgb(15, 20, 28));
                }
                p.rect_stroke(img_rect, 0.0, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);

                // Calcular bandas visibles para etiquetas
                let (row_start, row_end) = get_freq_window(FFT_SIZE, state.zoom_y, state.pan_y);
                let half_fs = sample_rate as f32 / 2.0;
                let f_top = half_fs * (1.0 - 2.0 * row_start / FFT_SIZE as f32);
                let f_bot = half_fs * (1.0 - 2.0 * row_end / FFT_SIZE as f32);
                let f_span = (f_top - f_bot).max(1.0);

                // Línea central de 0 Hz (DC) si está visible
                if f_bot <= 0.0 && f_top >= 0.0 {
                    let t_dc = (f_top - 0.0) / f_span;
                    let dc_y = img_rect.top() + t_dc * spectro_h;
                    if dc_y >= img_rect.top() && dc_y <= img_rect.bottom() {
                        p.line_segment(
                            [Pos2::new(img_rect.left(), dc_y), Pos2::new(img_rect.right(), dc_y)],
                            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 110)),
                        );
                    }
                }

                // EJE Y: Frecuencia (kHz o Hz)
                let text_color = theme.text_secondary();
                let font_id = egui::FontId::proportional(10.5);

                let format_y_lbl = |f: f32| -> String {
                    if f.abs() >= 1000.0 {
                        format!("{:+5.1}k", f / 1000.0)
                    } else {
                        format!("{:+5.0}Hz", f)
                    }
                };

                let y_ticks = [
                    (img_rect.top(), format_y_lbl(f_top)),
                    (img_rect.top() + spectro_h * 0.25, format_y_lbl(f_top - 0.25 * f_span)),
                    (img_rect.top() + spectro_h * 0.50, format_y_lbl(f_top - 0.50 * f_span)),
                    (img_rect.top() + spectro_h * 0.75, format_y_lbl(f_top - 0.75 * f_span)),
                    (img_rect.bottom(), format_y_lbl(f_bot)),
                ];

                for (y_pos, lbl) in y_ticks {
                    p.line_segment(
                        [Pos2::new(img_rect.left() - 4.0, y_pos), Pos2::new(img_rect.left(), y_pos)],
                        Stroke::new(1.0_f32, theme.card_border()),
                    );
                    p.text(
                        Pos2::new(img_rect.left() - 6.0, y_pos),
                        egui::Align2::RIGHT_CENTER,
                        lbl,
                        font_id.clone(),
                        text_color,
                    );
                }

                // Título del eje Y
                p.text(
                    Pos2::new(full_rect.left() + 2.0, img_rect.top() - 2.0),
                    egui::Align2::LEFT_BOTTOM,
                    "Freq (Hz)",
                    egui::FontId::proportional(10.0),
                    theme.text_secondary(),
                );

                // EJE X: Tiempo (segundos)
                let (start_idx, end_idx) = get_time_window(samples.len(), state.zoom_x, state.pan_x);
                let t_start = if sample_rate > 0 { start_idx as f32 / sample_rate as f32 } else { 0.0 };
                let t_end = if sample_rate > 0 { end_idx as f32 / sample_rate as f32 } else { 0.0 };
                let t_dur = t_end - t_start;

                let x_ticks = [
                    (img_rect.left(), format!("{:.3}s", t_start)),
                    (img_rect.left() + spectro_w * 0.25, format!("{:.3}s", t_start + t_dur * 0.25)),
                    (img_rect.left() + spectro_w * 0.50, format!("{:.3}s", t_start + t_dur * 0.50)),
                    (img_rect.left() + spectro_w * 0.75, format!("{:.3}s", t_start + t_dur * 0.75)),
                    (img_rect.right(), format!("{:.3}s", t_end)),
                ];

                for (x_pos, lbl) in x_ticks {
                    p.line_segment(
                        [Pos2::new(x_pos, img_rect.bottom()), Pos2::new(x_pos, img_rect.bottom() + 4.0)],
                        Stroke::new(1.0_f32, theme.card_border()),
                    );
                    p.text(
                        Pos2::new(x_pos, img_rect.bottom() + 6.0),
                        egui::Align2::CENTER_TOP,
                        lbl,
                        font_id.clone(),
                        text_color,
                    );
                }

                // Título del eje X
                p.text(
                    Pos2::new(img_rect.right(), img_rect.bottom() + 6.0),
                    egui::Align2::RIGHT_TOP,
                    "Tiempo ->",
                    egui::FontId::proportional(10.0),
                    theme.text_secondary(),
                );

                // BARRA DE COLOR (Intensidad / Potencia dB)
                let bar_x = img_rect.right() + 10.0;
                let bar_w = 12.0;
                let bar_rect = Rect::from_min_size(
                    Pos2::new(bar_x, img_rect.top()),
                    Vec2::new(bar_w, spectro_h),
                );

                // Dibujar gradiente vertical de color
                let n_grad_steps = 30;
                for step in 0..n_grad_steps {
                    let f0 = step as f32 / n_grad_steps as f32;
                    let f1 = (step + 1) as f32 / n_grad_steps as f32;
                    let y0 = bar_rect.top() + f0 * spectro_h;
                    let y1 = bar_rect.top() + f1 * spectro_h;

                    let norm_val = 1.0 - (f0 + f1) * 0.5;
                    let color = rf_waterfall_colormap(norm_val);
                    let c32 = Color32::from_rgb(color[0], color[1], color[2]);

                    p.rect_filled(
                        Rect::from_min_max(Pos2::new(bar_x, y0), Pos2::new(bar_x + bar_w, y1)),
                        0.0,
                        c32,
                    );
                }
                p.rect_stroke(bar_rect, 0.0, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);

                // Etiquetas de la barra de color
                p.text(
                    Pos2::new(bar_x + bar_w + 4.0, bar_rect.top()),
                    egui::Align2::LEFT_CENTER,
                    "0 dB",
                    egui::FontId::proportional(10.0),
                    text_color,
                );
                p.text(
                    Pos2::new(bar_x + bar_w + 4.0, bar_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "-25 dB",
                    egui::FontId::proportional(10.0),
                    text_color,
                );
                p.text(
                    Pos2::new(bar_x + bar_w + 4.0, bar_rect.bottom()),
                    egui::Align2::LEFT_CENTER,
                    "-50 dB",
                    egui::FontId::proportional(10.0),
                    text_color,
                );
            });
        });
}

/// Algoritmo Cooley-Tukey Radix-2 FFT (in-place)
pub fn fft_radix2(buffer: &mut [Complex32]) {
    let n = buffer.len();
    if !n.is_power_of_two() {
        return;
    }

    let mut j = 0;
    for i in 0..n {
        if i < j {
            buffer.swap(i, j);
        }
        let mut bit = n >> 1;
        while (j & bit) != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
    }

    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle = -2.0 * PI / (len as f32);
        let w_step = Complex32::from_polar(1.0, angle);

        for i in (0..n).step_by(len) {
            let mut w = Complex32::new(1.0, 0.0);
            for k in 0..half {
                let u = buffer[i + k];
                let v = buffer[i + k + half] * w;
                buffer[i + k] = u + v;
                buffer[i + k + half] = u - v;
                w *= w_step;
            }
        }
        len <<= 1;
    }
}

/// Mapeo de intensidad normalizada [0.0, 1.0] a mapa de calor RF de alta definición estilo SDR#
pub fn rf_waterfall_colormap(t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let (r, g, b) = if t < 0.20 {
        let f = t / 0.20;
        (lerp(12.0, 0.0, f), lerp(16.0, 120.0, f), lerp(48.0, 210.0, f))
    } else if t < 0.45 {
        let f = (t - 0.20) / 0.25;
        (lerp(0.0, 20.0, f), lerp(120.0, 190.0, f), lerp(210.0, 80.0, f))
    } else if t < 0.70 {
        let f = (t - 0.45) / 0.25;
        (lerp(20.0, 245.0, f), lerp(190.0, 210.0, f), lerp(80.0, 20.0, f))
    } else if t < 0.90 {
        let f = (t - 0.70) / 0.20;
        (lerp(245.0, 240.0, f), lerp(210.0, 50.0, f), lerp(20.0, 20.0, f))
    } else {
        let f = (t - 0.90) / 0.10;
        (lerp(240.0, 255.0, f), lerp(50.0, 255.0, f), lerp(20.0, 240.0, f))
    };
    [r as u8, g as u8, b as u8, 255]
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// =========================================================================
// RASTERIZADOR Y DIBUJO DIRECTO EN IMAGEN (ALTA RESOLUCIÓN PARA EXPORTACIÓN)
// =========================================================================

pub fn draw_rect_filled(img: &mut image::RgbaImage, x: u32, y: u32, w: u32, h: u32, col: [u8; 4]) {
    let rgba = image::Rgba(col);
    for py in y..(y + h).min(img.height()) {
        for px in x..(x + w).min(img.width()) {
            img.put_pixel(px, py, rgba);
        }
    }
}

pub fn draw_rect_outline(img: &mut image::RgbaImage, x: u32, y: u32, w: u32, h: u32, col: [u8; 4]) {
    let rgba = image::Rgba(col);
    let right = (x + w - 1).min(img.width() - 1);
    let bottom = (y + h - 1).min(img.height() - 1);

    for px in x..=right {
        img.put_pixel(px, y, rgba);
        img.put_pixel(px, bottom, rgba);
    }
    for py in y..=bottom {
        img.put_pixel(x, py, rgba);
        img.put_pixel(right, py, rgba);
    }
}

pub fn draw_line_horizontal(img: &mut image::RgbaImage, x0: u32, x1: u32, y: u32, col: [u8; 4]) {
    if y >= img.height() {
        return;
    }
    let rgba = image::Rgba(col);
    for x in x0..=x1.min(img.width() - 1) {
        img.put_pixel(x, y, rgba);
    }
}

pub fn draw_dashed_line_horizontal(img: &mut image::RgbaImage, x0: u32, x1: u32, y: u32, col: [u8; 4]) {
    if y >= img.height() {
        return;
    }
    let rgba = image::Rgba(col);
    for x in x0..=x1.min(img.width() - 1) {
        if (x / 6) % 2 == 0 {
            img.put_pixel(x, y, rgba);
        }
    }
}

pub fn draw_line_vertical(img: &mut image::RgbaImage, x: u32, y0: u32, y1: u32, col: [u8; 4]) {
    if x >= img.width() {
        return;
    }
    let rgba = image::Rgba(col);
    for y in y0..=y1.min(img.height() - 1) {
        img.put_pixel(x, y, rgba);
    }
}

pub fn draw_dashed_line_vertical(img: &mut image::RgbaImage, x: u32, y0: u32, y1: u32, col: [u8; 4]) {
    if x >= img.width() {
        return;
    }
    let rgba = image::Rgba(col);
    for y in y0..=y1.min(img.height() - 1) {
        if (y / 6) % 2 == 0 {
            img.put_pixel(x, y, rgba);
        }
    }
}

/// Algoritmo de Bresenham para dibujar segmentos continuos entre dos puntos en la imagen
pub fn draw_line_segment(img: &mut image::RgbaImage, mut x0: i32, mut y0: i32, x1: i32, y1: i32, col: [u8; 4]) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let w = img.width() as i32;
    let h = img.height() as i32;
    let rgba = image::Rgba(col);

    loop {
        if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
            img.put_pixel(x0 as u32, y0 as u32, rgba);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Dibuja texto ASCII en la imagen usando una fuente matricial 8x8 escalable
pub fn draw_text_on_image(
    img: &mut image::RgbaImage,
    start_x: u32,
    start_y: u32,
    text: &str,
    color: [u8; 4],
    scale: u32,
) {
    let s = scale.max(1);
    let mut cur_x = start_x;

    for ch in text.chars() {
        let ascii = ch as usize;
        let glyph_idx = if (32..=126).contains(&ascii) {
            ascii - 32
        } else {
            0
        };

        let glyph = FONT_8X8[glyph_idx];
        for row in 0..8 {
            let row_byte = glyph[row];
            for col in 0..8 {
                if (row_byte & (1 << (7 - col))) != 0 {
                    for dy in 0..s {
                        for dx in 0..s {
                            let px = cur_x + col as u32 * s + dx;
                            let py = start_y + row as u32 * s + dy;
                            if px < img.width() && py < img.height() {
                                img.put_pixel(px, py, image::Rgba(color));
                            }
                        }
                    }
                }
            }
        }
        cur_x += 8 * s;
    }
}

/// Tabla de fuente matricial 8x8 para todos los caracteres imprimibles ASCII (32..126)
pub const FONT_8X8: [[u8; 8]; 96] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 32 ' '
    [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00], // 33 '!'
    [0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00], // 34 '"'
    [0x6C, 0x6C, 0xFE, 0x6C, 0xFE, 0x6C, 0x6C, 0x00], // 35 '#'
    [0x18, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x18, 0x00], // 36 '$'
    [0x00, 0xC6, 0xCC, 0x18, 0x30, 0x66, 0xC6, 0x00], // 37 '%'
    [0x38, 0x6C, 0x38, 0x76, 0xDC, 0xCC, 0x76, 0x00], // 38 '&'
    [0x18, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00], // 39 '''
    [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00], // 40 '('
    [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00], // 41 ')'
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00], // 42 '*'
    [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00], // 43 '+'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30], // 44 ','
    [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00], // 45 '-'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00], // 46 '.'
    [0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0x80, 0x00], // 47 '/'
    [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00], // 48 '0'
    [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00], // 49 '1'
    [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00], // 50 '2'
    [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00], // 51 '3'
    [0x0C, 0x1C, 0x34, 0x64, 0x7E, 0x04, 0x0E, 0x00], // 52 '4'
    [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00], // 53 '5'
    [0x1C, 0x30, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00], // 54 '6'
    [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x00], // 55 '7'
    [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00], // 56 '8'
    [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x0C, 0x38, 0x00], // 57 '9'
    [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00], // 58 ':'
    [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x30, 0x00], // 59 ';'
    [0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x00], // 60 '<'
    [0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00], // 61 '='
    [0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x00], // 62 '>'
    [0x3C, 0x66, 0x0C, 0x18, 0x18, 0x00, 0x18, 0x00], // 63 '?'
    [0x3C, 0x66, 0x6E, 0x6A, 0x6E, 0x60, 0x3C, 0x00], // 64 '@'
    [0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x00], // 65 'A'
    [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00], // 66 'B'
    [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00], // 67 'C'
    [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00], // 68 'D'
    [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00], // 69 'E'
    [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00], // 70 'F'
    [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3E, 0x00], // 71 'G'
    [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00], // 72 'H'
    [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00], // 73 'I'
    [0x0E, 0x06, 0x06, 0x06, 0x66, 0x66, 0x3C, 0x00], // 74 'J'
    [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00], // 75 'K'
    [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00], // 76 'L'
    [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00], // 77 'M'
    [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00], // 78 'N'
    [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00], // 79 'O'
    [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00], // 80 'P'
    [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00], // 81 'Q'
    [0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x00], // 82 'R'
    [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00], // 83 'S'
    [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00], // 84 'T'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00], // 85 'U'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00], // 86 'V'
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00], // 87 'W'
    [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00], // 88 'X'
    [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00], // 89 'Y'
    [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00], // 90 'Z'
    [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00], // 91 '['
    [0xC0, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x00], // 92 '\'
    [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00], // 93 ']'
    [0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00], // 94 '^'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00], // 95 '_'
    [0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00], // 96 '`'
    [0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3B, 0x00], // 97 'a'
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00], // 98 'b'
    [0x00, 0x00, 0x3C, 0x66, 0x60, 0x66, 0x3C, 0x00], // 99 'c'
    [0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00], // 100 'd'
    [0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0x00], // 101 'e'
    [0x1C, 0x30, 0x78, 0x30, 0x30, 0x30, 0x30, 0x00], // 102 'f'
    [0x00, 0x00, 0x3B, 0x66, 0x66, 0x3E, 0x06, 0x3C], // 103 'g'
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00], // 104 'h'
    [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00], // 105 'i'
    [0x06, 0x00, 0x0E, 0x06, 0x06, 0x66, 0x66, 0x3C], // 106 'j'
    [0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0x00], // 107 'k'
    [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00], // 108 'l'
    [0x00, 0x00, 0x66, 0x7F, 0x7F, 0x6B, 0x63, 0x00], // 109 'm'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00], // 110 'n'
    [0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00], // 111 'o'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60], // 112 'p'
    [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06], // 113 'q'
    [0x00, 0x00, 0x7C, 0x66, 0x60, 0x60, 0x60, 0x00], // 114 'r'
    [0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x00], // 115 's'
    [0x18, 0x18, 0x7E, 0x18, 0x18, 0x18, 0x0C, 0x00], // 116 't'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3B, 0x00], // 117 'u'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00], // 118 'v'
    [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x3E, 0x36, 0x00], // 119 'w'
    [0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x00], // 120 'x'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x3C], // 121 'y'
    [0x00, 0x00, 0x7E, 0x0C, 0x18, 0x30, 0x7E, 0x00], // 122 'z'
    [0x0E, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0E, 0x00], // 123 '{'
    [0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00], // 124 '|'
    [0x70, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x70, 0x00], // 125 '}'
    [0x76, 0xDC, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 126 '~'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // 127 DEL
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_spectrogram_image() {
        let sample_rate = 288_000;
        let count = 4096;
        let mut samples = Vec::with_capacity(count);

        for n in 0..count {
            let t = n as f32 / sample_rate as f32;
            let phase = 2.0 * std::f32::consts::PI * 20_000.0 * t;
            samples.push((phase.cos(), phase.sin()));
        }

        let (color_img, raw_img) = generate_spectrogram_image(&samples, sample_rate, 1.0, 0.0, 1.0, 0.0);

        assert_eq!(color_img.width(), TIME_COLUMNS);
        assert_eq!(color_img.height(), FFT_SIZE);
        assert_eq!(raw_img.width() as usize, TIME_COLUMNS);
        assert_eq!(raw_img.height() as usize, FFT_SIZE);
        assert!(!color_img.pixels.is_empty());
    }

    #[test]
    fn test_render_high_res_spectrogram_chart() {
        let sample_rate = 288_000;
        let count = 4096;
        let mut samples = Vec::with_capacity(count);

        for n in 0..count {
            let t = n as f32 / sample_rate as f32;
            let phase = 2.0 * std::f32::consts::PI * 15_000.0 * t;
            samples.push((phase.cos(), phase.sin()));
        }

        let chart = render_high_res_spectrogram_chart(&samples, sample_rate, "test_pipeline", 1.0, 0.0, 1.0, 0.0);

        assert_eq!(chart.width(), 1600);
        assert_eq!(chart.height(), 900);

        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        use image::ImageEncoder;
        encoder
            .write_image(
                chart.as_raw(),
                chart.width(),
                chart.height(),
                image::ExtendedColorType::Rgba8,
            )
            .expect("Guardado PNG de gráfico de alta resolución");

        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[1..4], b"PNG");
    }
}
