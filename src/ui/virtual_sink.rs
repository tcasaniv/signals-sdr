//! Instrumento virtual interactivo de laboratorio.
//!
//! Permite visualizar y analizar señales I/Q moduladas en tiempo real mediante:
//! 1. Espectro RF (FFT, Peak Hold, PSD en dB).
//! 2. Cascada Espectral (Waterfall 2D temporal en colores).
//! 3. Osciloscopio en Dominio del Tiempo (I, Q y Envolvente |IQ|).
//! 4. Diagrama de Constelación (Plano Complejo I vs Q con persistencia tipo fósforo).
//! 5. Métricas avanzadas de señal (Potencia RMS, PAPR, SNR estimado, DC offset).

use crate::icons;
use crate::theme::AppTheme;
use crate::ui::spectrogram::fft_radix2;
use egui::{Color32, Frame, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use num_complex::Complex32;
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VirtualSinkTab {
    #[default]
    Spectrum,
    Waterfall,
    TimeDomain,
    Constellation,
    Metrics,
}

impl VirtualSinkTab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Spectrum => "Espectro RF (FFT)",
            Self::Waterfall => "Cascada (Waterfall)",
            Self::TimeDomain => "Dominio del Tiempo",
            Self::Constellation => "Constelación I/Q",
            Self::Metrics => "Métricas de Señal",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Spectrum => icons::ICON_QUERY_STATS.codepoint,
            Self::Waterfall => icons::ICON_GRADIENT.codepoint,
            Self::TimeDomain => icons::ICON_SHOW_CHART.codepoint,
            Self::Constellation => icons::ICON_SCATTER_PLOT.codepoint,
            Self::Metrics => icons::ICON_SPEED.codepoint,
        }
    }
}

/// Estado del instrumento virtual
#[derive(Debug, Clone)]
pub struct VirtualSinkModal {
    pub open: bool,
    pub active_tab: VirtualSinkTab,
    // Control de reproducción
    pub is_playing: bool,
    pub playback_speed: f32,
    pub sample_cursor: usize,
    pub last_tick: Option<Instant>,
    // Configuración FFT / Espectro
    pub fft_size: usize,
    pub peak_hold: bool,
    pub peak_buffer: Vec<f32>,
    pub avg_buffer: Vec<f32>,
    pub db_min: f32,
    pub db_max: f32,
    // Cascada Waterfall
    pub waterfall_history: VecDeque<Vec<f32>>,
    pub max_waterfall_rows: usize,
    // Dominio del tiempo
    pub show_i: bool,
    pub show_q: bool,
    pub show_envelope: bool,
    pub time_window_samples: usize,
    // Constelación
    pub constellation_points: usize,
}

impl Default for VirtualSinkModal {
    fn default() -> Self {
        Self {
            open: false,
            active_tab: VirtualSinkTab::Spectrum,
            is_playing: true,
            playback_speed: 1.0,
            sample_cursor: 0,
            last_tick: None,
            fft_size: 1024,
            peak_hold: true,
            peak_buffer: Vec::new(),
            avg_buffer: Vec::new(),
            db_min: -80.0,
            db_max: 0.0,
            waterfall_history: VecDeque::new(),
            max_waterfall_rows: 80,
            show_i: true,
            show_q: true,
            show_envelope: true,
            time_window_samples: 512,
            constellation_points: 1024,
        }
    }
}

impl VirtualSinkModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_modal(&mut self) {
        self.open = true;
        self.is_playing = true;
        self.last_tick = Some(Instant::now());
    }

    pub fn close(&mut self) {
        self.open = false;
        self.last_tick = None;
    }

    /// Actualiza el cursor de reproducción según el tiempo transcurrido y la tasa de muestreo
    pub fn update_playback(&mut self, total_samples: usize, sample_rate: u32) {
        if total_samples == 0 || sample_rate == 0 {
            return;
        }

        let now = Instant::now();
        if let Some(prev) = self.last_tick {
            if self.is_playing {
                let dt = now.duration_since(prev).as_secs_f32();
                let samples_to_advance = (sample_rate as f32 * dt * self.playback_speed).round() as usize;
                self.sample_cursor = (self.sample_cursor + samples_to_advance) % total_samples;
            }
        }
        self.last_tick = Some(now);
    }

    /// Renderiza la ventana modal a pantalla completa
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) {
        if !self.open {
            return;
        }

        self.update_playback(samples.len(), sample_rate);

        // Repintar activamente mientras se reproduce
        if self.is_playing {
            ctx.request_repaint();
        }

        let mut is_open = self.open;
        egui::Window::new(RichText::new(format!("{} Instrumento Virtual de Laboratorio", icons::ICON_QUERY_STATS.codepoint)).strong().size(16.0))
            .open(&mut is_open)
            .default_size(Vec2::new(960.0, 680.0))
            .min_size(Vec2::new(600.0, 480.0))
            .resizable(true)
            .collapsible(false)
            .frame(
                Frame::new()
                    .fill(if *theme == AppTheme::Dark { Color32::from_rgb(14, 18, 25) } else { Color32::from_rgb(248, 250, 252) })
                    .stroke(Stroke::new(1.0, theme.card_border()))
                    .corner_radius(12.0)
                    .inner_margin(12.0),
            )
            .show(ctx, |ui| {
                self.render_content(ui, theme, samples, sample_rate, center_freq_hz);
            });
        self.open = is_open;
    }

    fn render_content(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) {
        ui.vertical(|ui| {
            // 1. Barra superior: Pestañas de análisis
            ui.horizontal_wrapped(|ui| {
                let tabs = [
                    VirtualSinkTab::Spectrum,
                    VirtualSinkTab::Waterfall,
                    VirtualSinkTab::TimeDomain,
                    VirtualSinkTab::Constellation,
                    VirtualSinkTab::Metrics,
                ];

                for tab in tabs {
                    let is_active = self.active_tab == tab;
                    let bg_color = if is_active {
                        theme.active_pill()
                    } else {
                        theme.card_bg()
                    };
                    let stroke = Stroke::new(
                        1.0,
                        if is_active { theme.accent_blue() } else { theme.card_border() },
                    );

                    let pill = egui::Button::new(
                        RichText::new(format!("{} {}", tab.icon(), tab.name()))
                            .strong()
                            .size(12.0)
                            .color(if is_active { theme.accent_blue() } else { theme.text_secondary() }),
                    )
                    .fill(bg_color)
                    .stroke(stroke)
                    .corner_radius(14.0);

                    if ui.add(pill).clicked() {
                        self.active_tab = tab;
                    }
                }
            });

            ui.add_space(8.0);

            // 2. Área principal de graficación
            let avail_size = Vec2::new(ui.available_width(), (ui.available_height() - 72.0).max(300.0));
            Frame::new()
                .fill(if *theme == AppTheme::Dark { Color32::from_rgb(8, 12, 18) } else { Color32::from_rgb(255, 255, 255) })
                .stroke(Stroke::new(1.0, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    match self.active_tab {
                        VirtualSinkTab::Spectrum => {
                            self.render_spectrum_view(ui, theme, samples, sample_rate, center_freq_hz, avail_size);
                        }
                        VirtualSinkTab::Waterfall => {
                            self.render_waterfall_view(ui, theme, samples, sample_rate, center_freq_hz, avail_size);
                        }
                        VirtualSinkTab::TimeDomain => {
                            self.render_time_domain_view(ui, theme, samples, sample_rate, avail_size);
                        }
                        VirtualSinkTab::Constellation => {
                            self.render_constellation_view(ui, theme, samples, avail_size);
                        }
                        VirtualSinkTab::Metrics => {
                            self.render_metrics_view(ui, theme, samples, sample_rate, center_freq_hz);
                        }
                    }
                });

            ui.add_space(8.0);

            // 3. Barra inferior de control de reproducción y simulación en tiempo real
            self.render_playback_controls(ui, theme, samples.len(), sample_rate);
        });
    }

    /// Barra de controles inferiores (Play, Pause, Cursor de tiempo, Velocidad, FFT size)
    fn render_playback_controls(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        total_samples: usize,
        sample_rate: u32,
    ) {
        ui.horizontal(|ui| {
            // Botón Play / Pause
            let play_label = if self.is_playing {
                format!("{} Pausar", icons::ICON_PAUSE.codepoint)
            } else {
                format!("{} Reproducir", icons::ICON_PLAY_ARROW.codepoint)
            };
            let play_btn = egui::Button::new(RichText::new(play_label).strong().size(12.0).color(Color32::WHITE))
                .fill(if self.is_playing { Color32::from_rgb(220, 38, 38) } else { theme.accent_blue() })
                .corner_radius(6.0);
            if ui.add(play_btn).clicked() {
                self.is_playing = !self.is_playing;
            }

            // Botón Replay / Reiniciar
            let reset_btn = egui::Button::new(RichText::new(format!("{} Reiniciar", icons::ICON_REPLAY.codepoint)).size(12.0));
            if ui.add(reset_btn).clicked() {
                self.sample_cursor = 0;
                self.peak_buffer.clear();
                self.waterfall_history.clear();
            }

            ui.add_space(8.0);

            // Control deslizante de posición temporal en la señal
            if total_samples > 0 && sample_rate > 0 {
                let total_secs = total_samples as f32 / sample_rate as f32;
                let current_secs = self.sample_cursor as f32 / sample_rate as f32;
                let mut cursor_val = self.sample_cursor;

                let slider_w = (ui.available_width() - 340.0).max(100.0);
                let slider = egui::Slider::new(&mut cursor_val, 0..=total_samples)
                    .show_value(false);

                if ui.add_sized([slider_w, 20.0], slider).changed() {
                    self.sample_cursor = cursor_val;
                }

                ui.label(
                    RichText::new(format!(
                        "{:02}:{:04.1} / {:02}:{:04.1}",
                        (current_secs / 60.0).floor() as u32,
                        current_secs % 60.0,
                        (total_secs / 60.0).floor() as u32,
                        total_secs % 60.0,
                    ))
                    .monospace()
                    .size(11.5)
                    .color(theme.text_secondary()),
                );
            }

            // Selector de velocidad
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("sink_fft_size")
                    .selected_text(format!("FFT: {}", self.fft_size))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.fft_size, 512, "512 bins");
                        ui.selectable_value(&mut self.fft_size, 1024, "1024 bins");
                        ui.selectable_value(&mut self.fft_size, 2048, "2048 bins");
                    });

                egui::ComboBox::from_id_salt("sink_speed_combo")
                    .selected_text(format!("{}x", self.playback_speed))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.playback_speed, 0.25, "0.25x");
                        ui.selectable_value(&mut self.playback_speed, 0.5, "0.5x");
                        ui.selectable_value(&mut self.playback_speed, 1.0, "1.0x (Real)");
                        ui.selectable_value(&mut self.playback_speed, 2.0, "2.0x");
                        ui.selectable_value(&mut self.playback_speed, 5.0, "5.0x");
                    });
            });
        });
    }

    /// Calcula la FFT de la ventana actual de muestras con ventana Hann
    fn compute_current_fft(&mut self, samples: &[(f32, f32)]) -> Vec<f32> {
        let n = self.fft_size;
        let mut buffer = vec![Complex32::new(0.0, 0.0); n];

        if samples.is_empty() {
            return vec![-100.0; n];
        }

        let start = self.sample_cursor;
        for i in 0..n {
            let sample_idx = (start + i) % samples.len();
            let (re, im) = samples[sample_idx];
            // Ventana Hann
            let win = 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos());
            buffer[i] = Complex32::new(re * win, im * win);
        }

        fft_radix2(&mut buffer);

        // Reordenamiento FFT Shift (frecuencias negativas a la izquierda, positivas a la derecha)
        let mut db_vals = vec![0.0f32; n];
        for i in 0..n {
            let raw_idx = (i + n / 2) % n;
            let pwr = buffer[raw_idx].norm_sqr() / (n as f32);
            let db = (10.0 * (pwr + 1e-12).log10()).clamp(-120.0, 10.0);
            db_vals[i] = db;
        }

        // Actualizar Peak Hold
        if self.peak_buffer.len() != n {
            self.peak_buffer = db_vals.clone();
        } else if self.peak_hold {
            for i in 0..n {
                self.peak_buffer[i] = self.peak_buffer[i].max(db_vals[i]) * 0.999 + db_vals[i] * 0.001;
            }
        }

        db_vals
    }

    // ==========================================
    // 1. PESTAÑA: ESPECTRO RF (FFT)
    // ==========================================
    fn render_spectrum_view(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
        size: Vec2,
    ) {
        let db_vals = self.compute_current_fft(samples);
        let n = db_vals.len();

        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);

        // Fondo oscuro de cuadrícula
        painter.rect_filled(rect, 4.0, if *theme == AppTheme::Dark { Color32::from_rgb(6, 10, 16) } else { Color32::from_rgb(245, 247, 250) });

        let margin_l = 45.0;
        let margin_b = 25.0;
        let plot_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + margin_l, rect.min.y + 10.0),
            Pos2::new(rect.max.x - 10.0, rect.max.y - margin_b),
        );

        let grid_col = if *theme == AppTheme::Dark { Color32::from_rgb(30, 40, 55) } else { Color32::from_rgb(225, 232, 240) };
        let text_col = theme.text_secondary();

        // Líneas de cuadrícula horizontal (dB: 0, -20, -40, -60, -80)
        let db_ticks = [0.0, -20.0, -40.0, -60.0, -80.0];
        for &db in &db_ticks {
            let t = (db - self.db_min) / (self.db_max - self.db_min);
            let y = plot_rect.max.y - t * plot_rect.height();
            painter.line_segment([Pos2::new(plot_rect.min.x, y), Pos2::new(plot_rect.max.x, y)], Stroke::new(1.0, grid_col));
            painter.text(Pos2::new(rect.min.x + 36.0, y), egui::Align2::RIGHT_CENTER, format!("{:.0} dB", db), egui::FontId::monospace(10.0), text_col);
        }

        // Líneas de cuadrícula vertical (Frecuencia: -Fs/2 a +Fs/2)
        let freq_steps = 6;
        for i in 0..=freq_steps {
            let t = i as f32 / freq_steps as f32;
            let x = plot_rect.min.x + t * plot_rect.width();
            painter.line_segment([Pos2::new(x, plot_rect.min.y), Pos2::new(x, plot_rect.max.y)], Stroke::new(1.0, grid_col));

            let freq_offset_khz = (-0.5 + t) * (sample_rate as f32 / 1000.0);
            let freq_label = if center_freq_hz > 0.0 {
                let actual_mhz = (center_freq_hz + (freq_offset_khz as f64 * 1000.0)) / 1e6;
                format!("{:.3}", actual_mhz)
            } else {
                format!("{:+.0}k", freq_offset_khz)
            };
            painter.text(Pos2::new(x, rect.max.y - 12.0), egui::Align2::CENTER_CENTER, freq_label, egui::FontId::monospace(10.0), text_col);
        }

        // Línea vertical central
        let center_x = plot_rect.center().x;
        painter.line_segment(
            [Pos2::new(center_x, plot_rect.min.y), Pos2::new(center_x, plot_rect.max.y)],
            Stroke::new(1.0, Color32::from_rgb(239, 68, 68).gamma_multiply(0.6)),
        );

        if n > 1 {
            // 1. Trazado Peak Hold (Amarillo)
            if self.peak_hold && self.peak_buffer.len() == n {
                let peak_pts: Vec<Pos2> = (0..n).map(|i| {
                    let tx = i as f32 / (n - 1) as f32;
                    let ty = ((self.peak_buffer[i] - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                    Pos2::new(plot_rect.min.x + tx * plot_rect.width(), plot_rect.max.y - ty * plot_rect.height())
                }).collect();

                for w in peak_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.0, Color32::from_rgb(234, 179, 8).gamma_multiply(0.8)));
                }
            }

            // 2. Trazado Espectro Activo (Cian brillante con relleno)
            let live_pts: Vec<Pos2> = (0..n).map(|i| {
                let tx = i as f32 / (n - 1) as f32;
                let ty = ((db_vals[i] - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                Pos2::new(plot_rect.min.x + tx * plot_rect.width(), plot_rect.max.y - ty * plot_rect.height())
            }).collect();

            for w in live_pts.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(1.5, Color32::from_rgb(56, 189, 248)));
            }
        }

        // Leyenda
        let legend_pos = Pos2::new(plot_rect.max.x - 140.0, plot_rect.min.y + 12.0);
        painter.rect_filled(Rect::from_min_size(legend_pos, Vec2::new(130.0, 36.0)), 4.0, Color32::from_black_alpha(160));
        painter.circle_filled(legend_pos + Vec2::new(12.0, 12.0), 4.0, Color32::from_rgb(56, 189, 248));
        painter.text(legend_pos + Vec2::new(22.0, 12.0), egui::Align2::LEFT_CENTER, "Espectro I/Q", egui::FontId::proportional(11.0), Color32::WHITE);
        painter.circle_filled(legend_pos + Vec2::new(12.0, 26.0), 4.0, Color32::from_rgb(234, 179, 8));
        painter.text(legend_pos + Vec2::new(22.0, 26.0), egui::Align2::LEFT_CENTER, "Peak Hold", egui::FontId::proportional(11.0), Color32::WHITE);
    }

    // ==========================================
    // 2. PESTAÑA: CASCADA (WATERFALL)
    // ==========================================
    fn render_waterfall_view(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        _sample_rate: u32,
        _center_freq_hz: f64,
        size: Vec2,
    ) {
        let db_vals = self.compute_current_fft(samples);

        // Agregar fila a la historia de la cascada
        if self.is_playing {
            self.waterfall_history.push_front(db_vals);
            if self.waterfall_history.len() > self.max_waterfall_rows {
                self.waterfall_history.pop_back();
            }
        }

        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::from_rgb(4, 6, 10));

        let rows = self.waterfall_history.len();
        if rows == 0 {
            return;
        }

        let row_height = rect.height() / self.max_waterfall_rows as f32;

        for (r_idx, row_db) in self.waterfall_history.iter().enumerate() {
            let y0 = rect.min.y + r_idx as f32 * row_height;
            let y1 = y0 + row_height.max(1.0);
            let n_cols = row_db.len();
            let col_step = (n_cols / 64).max(1);

            for c in (0..n_cols).step_by(col_step) {
                let t_x0 = c as f32 / n_cols as f32;
                let t_x1 = (c + col_step) as f32 / n_cols as f32;
                let x0 = rect.min.x + t_x0 * rect.width();
                let x1 = rect.min.x + t_x1 * rect.width();

                let db = row_db[c];
                let norm = ((db - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                let col = waterfall_colormap(norm);

                painter.rect_filled(Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)), 0.0, col);
            }
        }

        // Título discreto de ayuda
        painter.text(
            Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            format!("Cascada Temporal ({}/{} tramas)", rows, self.max_waterfall_rows),
            egui::FontId::monospace(11.0),
            theme.text_secondary(),
        );
    }

    // ==========================================
    // 3. PESTAÑA: DOMINIO DEL TIEMPO (OSCILOSCOPIO)
    // ==========================================
    fn render_time_domain_view(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        _sample_rate: u32,
        size: Vec2,
    ) {
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 4.0, if *theme == AppTheme::Dark { Color32::from_rgb(6, 10, 16) } else { Color32::from_rgb(245, 247, 250) });

        let margin_l = 40.0;
        let margin_b = 25.0;
        let plot_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + margin_l, rect.min.y + 10.0),
            Pos2::new(rect.max.x - 10.0, rect.max.y - margin_b),
        );

        let grid_col = if *theme == AppTheme::Dark { Color32::from_rgb(30, 40, 55) } else { Color32::from_rgb(225, 232, 240) };
        let text_col = theme.text_secondary();

        // Cuadrícula vertical de amplitud (-1.0 a +1.0)
        let amp_ticks = [-1.0, -0.5, 0.0, 0.5, 1.0];
        for &amp in &amp_ticks {
            let t = (amp + 1.2) / 2.4;
            let y = plot_rect.max.y - t * plot_rect.height();
            painter.line_segment([Pos2::new(plot_rect.min.x, y), Pos2::new(plot_rect.max.x, y)], Stroke::new(1.0, grid_col));
            painter.text(Pos2::new(rect.min.x + 32.0, y), egui::Align2::RIGHT_CENTER, format!("{:+.1}", amp), egui::FontId::monospace(10.0), text_col);
        }

        // Ventana de muestras a graficar
        let n = self.time_window_samples.min(samples.len().max(1));
        if samples.is_empty() || n == 0 {
            return;
        }

        let start = self.sample_cursor;
        let mut pts_i = Vec::with_capacity(n);
        let mut pts_q = Vec::with_capacity(n);
        let mut pts_env = Vec::with_capacity(n);

        for i in 0..n {
            let idx = (start + i) % samples.len();
            let (re, im) = samples[idx];
            let env = (re * re + im * im).sqrt();

            let tx = i as f32 / (n - 1) as f32;
            let x = plot_rect.min.x + tx * plot_rect.width();

            let ty_i = ((re + 1.2) / 2.4).clamp(0.0, 1.0);
            let ty_q = ((im + 1.2) / 2.4).clamp(0.0, 1.0);
            let ty_env = ((env + 1.2) / 2.4).clamp(0.0, 1.0);

            pts_i.push(Pos2::new(x, plot_rect.max.y - ty_i * plot_rect.height()));
            pts_q.push(Pos2::new(x, plot_rect.max.y - ty_q * plot_rect.height()));
            pts_env.push(Pos2::new(x, plot_rect.max.y - ty_env * plot_rect.height()));
        }

        // Trazar I (Azul)
        if self.show_i {
            for w in pts_i.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(1.5, Color32::from_rgb(59, 130, 246)));
            }
        }
        // Trazar Q (Verde)
        if self.show_q {
            for w in pts_q.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(1.5, Color32::from_rgb(16, 185, 129)));
            }
        }
        // Trazar Envolvente (Naranja / Oro)
        if self.show_envelope {
            for w in pts_env.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(1.2, Color32::from_rgb(245, 158, 11).gamma_multiply(0.8)));
            }
        }

        // Leyenda
        let legend_pos = Pos2::new(plot_rect.max.x - 170.0, plot_rect.min.y + 12.0);
        painter.rect_filled(Rect::from_min_size(legend_pos, Vec2::new(160.0, 48.0)), 4.0, Color32::from_black_alpha(170));
        painter.circle_filled(legend_pos + Vec2::new(12.0, 12.0), 4.0, Color32::from_rgb(59, 130, 246));
        painter.text(legend_pos + Vec2::new(22.0, 12.0), egui::Align2::LEFT_CENTER, "En fase: I(t)", egui::FontId::proportional(11.0), Color32::WHITE);
        painter.circle_filled(legend_pos + Vec2::new(12.0, 24.0), 4.0, Color32::from_rgb(16, 185, 129));
        painter.text(legend_pos + Vec2::new(22.0, 24.0), egui::Align2::LEFT_CENTER, "Cuadratura: Q(t)", egui::FontId::proportional(11.0), Color32::WHITE);
        painter.circle_filled(legend_pos + Vec2::new(12.0, 36.0), 4.0, Color32::from_rgb(245, 158, 11));
        painter.text(legend_pos + Vec2::new(22.0, 36.0), egui::Align2::LEFT_CENTER, "Envolvente |IQ|", egui::FontId::proportional(11.0), Color32::WHITE);
    }

    // ==========================================
    // 4. PESTAÑA: DIAGRAMA DE CONSTELACIÓN (I vs Q)
    // ==========================================
    fn render_constellation_view(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        size: Vec2,
    ) {
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 4.0, if *theme == AppTheme::Dark { Color32::from_rgb(6, 10, 16) } else { Color32::from_rgb(245, 247, 250) });

        let plot_side = rect.width().min(rect.height()) - 30.0;
        let plot_rect = Rect::from_center_size(rect.center(), Vec2::splat(plot_side));

        let center = plot_rect.center();
        let scale = plot_side * 0.42;

        let grid_col = if *theme == AppTheme::Dark { Color32::from_rgb(30, 42, 58) } else { Color32::from_rgb(220, 228, 238) };

        // Círculos concéntricos de referencia (r = 0.5, 1.0)
        painter.circle_stroke(center, scale * 0.5, Stroke::new(1.0, grid_col));
        painter.circle_stroke(center, scale * 1.0, Stroke::new(1.5, grid_col));

        // Ejes ortogonales I y Q
        painter.line_segment([Pos2::new(plot_rect.min.x, center.y), Pos2::new(plot_rect.max.x, center.y)], Stroke::new(1.0, grid_col));
        painter.line_segment([Pos2::new(center.x, plot_rect.min.y), Pos2::new(center.x, plot_rect.max.y)], Stroke::new(1.0, grid_col));

        painter.text(Pos2::new(plot_rect.max.x - 8.0, center.y - 12.0), egui::Align2::RIGHT_CENTER, "+I (Fase)", egui::FontId::monospace(11.0), theme.accent_blue());
        painter.text(Pos2::new(center.x + 8.0, plot_rect.min.y + 12.0), egui::Align2::LEFT_CENTER, "+Q (Cuadratura)", egui::FontId::monospace(11.0), theme.accent_blue());

        if samples.is_empty() {
            return;
        }

        let n = self.constellation_points.min(samples.len());
        let start = self.sample_cursor;

        // Dibujar puntos de constelación con persistencia tipo fósforo
        for i in 0..n {
            let idx = (start + i) % samples.len();
            let (re, im) = samples[idx];

            let pt = Pos2::new(center.x + re * scale, center.y - im * scale);

            let age_factor = i as f32 / n as f32; // Muestras más recientes más brillantes
            let alpha = (60.0 + age_factor * 195.0) as u8;
            let dot_col = Color32::from_rgba_premultiplied(56, 189, 248, alpha);

            painter.circle_filled(pt, 1.8, dot_col);
        }
    }

    // ==========================================
    // 5. PESTAÑA: MÉTRICAS DE SEÑAL
    // ==========================================
    fn render_metrics_view(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) {
        if samples.is_empty() {
            ui.label("No hay muestras cargadas para calcular métricas.");
            return;
        }

        // Cálculo de métricas sobre un bloque representativo
        let n = samples.len().min(8192);
        let mut sum_sq = 0.0f32;
        let mut peak_sq = 0.0f32;
        let mut dc_i = 0.0f32;
        let mut dc_q = 0.0f32;

        for &(i, q) in &samples[..n] {
            let mag_sq = i * i + q * q;
            sum_sq += mag_sq;
            if mag_sq > peak_sq {
                peak_sq = mag_sq;
            }
            dc_i += i;
            dc_q += q;
        }

        let rms_pwr = sum_sq / (n as f32);
        let rms_db = 10.0 * (rms_pwr + 1e-12).log10();
        let peak_db = 10.0 * (peak_sq + 1e-12).log10();
        let papr_db = (peak_db - rms_db).max(0.0);
        let dc_i_mean = dc_i / (n as f32);
        let dc_q_mean = dc_q / (n as f32);

        ui.vertical(|ui| {
            ui.label(RichText::new("Telemetría y Diagnóstico de Señal").strong().size(16.0).color(theme.accent_blue()));
            ui.label(RichText::new("Parámetros de potencia RF, factor de cresta y offset calculados sobre la señal activa.").size(12.0).color(theme.text_secondary()));
            ui.add_space(12.0);

            ui.columns(3, |cols| {
                // Tarjeta 1: Potencias
                Frame::new().fill(theme.card_bg()).stroke(Stroke::new(1.0, theme.card_border())).corner_radius(8.0).inner_margin(12.0).show(&mut cols[0], |ui| {
                    ui.label(RichText::new("Potencia de Transmisión").strong().color(theme.text_primary()));
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Potencia Pico: {:.2} dBFS", peak_db)).size(13.0).color(theme.accent_blue()));
                    ui.label(RichText::new(format!("Potencia RMS: {:.2} dBFS", rms_db)).size(13.0));
                    ui.label(RichText::new(format!("PAPR / Cresta: {:.2} dB", papr_db)).size(13.0).color(Color32::from_rgb(234, 179, 8)));
                });

                // Tarjeta 2: Balance I/Q y DC Offset
                Frame::new().fill(theme.card_bg()).stroke(Stroke::new(1.0, theme.card_border())).corner_radius(8.0).inner_margin(12.0).show(&mut cols[1], |ui| {
                    ui.label(RichText::new("Balance y Offset DC").strong().color(theme.text_primary()));
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("DC Offset I: {:+.4}", dc_i_mean)).size(13.0));
                    ui.label(RichText::new(format!("DC Offset Q: {:+.4}", dc_q_mean)).size(13.0));
                    let isolation = 10.0 * (1.0 / (dc_i_mean.abs() + dc_q_mean.abs() + 1e-6)).log10();
                    ui.label(RichText::new(format!("Aislamiento Carrier: {:.1} dB", isolation)).size(13.0).color(Color32::from_rgb(34, 197, 94)));
                });

                // Tarjeta 3: Configuración de Frecuencia
                Frame::new().fill(theme.card_bg()).stroke(Stroke::new(1.0, theme.card_border())).corner_radius(8.0).inner_margin(12.0).show(&mut cols[2], |ui| {
                    ui.label(RichText::new("Parámetros de RF").strong().color(theme.text_primary()));
                    ui.add_space(6.0);
                    if center_freq_hz > 0.0 {
                        ui.label(RichText::new(format!("Frecuencia Central: {:.3} MHz", center_freq_hz / 1e6)).size(13.0).color(theme.accent_blue()));
                    } else {
                        ui.label(RichText::new("Frecuencia Central: Baseband (0 Hz)").size(13.0));
                    }
                    ui.label(RichText::new(format!("Sample Rate: {} Sps", sample_rate)).size(13.0));
                    ui.label(RichText::new(format!("Ancho de Banda Nyquist: {} kHz", sample_rate / 1000)).size(13.0));
                });
            });
        });
    }
}

/// Mapa de color para cascada espectral tipo SDR (Azul -> Cian -> Verde -> Amarillo -> Rojo -> Blanco)
fn waterfall_colormap(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.2 {
        let u = t / 0.2;
        Color32::from_rgb(0, 0, (u * 200.0) as u8)
    } else if t < 0.4 {
        let u = (t - 0.2) / 0.2;
        Color32::from_rgb(0, (u * 220.0) as u8, 220)
    } else if t < 0.6 {
        let u = (t - 0.4) / 0.2;
        Color32::from_rgb((u * 220.0) as u8, 255, ((1.0 - u) * 220.0) as u8)
    } else if t < 0.8 {
        let u = (t - 0.6) / 0.2;
        Color32::from_rgb(255, (255.0 - u * 100.0) as u8, 0)
    } else {
        let u = ((t - 0.8) / 0.2).clamp(0.0, 1.0);
        Color32::from_rgb(255, (155.0 + u * 100.0).round() as u8, (u * 255.0).round() as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_sink_defaults() {
        let mut sink = VirtualSinkModal::new();
        assert!(!sink.open);
        assert_eq!(sink.active_tab, VirtualSinkTab::Spectrum);
        assert_eq!(sink.fft_size, 1024);

        sink.open_modal();
        assert!(sink.open);
        assert!(sink.is_playing);

        sink.close();
        assert!(!sink.open);
    }

    #[test]
    fn test_colormap_bounds() {
        let c0 = waterfall_colormap(0.0);
        let c1 = waterfall_colormap(1.0);
        assert_eq!(c0.r(), 0);
        assert_eq!(c1.r(), 255);
        assert_eq!(c1.b(), 255);
    }

    #[test]
    fn test_fft_computation() {
        let mut sink = VirtualSinkModal::new();
        sink.fft_size = 512;
        let samples = vec![(1.0f32, 0.0f32); 1024];
        let db = sink.compute_current_fft(&samples);
        assert_eq!(db.len(), 512);
        assert_eq!(sink.peak_buffer.len(), 512);
    }
}
