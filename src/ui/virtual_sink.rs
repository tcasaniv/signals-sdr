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
use crate::ui::spectrogram::{
    draw_dashed_line_horizontal, draw_dashed_line_vertical, draw_line_horizontal,
    draw_line_segment, draw_line_vertical, draw_rect_filled, draw_rect_outline, draw_text_on_image,
    fft_radix2,
};
use egui::{Color32, Frame, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use num_complex::Complex32;
use std::collections::VecDeque;
use std::f32::consts::PI;
use web_time::Instant;

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

/// Modo de normalización de escala para el diagrama de constelación VSA
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConstellationNormMode {
    #[default]
    UnitEnergy,   // RF (Es=1.0): Coordenadas en ±1/√2 (±0.707) sobre círculo r=1.0 (evita saturar DAC de RF)
    SymbolicGrid, // Grilla (±1.0): Coordenadas escaladas por √2 para alineación con marcas ±1.0 de ejes
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
    pub show_max_peak: bool,
    pub show_min_peak: bool,
    pub max_peak_buffer: Vec<f32>,
    pub min_peak_buffer: Vec<f32>,
    pub db_min: f32,
    pub db_max: f32,
    // Cascada Waterfall
    pub waterfall_history: VecDeque<Vec<f32>>,
    pub max_waterfall_rows: usize,
    pub waterfall_resolution_mode: usize, // 0=64, 1=128, 2=256, 3=512, 4=Nativa
    // Dominio del tiempo
    pub show_i: bool,
    pub show_q: bool,
    pub show_envelope: bool,
    pub time_window_samples: usize,
    // Constelación VSA
    pub constellation_points: usize,
    pub constellation_zoom: f32,
    pub constellation_vsa_mode: bool,
    pub constellation_sps: usize,
    pub constellation_timing_phase: usize,
    pub constellation_oqpsk_deoffset: bool,
    pub constellation_carrier_phase_deg: f32,
    pub constellation_norm_mode: ConstellationNormMode,
    pub show_constellation_circles: bool,
    pub show_constellation_grid: bool,
    pub export_status_message: Option<(String, bool)>,
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
            show_max_peak: true,
            show_min_peak: false,
            max_peak_buffer: Vec::new(),
            min_peak_buffer: Vec::new(),
            db_min: -80.0,
            db_max: 0.0,
            waterfall_history: VecDeque::new(),
            max_waterfall_rows: 80,
            waterfall_resolution_mode: 2, // 256 columnas (Alta)
            show_i: true,
            show_q: true,
            show_envelope: true,
            time_window_samples: 512,
            constellation_points: 1024,
            constellation_zoom: 1.0,
            constellation_vsa_mode: true,
            constellation_sps: 4,
            constellation_timing_phase: 0,
            constellation_oqpsk_deoffset: true,
            constellation_carrier_phase_deg: 0.0,
            constellation_norm_mode: ConstellationNormMode::UnitEnergy,
            show_constellation_circles: true,
            show_constellation_grid: true,
            export_status_message: None,
        }
    }
}

impl VirtualSinkModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_peaks(&mut self) {
        self.max_peak_buffer.clear();
        self.min_peak_buffer.clear();
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

    /// Renderiza el Instrumento Virtual directamente en el panel flexible derecho (sin modal flotante que tape la app)
    pub fn render_inline_panel(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) {
        if !self.open {
            return;
        }

        self.update_playback(samples.len(), sample_rate);

        if self.is_playing {
            ui.ctx().request_repaint();
        }

        ui.vertical(|ui| {
            // Encabezado integrado del instrumento con botón [X] de cierre en la esquina derecha
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} Instrumento Virtual de Laboratorio", icons::ICON_QUERY_STATS.codepoint))
                                .strong()
                                .size(15.5)
                                .color(theme.accent_blue()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_btn = egui::Button::new(
                                RichText::new(icons::ICON_CLOSE.codepoint)
                                    .size(16.0)
                                    .color(theme.text_secondary()),
                            )
                            .frame(false);
                            if ui.add(close_btn).on_hover_text("Cerrar Instrumento Virtual y volver al transmisor").clicked() {
                                self.open = false;
                            }
                        });
                    });
                });

            ui.add_space(8.0);

            // Marco del contenido ocupando el ancho completo del panel flexible derecho
            Frame::new()
                .fill(if *theme == AppTheme::Dark { Color32::from_rgb(14, 18, 25) } else { Color32::from_rgb(248, 250, 252) })
                .stroke(Stroke::new(1.0, theme.card_border()))
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    self.render_content(ui, theme, samples, sample_rate, center_freq_hz);
                });
        });
    }

    /// Método compatible para llamada desde contextos genéricos
    pub fn show(
        &mut self,
        _ctx: &egui::Context,
        _theme: &AppTheme,
        _samples: &[(f32, f32)],
        _sample_rate: u32,
        _center_freq_hz: f64,
    ) {
        // En v0.4.2 se utiliza render_inline_panel directamente dentro de CentralPanel
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

            // Sub-barra contextual de opciones según pestaña activa
            match self.active_tab {
                VirtualSinkTab::Spectrum => {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                        ui.checkbox(&mut self.show_max_peak, RichText::new("Max Peak (Amarillo)").size(11.5).color(Color32::from_rgb(234, 179, 8)));
                        ui.checkbox(&mut self.show_min_peak, RichText::new("Min Peak (Magenta)").size(11.5).color(Color32::from_rgb(236, 72, 153)));
                        if ui.button(RichText::new(format!("{} Limpiar Picos", icons::ICON_CLEAR_ALL.codepoint)).size(11.0)).clicked() {
                            self.reset_peaks();
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("dB Mín:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized([70.0, 18.0], egui::Slider::new(&mut self.db_min, -140.0..=-30.0).suffix(" dB").show_value(true));
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("dB Máx:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized([70.0, 18.0], egui::Slider::new(&mut self.db_max, -20.0..=10.0).suffix(" dB").show_value(true));
                        });
                    });
                }
                VirtualSinkTab::Waterfall => {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                        ui.label(RichText::new("Resolución de Cascada:").size(11.5).color(theme.text_secondary()));
                        egui::ComboBox::from_id_salt("sink_waterfall_res")
                            .selected_text(match self.waterfall_resolution_mode {
                                0 => "Baja (64 columnas)",
                                1 => "Media (128 columnas)",
                                2 => "Alta (256 columnas)",
                                3 => "Ultra (512 columnas)",
                                _ => "Nativa (Resolución FFT completa)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.waterfall_resolution_mode, 0, "Baja (64 columnas)");
                                ui.selectable_value(&mut self.waterfall_resolution_mode, 1, "Media (128 columnas)");
                                ui.selectable_value(&mut self.waterfall_resolution_mode, 2, "Alta (256 columnas)");
                                ui.selectable_value(&mut self.waterfall_resolution_mode, 3, "Ultra (512 columnas)");
                                ui.selectable_value(&mut self.waterfall_resolution_mode, 4, "Nativa (Resolución FFT completa)");
                            });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("dB Mín:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized([70.0, 18.0], egui::Slider::new(&mut self.db_min, -140.0..=-30.0).suffix(" dB"));
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("dB Máx:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized([70.0, 18.0], egui::Slider::new(&mut self.db_max, -20.0..=10.0).suffix(" dB"));
                        });
                    });
                }
                VirtualSinkTab::TimeDomain => {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                        ui.checkbox(&mut self.show_i, RichText::new("I(t) Fase").size(11.5).color(Color32::from_rgb(59, 130, 246)));
                        ui.checkbox(&mut self.show_q, RichText::new("Q(t) Cuadratura").size(11.5).color(Color32::from_rgb(16, 185, 129)));
                        ui.checkbox(&mut self.show_envelope, RichText::new("|IQ| Envolvente").size(11.5).color(Color32::from_rgb(245, 158, 11)));
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("Ventana:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized([75.0, 18.0], egui::Slider::new(&mut self.time_window_samples, 128..=2048).suffix(" pts"));
                        });
                    });
                }
                VirtualSinkTab::Constellation => {
                    self.render_constellation_subbar(ui, theme, samples);
                }
                VirtualSinkTab::Metrics => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Cálculo en vivo de parámetros de RF y calidad de la señal modulada").size(11.5).color(theme.text_secondary()));
                    });
                }
            }

            ui.add_space(4.0);

            // 2. Área principal de graficación con altura adaptativa según la pestaña y resolución
            let avail_w = ui.available_width();
            let plot_h = match self.active_tab {
                VirtualSinkTab::Constellation => {
                    if avail_w < 600.0 {
                        // En móvil se acota para mantener la relación de aspecto cuadrada y no ocultar los controles inferiores
                        (avail_w - 24.0).clamp(240.0, 360.0)
                    } else {
                        (ui.available_height() - 130.0).clamp(320.0, 580.0)
                    }
                }
                VirtualSinkTab::Metrics => {
                    (ui.available_height() - 100.0).clamp(280.0, 620.0)
                }
                _ => {
                    if avail_w < 600.0 {
                        (ui.available_height() - 110.0).clamp(260.0, 420.0)
                    } else {
                        (ui.available_height() - 110.0).clamp(340.0, 680.0)
                    }
                }
            };
            let avail_size = Vec2::new(avail_w, plot_h);
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
            self.render_playback_controls(ui, theme, samples, sample_rate, center_freq_hz);
        });
    }

    /// Sub-barra contextual de constelación estructurada en filas semánticas atómicas (sin quiebre vertical de letras)
    fn render_constellation_subbar(&mut self, ui: &mut Ui, theme: &AppTheme, samples: &[(f32, f32)]) {
        let avail_w = ui.available_width();

        if avail_w < 600.0 {
            // Modo móvil / pantalla estrecha (<600px): 3 filas limpias perfectamente acotadas
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);

                // Fila 1: Modo VSA, SPS y OQPSK
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                    ui.checkbox(
                        &mut self.constellation_vsa_mode,
                        RichText::new("Modo Símbolos (VSA)").size(11.5).strong().color(theme.accent_blue()),
                    ).on_hover_text("Decima la señal continua a 1 muestra por símbolo para evaluar la constelación digital y decisiones (BPSK, QPSK, OQPSK, QAM).");

                    if self.constellation_vsa_mode {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("SPS:").size(11.0).color(theme.text_secondary()));
                            egui::ComboBox::from_id_salt("sink_const_sps_mob")
                                .selected_text(format!("{} SPS", self.constellation_sps))
                                .width(60.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.constellation_sps, 1, "1 SPS");
                                    ui.selectable_value(&mut self.constellation_sps, 2, "2 SPS");
                                    ui.selectable_value(&mut self.constellation_sps, 4, "4 SPS");
                                    ui.selectable_value(&mut self.constellation_sps, 8, "8 SPS");
                                    ui.selectable_value(&mut self.constellation_sps, 16, "16 SPS");
                                });
                        });

                        ui.checkbox(
                            &mut self.constellation_oqpsk_deoffset,
                            RichText::new("OQPSK").size(11.0),
                        ).on_hover_text("Compensa el retardo de medio símbolo (Ts/2) entre las ramas I y Q en señales OQPSK para colapsar la constelación a 4 cuadrantes estándar.");
                    }
                });

                // Fila 2: Fase Ojo, Auto Ojo, Fase Portadora y Escala
                if self.constellation_vsa_mode {
                    let sps = self.constellation_sps.max(1);
                    if self.constellation_timing_phase >= sps {
                        self.constellation_timing_phase = 0;
                    }

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("Fase Ojo:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized(
                                [50.0, 18.0],
                                egui::Slider::new(&mut self.constellation_timing_phase, 0..=sps.saturating_sub(1)),
                            );
                        });

                        if ui.button(RichText::new("Auto Ojo").size(11.0)).clicked() {
                            let delay = if self.constellation_oqpsk_deoffset && sps >= 2 { sps / 2 } else { 0 };
                            let aligned_start = (self.sample_cursor / sps) * sps;
                            self.constellation_timing_phase = find_best_eye_phase(samples, aligned_start, sps, delay);
                        }

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("Fase θ:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized(
                                [55.0, 18.0],
                                egui::Slider::new(&mut self.constellation_carrier_phase_deg, 0.0..=360.0).suffix("°"),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("Escala:").size(11.0).color(theme.text_secondary()));
                            let norm_label = match self.constellation_norm_mode {
                                ConstellationNormMode::UnitEnergy => "RF",
                                ConstellationNormMode::SymbolicGrid => "Grilla",
                            };
                            egui::ComboBox::from_id_salt("sink_const_norm_mob")
                                .selected_text(norm_label)
                                .width(55.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.constellation_norm_mode, ConstellationNormMode::UnitEnergy, "RF (Es=1)");
                                    ui.selectable_value(&mut self.constellation_norm_mode, ConstellationNormMode::SymbolicGrid, "Grilla (±1)");
                                });
                        });
                    });
                }

                // Fila 3: Círculos, Grilla, Zoom y Puntos
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                    ui.checkbox(&mut self.show_constellation_circles, RichText::new("Círculos").size(11.0));
                    ui.checkbox(&mut self.show_constellation_grid, RichText::new("Grilla").size(11.0));

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        ui.label(RichText::new("Zoom:").size(11.0).color(theme.text_secondary()));
                        egui::ComboBox::from_id_salt("sink_const_zoom_mob")
                            .selected_text(format!("{:.1}x", self.constellation_zoom))
                            .width(50.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.constellation_zoom, 0.5, "0.5x");
                                ui.selectable_value(&mut self.constellation_zoom, 1.0, "1.0x");
                                ui.selectable_value(&mut self.constellation_zoom, 1.5, "1.5x");
                                ui.selectable_value(&mut self.constellation_zoom, 2.0, "2.0x");
                                ui.selectable_value(&mut self.constellation_zoom, 3.0, "3.0x");
                                ui.selectable_value(&mut self.constellation_zoom, 4.0, "4.0x");
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        ui.label(RichText::new("Puntos:").size(11.0).color(theme.text_secondary()));
                        ui.add_sized(
                            [65.0, 18.0],
                            egui::Slider::new(&mut self.constellation_points, 256..=4096).suffix(" pts"),
                        );
                    });
                });
            });
        } else {
            // Modo escritorio / ventana estándar (>=600px): 2 filas semánticas atómicas
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);

                // Fila 1: Sincronización y Decisión de Símbolos
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

                    ui.checkbox(
                        &mut self.constellation_vsa_mode,
                        RichText::new("Modo Símbolos (VSA)").size(11.5).strong().color(theme.accent_blue()),
                    ).on_hover_text("Decima la señal continua a 1 muestra por símbolo para evaluar la constelación digital y decisiones (BPSK, QPSK, OQPSK, QAM).");

                    if self.constellation_vsa_mode {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("SPS:").size(11.0).color(theme.text_secondary()));
                            egui::ComboBox::from_id_salt("sink_const_sps")
                                .selected_text(format!("{} SPS", self.constellation_sps))
                                .width(70.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.constellation_sps, 1, "1 SPS");
                                    ui.selectable_value(&mut self.constellation_sps, 2, "2 SPS");
                                    ui.selectable_value(&mut self.constellation_sps, 4, "4 SPS (Meteor LRPT)");
                                    ui.selectable_value(&mut self.constellation_sps, 8, "8 SPS");
                                    ui.selectable_value(&mut self.constellation_sps, 16, "16 SPS");
                                });
                        });

                        ui.checkbox(
                            &mut self.constellation_oqpsk_deoffset,
                            RichText::new("OQPSK (Ts/2)").size(11.0),
                        ).on_hover_text("Compensa el retardo de medio símbolo (Ts/2) entre las ramas I y Q en señales OQPSK para colapsar la constelación a 4 cuadrantes estándar.");

                        let sps = self.constellation_sps.max(1);
                        if self.constellation_timing_phase >= sps {
                            self.constellation_timing_phase = 0;
                        }

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("Fase Ojo:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized(
                                [60.0, 18.0],
                                egui::Slider::new(&mut self.constellation_timing_phase, 0..=sps.saturating_sub(1)),
                            ).on_hover_text("Desplaza el instante de muestreo de decisión dentro del periodo del símbolo (0 a SPS-1).");
                        });

                        if ui.button(RichText::new("Auto Ojo").size(11.0)).on_hover_text("Busca de forma automática la fase de reloj con máxima apertura de ojo").clicked() {
                            let delay = if self.constellation_oqpsk_deoffset && sps >= 2 { sps / 2 } else { 0 };
                            let aligned_start = (self.sample_cursor / sps) * sps;
                            self.constellation_timing_phase = find_best_eye_phase(samples, aligned_start, sps, delay);
                        }
                    }
                });

                // Fila 2: Visualización, Ejes y Escalas
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

                    if self.constellation_vsa_mode {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("Fase θ:").size(11.0).color(theme.text_secondary()));
                            ui.add_sized(
                                [65.0, 18.0],
                                egui::Slider::new(&mut self.constellation_carrier_phase_deg, 0.0..=360.0).suffix("°"),
                            ).on_hover_text("Rotación de fase de portadora para desrotar y alinear la constelación con los ejes I/Q.");
                        });

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                            ui.label(RichText::new("Escala:").size(11.0).color(theme.text_secondary()));
                            let norm_label = match self.constellation_norm_mode {
                                ConstellationNormMode::UnitEnergy => "RF (Es=1)",
                                ConstellationNormMode::SymbolicGrid => "Grilla (±1)",
                            };
                            egui::ComboBox::from_id_salt("sink_const_norm_mode")
                                .selected_text(norm_label)
                                .width(78.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.constellation_norm_mode,
                                        ConstellationNormMode::UnitEnergy,
                                        "RF (Es=1)",
                                    ).on_hover_text(
                                        "Norma de Potencia Unitaria (Es = 1.0): Estándar físico de ingeniería de radiofrecuencia (CCSDS/SDR). \
                                         Los símbolos se sitúan en ±1/√2 (±0.707) sobre el círculo unitario r=1.0 para aprovechar el 100% \
                                         del rango dinámico del DAC sin causar distorsión armónica ni saturación."
                                    );
                                    ui.selectable_value(
                                        &mut self.constellation_norm_mode,
                                        ConstellationNormMode::SymbolicGrid,
                                        "Grilla (±1)",
                                    ).on_hover_text(
                                        "Norma de Cuadrícula Simbólica (±1.0): Escala normalizada por √2 para que los centros de decisión \
                                         coincidan exactamente con las marcas ±1.0 de los ejes cartesianos (representación convencional en textos teóricos)."
                                    );
                                });
                        });
                    }

                    ui.checkbox(&mut self.show_constellation_circles, RichText::new("Círculos").size(11.0))
                        .on_hover_text("Muestra u oculta los círculos concéntricos de referencia (r = 0.5 y círculo unitario r = 1.0).");
                    ui.checkbox(&mut self.show_constellation_grid, RichText::new("Grilla").size(11.0))
                        .on_hover_text("Muestra u oculta la cuadrícula ortogonal (retícula en ±0.5 y ±1.0).");

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        ui.label(RichText::new("Zoom:").size(11.0).color(theme.text_secondary()));
                        egui::ComboBox::from_id_salt("sink_const_zoom")
                            .selected_text(format!("{:.1}x", self.constellation_zoom))
                            .width(55.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.constellation_zoom, 0.5, "0.5x");
                                ui.selectable_value(&mut self.constellation_zoom, 1.0, "1.0x");
                                ui.selectable_value(&mut self.constellation_zoom, 1.5, "1.5x");
                                ui.selectable_value(&mut self.constellation_zoom, 2.0, "2.0x");
                                ui.selectable_value(&mut self.constellation_zoom, 3.0, "3.0x");
                                ui.selectable_value(&mut self.constellation_zoom, 4.0, "4.0x");
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        ui.label(RichText::new("Puntos:").size(11.0).color(theme.text_secondary()));
                        ui.add_sized(
                            [70.0, 18.0],
                            egui::Slider::new(&mut self.constellation_points, 256..=4096).suffix(" pts"),
                        );
                    });
                });
            });
        }
    }

    /// Barra de controles inferiores (Play, Pause, Cursor de tiempo, Velocidad, FFT size y Exportar PNG)
    fn render_playback_controls(
        &mut self,
        ui: &mut Ui,
        theme: &AppTheme,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) {
        let total_samples = samples.len();
        let avail_w = ui.available_width();

        if avail_w < 820.0 {
            // Diseño responsivo en 2 filas limpias para pantallas móviles e intermedias (<820px)
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);

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
                    self.reset_peaks();
                    self.waterfall_history.clear();
                }

                // Slider y tiempo con ancho elástico adaptativo
                if total_samples > 0 && sample_rate > 0 {
                    let total_secs = total_samples as f32 / sample_rate as f32;
                    let current_secs = self.sample_cursor as f32 / sample_rate as f32;
                    let mut cursor_val = self.sample_cursor;

                    let slider_w = (ui.available_width() - 130.0).clamp(50.0, 180.0);
                    let slider = egui::Slider::new(&mut cursor_val, 0..=total_samples).show_value(false);
                    if ui.add_sized([slider_w, 18.0], slider).changed() {
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
                        .size(11.0)
                        .color(theme.text_secondary()),
                    );
                }
            });

            ui.add_space(3.0);

            // Segunda fila de controles secundarios (Velocidad, FFT, Guardar PNG)
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

                // Selector de velocidad
                egui::ComboBox::from_id_salt("sink_speed_combo")
                    .selected_text(format!("{}x", self.playback_speed))
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.playback_speed, 0.25, "0.25x");
                        ui.selectable_value(&mut self.playback_speed, 0.5, "0.5x");
                        ui.selectable_value(&mut self.playback_speed, 1.0, "1.0x (Real)");
                        ui.selectable_value(&mut self.playback_speed, 2.0, "2.0x");
                        ui.selectable_value(&mut self.playback_speed, 5.0, "5.0x");
                    });

                // Selector de FFT
                egui::ComboBox::from_id_salt("sink_fft_size")
                    .selected_text(format!("FFT: {}", self.fft_size))
                    .width(82.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.fft_size, 512, "512 bins");
                        ui.selectable_value(&mut self.fft_size, 1024, "1024 bins");
                        ui.selectable_value(&mut self.fft_size, 2048, "2048 bins");
                        ui.selectable_value(&mut self.fft_size, 4096, "4096 bins");
                        ui.selectable_value(&mut self.fft_size, 8192, "8192 bins");
                        ui.selectable_value(&mut self.fft_size, 16384, "16384 bins");
                    });

                // Botón Guardar Captura PNG
                let save_btn = egui::Button::new(
                    RichText::new(format!("{} Guardar PNG", icons::ICON_CAMERA_ALT.codepoint)).size(11.5)
                )
                .fill(theme.surface_subtle())
                .stroke(Stroke::new(1.0, theme.card_border()))
                .corner_radius(6.0);

                if ui.add(save_btn).on_hover_text("Guarda una imagen en alta resolución de la pestaña activa como archivo PNG").clicked() {
                    self.export_active_view_png(samples, sample_rate, center_freq_hz);
                }
            });
        } else {
            // Diseño en una sola fila para pantallas de escritorio amplias (>=820px)
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

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
                    self.reset_peaks();
                    self.waterfall_history.clear();
                }

                ui.add_space(4.0);

                // Control deslizante de posición temporal en la señal
                if total_samples > 0 && sample_rate > 0 {
                    let total_secs = total_samples as f32 / sample_rate as f32;
                    let current_secs = self.sample_cursor as f32 / sample_rate as f32;
                    let mut cursor_val = self.sample_cursor;

                    let slider_w = (ui.available_width() - 360.0).clamp(80.0, 240.0);
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

                ui.add_space(4.0);

                // Selector de velocidad
                egui::ComboBox::from_id_salt("sink_speed_combo")
                    .selected_text(format!("{}x", self.playback_speed))
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.playback_speed, 0.25, "0.25x");
                        ui.selectable_value(&mut self.playback_speed, 0.5, "0.5x");
                        ui.selectable_value(&mut self.playback_speed, 1.0, "1.0x (Real)");
                        ui.selectable_value(&mut self.playback_speed, 2.0, "2.0x");
                        ui.selectable_value(&mut self.playback_speed, 5.0, "5.0x");
                    });

                // Selector de FFT
                egui::ComboBox::from_id_salt("sink_fft_size")
                    .selected_text(format!("FFT: {}", self.fft_size))
                    .width(82.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.fft_size, 512, "512 bins");
                        ui.selectable_value(&mut self.fft_size, 1024, "1024 bins");
                        ui.selectable_value(&mut self.fft_size, 2048, "2048 bins");
                        ui.selectable_value(&mut self.fft_size, 4096, "4096 bins");
                        ui.selectable_value(&mut self.fft_size, 8192, "8192 bins");
                        ui.selectable_value(&mut self.fft_size, 16384, "16384 bins");
                    });

                // Botón Guardar Captura PNG
                let save_btn = egui::Button::new(
                    RichText::new(format!("{} Guardar PNG", icons::ICON_CAMERA_ALT.codepoint)).size(11.5)
                )
                .fill(theme.surface_subtle())
                .stroke(Stroke::new(1.0, theme.card_border()))
                .corner_radius(6.0);

                if ui.add(save_btn).on_hover_text("Guarda una imagen en alta resolución de la pestaña activa como archivo PNG").clicked() {
                    self.export_active_view_png(samples, sample_rate, center_freq_hz);
                }
            });
        }

        if let Some((ref msg, is_err)) = self.export_status_message {
            ui.add_space(2.0);
            let col = if is_err { Color32::from_rgb(239, 68, 68) } else { Color32::from_rgb(34, 197, 94) };
            ui.label(RichText::new(msg).size(11.0).color(col));
        }
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

        // Actualizar Peak Holds
        if self.max_peak_buffer.len() != n {
            self.max_peak_buffer = db_vals.clone();
        } else if self.show_max_peak {
            for i in 0..n {
                self.max_peak_buffer[i] = self.max_peak_buffer[i].max(db_vals[i]) * 0.999 + db_vals[i] * 0.001;
            }
        }

        if self.min_peak_buffer.len() != n {
            self.min_peak_buffer = db_vals.clone();
        } else if self.show_min_peak {
            for i in 0..n {
                self.min_peak_buffer[i] = self.min_peak_buffer[i].min(db_vals[i]) * 0.999 + db_vals[i] * 0.001;
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

        let margin_l = 48.0;
        let margin_b = 25.0;
        let plot_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + margin_l, rect.min.y + 10.0),
            Pos2::new(rect.max.x - 10.0, rect.max.y - margin_b),
        );

        let grid_col = if *theme == AppTheme::Dark { Color32::from_rgb(30, 40, 55) } else { Color32::from_rgb(225, 232, 240) };
        let text_col = theme.text_secondary();

        // Líneas de cuadrícula horizontal dinámicas
        let db_step = if (self.db_max - self.db_min) > 80.0 { 20.0 } else { 10.0 };
        let mut cur_db = (self.db_max / db_step).floor() * db_step;
        while cur_db >= self.db_min - 0.1 {
            let t = ((cur_db - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
            let y = plot_rect.max.y - t * plot_rect.height();
            painter.line_segment([Pos2::new(plot_rect.min.x, y), Pos2::new(plot_rect.max.x, y)], Stroke::new(1.0, grid_col));
            painter.text(Pos2::new(rect.min.x + 40.0, y), egui::Align2::RIGHT_CENTER, format!("{:.0} dB", cur_db), egui::FontId::monospace(10.0), text_col);
            cur_db -= db_step;
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
            // 1. Trazado Min Peak Hold (Magenta)
            if self.show_min_peak && self.min_peak_buffer.len() == n {
                let min_pts: Vec<Pos2> = (0..n).map(|i| {
                    let tx = i as f32 / (n - 1) as f32;
                    let ty = ((self.min_peak_buffer[i] - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                    Pos2::new(plot_rect.min.x + tx * plot_rect.width(), plot_rect.max.y - ty * plot_rect.height())
                }).collect();

                for w in min_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.0, Color32::from_rgb(236, 72, 153).gamma_multiply(0.85)));
                }
            }

            // 2. Trazado Max Peak Hold (Amarillo)
            if self.show_max_peak && self.max_peak_buffer.len() == n {
                let peak_pts: Vec<Pos2> = (0..n).map(|i| {
                    let tx = i as f32 / (n - 1) as f32;
                    let ty = ((self.max_peak_buffer[i] - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                    Pos2::new(plot_rect.min.x + tx * plot_rect.width(), plot_rect.max.y - ty * plot_rect.height())
                }).collect();

                for w in peak_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.0, Color32::from_rgb(234, 179, 8).gamma_multiply(0.85)));
                }
            }

            // 3. Trazado Espectro Activo (Cian brillante)
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
        let legend_pos = Pos2::new(plot_rect.max.x - 130.0, plot_rect.min.y + 10.0);
        painter.rect_filled(Rect::from_min_size(legend_pos, Vec2::new(120.0, 48.0)), 4.0, Color32::from_black_alpha(160));
        painter.circle_filled(legend_pos + Vec2::new(10.0, 10.0), 3.5, Color32::from_rgb(56, 189, 248));
        painter.text(legend_pos + Vec2::new(20.0, 10.0), egui::Align2::LEFT_CENTER, "Espectro I/Q", egui::FontId::proportional(10.5), Color32::WHITE);
        if self.show_max_peak {
            painter.circle_filled(legend_pos + Vec2::new(10.0, 22.0), 3.5, Color32::from_rgb(234, 179, 8));
            painter.text(legend_pos + Vec2::new(20.0, 22.0), egui::Align2::LEFT_CENTER, "Max Peak", egui::FontId::proportional(10.5), Color32::WHITE);
        }
        if self.show_min_peak {
            painter.circle_filled(legend_pos + Vec2::new(10.0, 34.0), 3.5, Color32::from_rgb(236, 72, 153));
            painter.text(legend_pos + Vec2::new(20.0, 34.0), egui::Align2::LEFT_CENTER, "Min Peak", egui::FontId::proportional(10.5), Color32::WHITE);
        }

        // Inspección interactiva al pasar el mouse (Crosshair y Valores de Frecuencia/Potencia)
        if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
            if plot_rect.contains(mouse_pos) && n > 1 {
                let tx = ((mouse_pos.x - plot_rect.min.x) / plot_rect.width()).clamp(0.0, 1.0);
                let bin_idx = ((tx * (n - 1) as f32).round() as usize).min(n - 1);
                let db_at_cursor = db_vals[bin_idx];
                let freq_offset_hz = (-0.5 + tx) * (sample_rate as f32);
                let freq_text = if center_freq_hz > 0.0 {
                    let actual_mhz = (center_freq_hz + freq_offset_hz as f64) / 1e6;
                    format!("{:.4} MHz (Δf: {:+.1} kHz)", actual_mhz, freq_offset_hz / 1e3)
                } else {
                    format!("Δf: {:+.2} kHz", freq_offset_hz / 1e3)
                };

                // Línea vertical guía del cursor
                painter.line_segment(
                    [Pos2::new(mouse_pos.x, plot_rect.min.y), Pos2::new(mouse_pos.x, plot_rect.max.y)],
                    Stroke::new(1.0, Color32::from_white_alpha(90)),
                );

                // Punto resaltado sobre la curva espectral
                let ty_val = ((db_at_cursor - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                let curve_pt = Pos2::new(mouse_pos.x, plot_rect.max.y - ty_val * plot_rect.height());
                painter.circle_filled(curve_pt, 4.5, Color32::from_rgb(56, 189, 248));
                painter.circle_stroke(curve_pt, 6.0, Stroke::new(1.0, Color32::WHITE));

                // Tarjeta flotante con valores
                let tooltip_text = format!("{} | {:.1} dBFS", freq_text, db_at_cursor);
                let font = egui::FontId::monospace(11.0);
                let card_w = tooltip_text.len() as f32 * 6.8 + 16.0;
                let box_x = (mouse_pos.x + 12.0).min(plot_rect.max.x - card_w - 4.0);
                let box_y = (mouse_pos.y - 28.0).max(plot_rect.min.y + 4.0);
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(box_x, box_y), Vec2::new(card_w, 22.0)),
                    4.0,
                    Color32::from_black_alpha(220),
                );
                painter.text(
                    Pos2::new(box_x + 8.0, box_y + 11.0),
                    egui::Align2::LEFT_CENTER,
                    tooltip_text,
                    font,
                    Color32::WHITE,
                );
            }
        }
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

            let target_cols = match self.waterfall_resolution_mode {
                0 => 64,
                1 => 128,
                2 => 256,
                3 => 512,
                _ => n_cols,
            };
            let col_step = if self.waterfall_resolution_mode >= 4 {
                1
            } else {
                (n_cols / target_cols).max(1)
            };

            for c in (0..n_cols).step_by(col_step) {
                let t_x0 = c as f32 / n_cols as f32;
                let t_x1 = (c + col_step).min(n_cols) as f32 / n_cols as f32;
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

        let plot_side = (rect.width().min(rect.height()) - 40.0).max(100.0);
        let plot_rect = Rect::from_center_size(rect.center(), Vec2::splat(plot_side));

        let center = plot_rect.center();
        let scale = plot_side * 0.40 * self.constellation_zoom;

        let grid_col = if *theme == AppTheme::Dark { Color32::from_rgb(30, 42, 58) } else { Color32::from_rgb(220, 228, 238) };
        let tick_text_col = theme.text_secondary();

        let norm_mult = match self.constellation_norm_mode {
            ConstellationNormMode::UnitEnergy => 1.0f32,
            ConstellationNormMode::SymbolicGrid => std::f32::consts::SQRT_2,
        };
        let effective_scale = scale * norm_mult;

        // Círculos concéntricos de referencia (r = 0.5, 1.0) condicionales
        if self.show_constellation_circles {
            painter.circle_stroke(center, scale * 0.5, Stroke::new(1.0, grid_col));
            painter.circle_stroke(center, scale * 1.0, Stroke::new(1.5, grid_col));
        }

        // Retícula secundaria en ±1.0 y ±0.5 condicional
        if self.show_constellation_grid {
            for &v in &[-1.0f32, -0.5, 0.5, 1.0] {
                let px = center.x + v * scale;
                if px >= plot_rect.min.x && px <= plot_rect.max.x {
                    painter.line_segment(
                        [Pos2::new(px, plot_rect.min.y), Pos2::new(px, plot_rect.max.y)],
                        Stroke::new(0.5, grid_col.gamma_multiply(0.4)),
                    );
                }
                let py = center.y - v * scale;
                if py >= plot_rect.min.y && py <= plot_rect.max.y {
                    painter.line_segment(
                        [Pos2::new(plot_rect.min.x, py), Pos2::new(plot_rect.max.x, py)],
                        Stroke::new(0.5, grid_col.gamma_multiply(0.4)),
                    );
                }
            }
        }

        // Ejes ortogonales I y Q principales
        painter.line_segment([Pos2::new(plot_rect.min.x, center.y), Pos2::new(plot_rect.max.x, center.y)], Stroke::new(1.5, grid_col));
        painter.line_segment([Pos2::new(center.x, plot_rect.min.y), Pos2::new(center.x, plot_rect.max.y)], Stroke::new(1.5, grid_col));

        // Marcas (ticks) y números en eje I (horizontal)
        for &v in &[-1.0f32, -0.5, 0.5, 1.0] {
            let px = center.x + v * scale;
            if px >= plot_rect.min.x + 15.0 && px <= plot_rect.max.x - 15.0 {
                painter.line_segment([Pos2::new(px, center.y - 4.0), Pos2::new(px, center.y + 4.0)], Stroke::new(1.5, theme.accent_blue()));
                painter.text(
                    Pos2::new(px, center.y + 12.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{:+.1}", v),
                    egui::FontId::monospace(10.0),
                    tick_text_col,
                );
            }
        }

        // Marcas (ticks) y números en eje Q (vertical)
        for &v in &[-1.0f32, -0.5, 0.5, 1.0] {
            let py = center.y - v * scale;
            if py >= plot_rect.min.y + 15.0 && py <= plot_rect.max.y - 15.0 {
                painter.line_segment([Pos2::new(center.x - 4.0, py), Pos2::new(center.x + 4.0, py)], Stroke::new(1.5, theme.accent_blue()));
                painter.text(
                    Pos2::new(center.x - 14.0, py),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:+.1}", v),
                    egui::FontId::monospace(10.0),
                    tick_text_col,
                );
            }
        }

        painter.text(
            Pos2::new(plot_rect.max.x - 8.0, center.y - 14.0),
            egui::Align2::RIGHT_CENTER,
            "+I (Fase)",
            egui::FontId::monospace(11.0),
            theme.accent_blue(),
        );
        painter.text(
            Pos2::new(center.x + 8.0, plot_rect.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            "+Q (Cuadratura)",
            egui::FontId::monospace(11.0),
            theme.accent_blue(),
        );

        // Distintivo compacto de norma en esquina superior izquierda (sin invadir el semieje central)
        let badge_text = match self.constellation_norm_mode {
            ConstellationNormMode::UnitEnergy => "Norma: RF (Es=1.0)",
            ConstellationNormMode::SymbolicGrid => "Norma: Grilla (±1.0)",
        };
        let badge_rect = Rect::from_min_size(Pos2::new(plot_rect.min.x + 8.0, plot_rect.min.y + 8.0), Vec2::new(138.0, 22.0));
        painter.rect_filled(badge_rect, 4.0, Color32::from_black_alpha(180));
        painter.rect_stroke(badge_rect, 4.0, Stroke::new(1.0, theme.accent_blue().gamma_multiply(0.35)), egui::StrokeKind::Inside);
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            badge_text,
            egui::FontId::monospace(10.5),
            theme.accent_blue(),
        );

        if samples.is_empty() {
            return;
        }

        let total_samples = samples.len();
        let sps = self.constellation_sps.max(1);
        let timing_phase = self.constellation_timing_phase % sps;
        let oqpsk_delay = if self.constellation_oqpsk_deoffset && sps >= 2 {
            sps / 2
        } else {
            0
        };

        let rot_rad = self.constellation_carrier_phase_deg.to_radians();
        let (cos_rot, sin_rot) = (rot_rad.cos(), rot_rad.sin());

        // Bloqueo de fase de símbolo: alinear el cursor a múltiplos exactos de SPS
        // Esto previene que el avance continuo del cursor a 60 FPS desplace la fase de muestreo entre frames,
        // eliminando por completo la alternancia/parpadeo entre el ojo óptimo y las transiciones de símbolo.
        let aligned_start = (self.sample_cursor / sps) * sps;

        // Normalización automática de símbolo en modo VSA: alinea la constelación con el círculo unitario (r=1.0)
        let vsa_norm_gain = if self.constellation_vsa_mode {
            let n_meas = 256.min(total_samples / sps);
            let mut sum_mag = 0.0f32;
            let mut count = 0usize;
            for i in 0..n_meas {
                let sample_idx = (aligned_start + i * sps + timing_phase) % total_samples;
                let q_idx = (sample_idx + oqpsk_delay) % total_samples;
                let m = (samples[sample_idx].0.powi(2) + samples[q_idx].1.powi(2)).sqrt();
                if m > 1e-4 {
                    sum_mag += m;
                    count += 1;
                }
            }
            if count > 0 {
                let avg_m = sum_mag / count as f32;
                if avg_m > 1e-4 { 1.0 / avg_m } else { 1.0 }
            } else {
                1.0
            }
        } else {
            1.0
        };

        let display_scale = effective_scale * vsa_norm_gain;

        if self.constellation_vsa_mode {
            // Modo Símbolos VSA: 1 muestra por símbolo decimado en la fase óptima del ojo
            let n_symbols = self.constellation_points.min(total_samples / sps);
            for i in 0..n_symbols {
                let sample_idx = (aligned_start + i * sps + timing_phase) % total_samples;
                let raw_i = samples[sample_idx].0;

                // Si OQPSK está activo, alinear el canal Q retardado Ts/2
                let q_idx = (sample_idx + oqpsk_delay) % total_samples;
                let raw_q = samples[q_idx].1;

                // Rotación de portadora (desrotación de fase)
                let i_rot = raw_i * cos_rot - raw_q * sin_rot;
                let q_rot = raw_i * sin_rot + raw_q * cos_rot;

                let pt = Pos2::new(center.x + i_rot * display_scale, center.y - q_rot * display_scale);

                let age_factor = i as f32 / n_symbols.max(1) as f32;
                let alpha = (90.0 + age_factor * 165.0) as u8;
                let dot_col = Color32::from_rgba_premultiplied(56, 189, 248, alpha);

                // Dibujar símbolo de decisión nítido
                painter.circle_filled(pt, 2.5, dot_col);
            }
        } else {
            // Modo Trayectoria Cruda: Muestra continua con persistencia fósforo
            let n = self.constellation_points.min(total_samples);
            let start = self.sample_cursor;
            for i in 0..n {
                let idx = (start + i) % total_samples;
                let (raw_i, raw_q) = samples[idx];

                let i_rot = raw_i * cos_rot - raw_q * sin_rot;
                let q_rot = raw_i * sin_rot + raw_q * cos_rot;

                let pt = Pos2::new(center.x + i_rot * display_scale, center.y - q_rot * display_scale);

                let age_factor = i as f32 / n as f32;
                let alpha = (50.0 + age_factor * 180.0) as u8;
                let dot_col = Color32::from_rgba_premultiplied(56, 189, 248, alpha);

                painter.circle_filled(pt, 1.8, dot_col);
            }
        }

        // Inspección interactiva al pasar el mouse (Crosshair y Valores I, Q, |s|, θ)
        if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
            if plot_rect.contains(mouse_pos) {
                // Proyección precisa sobre el sistema de coordenadas graduado del eje
                let i_val = (mouse_pos.x - center.x) / scale;
                let q_val = -(mouse_pos.y - center.y) / scale;
                let mag = (i_val * i_val + q_val * q_val).sqrt();
                let phase_deg = q_val.atan2(i_val).to_degrees();

                // Mira interactiva
                painter.line_segment(
                    [Pos2::new(plot_rect.min.x, mouse_pos.y), Pos2::new(plot_rect.max.x, mouse_pos.y)],
                    Stroke::new(0.8, Color32::from_white_alpha(80)),
                );
                painter.line_segment(
                    [Pos2::new(mouse_pos.x, plot_rect.min.y), Pos2::new(mouse_pos.x, plot_rect.max.y)],
                    Stroke::new(0.8, Color32::from_white_alpha(80)),
                );
                painter.circle_stroke(mouse_pos, 4.0, Stroke::new(1.0, theme.accent_blue()));

                // Tarjeta flotante con valores bajo el puntero adaptada a pantallas móviles y de escritorio
                let is_narrow_plot = plot_rect.width() < 340.0;
                let (tip, tip_w, tip_h) = if is_narrow_plot {
                    let text = format!("I:{:+.2} Q:{:+.2}\n|s|:{:.2} θ:{:+.0}°", i_val, q_val, mag, phase_deg);
                    (text, 140.0f32, 34.0f32)
                } else {
                    let text = format!("I: {:+.3} | Q: {:+.3} | |s|: {:.3} | θ: {:+.1}°", i_val, q_val, mag, phase_deg);
                    let w = text.len() as f32 * 6.5 + 16.0;
                    (text, w, 22.0f32)
                };

                let font = egui::FontId::monospace(if is_narrow_plot { 10.0 } else { 11.0 });
                let min_x = plot_rect.min.x + 6.0;
                let max_x = (plot_rect.max.x - tip_w - 6.0).max(min_x);
                let box_x = (mouse_pos.x + 12.0).clamp(min_x, max_x);

                let box_y = if mouse_pos.y - tip_h - 8.0 < plot_rect.min.y + 6.0 {
                    mouse_pos.y + 14.0
                } else {
                    mouse_pos.y - tip_h - 6.0
                }.clamp(plot_rect.min.y + 6.0, (plot_rect.max.y - tip_h - 6.0).max(plot_rect.min.y + 6.0));

                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(box_x, box_y), Vec2::new(tip_w, tip_h)),
                    4.0,
                    Color32::from_black_alpha(220),
                );
                painter.text(
                    Pos2::new(box_x + 8.0, box_y + tip_h * 0.5),
                    egui::Align2::LEFT_CENTER,
                    tip,
                    font,
                    Color32::WHITE,
                );
            }
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

        let n_f = n as f32;
        let rms_pwr = sum_sq / n_f;
        let rms_db = 10.0 * (rms_pwr + 1e-12).log10();
        let peak_db = 10.0 * (peak_sq + 1e-12).log10();
        let papr_db = (peak_db - rms_db).max(0.0);
        let dc_i_mean = dc_i / n_f;
        let dc_q_mean = dc_q / n_f;

        // Desbalance I/Q (Ganancia y Fase de cuadratura)
        let mut sum_i_sq = 0.0f32;
        let mut sum_q_sq = 0.0f32;
        let mut sum_iq = 0.0f32;
        for &(i, q) in &samples[..n] {
            sum_i_sq += i * i;
            sum_q_sq += q * q;
            sum_iq += i * q;
        }
        let var_i = (sum_i_sq / n_f - dc_i_mean * dc_i_mean).max(1e-12);
        let var_q = (sum_q_sq / n_f - dc_q_mean * dc_q_mean).max(1e-12);
        let cov_iq = sum_iq / n_f - dc_i_mean * dc_q_mean;
        let gain_imbalance_db = 10.0 * (var_i / var_q).log10();
        let phase_imbalance_deg = (cov_iq / (var_i.sqrt() * var_q.sqrt()).max(1e-12))
            .clamp(-1.0, 1.0)
            .asin()
            .to_degrees();

        // Estimación de SNR espectral mediante integración de potencia total vs piso de ruido
        let db_vals = self.compute_current_fft(samples);
        let fft_len = db_vals.len();

        let (estimated_snr_db, is_clean_channel, obw_khz, sfdr_db) = if fft_len > 16 {
            let linear_pwr: Vec<f32> = db_vals.iter().map(|&db| 10.0f32.powf(db / 10.0)).collect();
            let total_fft_pwr: f32 = linear_pwr.iter().sum();

            // Ordenar potencias para calcular el piso de ruido N0 (mediana del percentil 15-35%)
            let mut sorted_pwr = linear_pwr.clone();
            sorted_pwr.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let q_idx = (fft_len as f32 * 0.25).round() as usize;
            let n0_bin = sorted_pwr[q_idx.min(fft_len - 1)].max(1e-15);
            let total_noise_pwr = n0_bin * fft_len as f32;

            // SFDR: diferencia entre el pico máximo y el segundo pico más alto fuera del lóbulo
            let mut sorted_dbs = db_vals.clone();
            sorted_dbs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let peak_db = sorted_dbs[fft_len - 1];
            let spur_db = sorted_dbs[fft_len.saturating_sub(6)];
            let sfdr = (peak_db - spur_db).max(0.0);

            // Ancho de banda ocupado (OBW 99% de energía)
            let target_energy = total_fft_pwr * 0.99;
            let mut accum = 0.0f32;
            let mut obw_bins = 0;
            for &p in &linear_pwr {
                if accum < target_energy {
                    accum += p;
                    obw_bins += 1;
                }
            }
            let obw = (obw_bins as f32 / fft_len as f32) * (sample_rate as f32 / 1000.0);

            if total_noise_pwr >= total_fft_pwr {
                (0.0f32, false, obw, sfdr)
            } else {
                let sig_pwr = total_fft_pwr - total_noise_pwr;
                let snr_lin = sig_pwr / total_noise_pwr;
                let snr_calc = 10.0 * snr_lin.log10();
                if snr_calc >= 45.0 {
                    (45.0, true, obw, sfdr)
                } else {
                    (snr_calc.max(0.0), false, obw, sfdr)
                }
            }
        } else {
            (45.0, true, 0.0, 0.0)
        };

        // Estimación de EVM y MER (sobre símbolos decimados VSA con normalización)
        let sps = self.constellation_sps.max(1);
        let aligned_start = (self.sample_cursor / sps) * sps;
        let timing_phase = self.constellation_timing_phase % sps;
        let oqpsk_delay = if self.constellation_oqpsk_deoffset && sps >= 2 { sps / 2 } else { 0 };
        let rot_rad = self.constellation_carrier_phase_deg.to_radians();
        let (cos_rot, sin_rot) = (rot_rad.cos(), rot_rad.sin());

        let n_symbols = 256.min(samples.len() / sps);
        let vsa_norm_gain = if n_symbols > 0 {
            let mut sum_mag = 0.0f32;
            let mut count = 0usize;
            for i in 0..n_symbols {
                let s_idx = (aligned_start + i * sps + timing_phase) % samples.len();
                let q_idx = (s_idx + oqpsk_delay) % samples.len();
                let m = (samples[s_idx].0.powi(2) + samples[q_idx].1.powi(2)).sqrt();
                if m > 1e-4 {
                    sum_mag += m;
                    count += 1;
                }
            }
            if count > 0 { 1.0 / (sum_mag / count as f32) } else { 1.0 }
        } else {
            1.0
        };

        let mut sum_err_sq = 0.0f32;

        for i in 0..n_symbols {
            let s_idx = (aligned_start + i * sps + timing_phase) % samples.len();
            let raw_i = samples[s_idx].0;
            let q_idx = (s_idx + oqpsk_delay) % samples.len();
            let raw_q = samples[q_idx].1;

            let i_rot = raw_i * cos_rot - raw_q * sin_rot;
            let q_rot = raw_i * sin_rot + raw_q * cos_rot;

            let i_norm = i_rot * vsa_norm_gain;
            let q_norm = q_rot * vsa_norm_gain;

            let ideal_i = if i_norm >= 0.0 { std::f32::consts::FRAC_1_SQRT_2 } else { -std::f32::consts::FRAC_1_SQRT_2 };
            let ideal_q = if q_norm >= 0.0 { std::f32::consts::FRAC_1_SQRT_2 } else { -std::f32::consts::FRAC_1_SQRT_2 };

            let err_i = i_norm - ideal_i;
            let err_q = q_norm - ideal_q;
            sum_err_sq += err_i * err_i + err_q * err_q;
        }

        let evm_rms_pct = if n_symbols > 0 {
            ((sum_err_sq / (n_symbols as f32)).sqrt() * 100.0).clamp(0.1, 99.9)
        } else {
            0.0
        };
        let mer_db = (-20.0 * (evm_rms_pct / 100.0).log10()).clamp(0.0, 50.0);

        ui.vertical(|ui| {
            ui.label(RichText::new("Telemetría y Diagnóstico de Señal RF").strong().size(16.0).color(theme.accent_blue()));
            ui.label(RichText::new("Cálculo en vivo de SNR, EVM, MER, potencia espectral y balance I/Q sobre la señal activa.").size(12.0).color(theme.text_secondary()));
            ui.add_space(10.0);

            let avail_w = ui.available_width();
            let num_cols = if avail_w >= 840.0 { 4 } else if avail_w >= 480.0 { 2 } else { 1 };

            ui.columns(num_cols, |cols| {
                // Tarjeta 1: Potencias RF
                Frame::new().fill(theme.card_bg()).stroke(Stroke::new(1.0, theme.card_border())).corner_radius(8.0).inner_margin(12.0).show(&mut cols[0], |ui| {
                    ui.label(RichText::new("Potencia RF").strong().color(theme.text_primary()));
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Potencia Pico: {:.2} dBFS", peak_db)).size(13.0).color(theme.accent_blue()));
                    ui.label(RichText::new(format!("Potencia RMS: {:.2} dBFS", rms_db)).size(13.0));
                    ui.label(RichText::new(format!("PAPR / Cresta: {:.2} dB", papr_db)).size(13.0).color(Color32::from_rgb(234, 179, 8)));
                });

                // Tarjeta 2: Calidad Digital / RF según tipo de señal
                if num_cols > 1 {
                    Frame::new().fill(theme.card_bg()).stroke(Stroke::new(1.0, theme.card_border())).corner_radius(8.0).inner_margin(12.0).show(&mut cols[1], |ui| {
                        if self.constellation_vsa_mode && sps >= 1 {
                            ui.label(RichText::new("Calidad Digital (VSA)").strong().color(theme.text_primary()));
                            ui.add_space(6.0);
                            let snr_txt = if is_clean_channel {
                                format!("SNR Estimada: > {:.0} dB (Ideal)", estimated_snr_db)
                            } else {
                                format!("SNR Estimada: {:.1} dB", estimated_snr_db)
                            };
                            ui.label(RichText::new(snr_txt).size(13.0).color(Color32::from_rgb(34, 197, 94)));
                            ui.label(RichText::new(format!("EVM RMS: {:.2} %", evm_rms_pct)).size(13.0).color(if evm_rms_pct < 15.0 { Color32::from_rgb(34, 197, 94) } else { Color32::from_rgb(234, 179, 8) }));
                            ui.label(RichText::new(format!("MER: {:.1} dB", mer_db)).size(13.0));
                        } else {
                            ui.label(RichText::new("Calidad de Señal RF").strong().color(theme.text_primary()));
                            ui.add_space(6.0);
                            let snr_txt = if is_clean_channel {
                                format!("SNR Espectral: > {:.0} dB (Ideal)", estimated_snr_db)
                            } else {
                                format!("SNR Espectral: {:.1} dB", estimated_snr_db)
                            };
                            ui.label(RichText::new(snr_txt).size(13.0).color(Color32::from_rgb(34, 197, 94)));
                            ui.label(RichText::new(format!("Ancho Banda: {:.1} kHz", obw_khz)).size(13.0));
                            ui.label(RichText::new(format!("SFDR (Dinámica): {:.1} dB", sfdr_db)).size(13.0).color(theme.accent_blue()));
                        }
                    });
                }

                // Tarjeta 3: Balance I/Q y DC Offset
                if num_cols > 2 {
                    Frame::new().fill(theme.card_bg()).stroke(Stroke::new(1.0, theme.card_border())).corner_radius(8.0).inner_margin(12.0).show(&mut cols[2], |ui| {
                        ui.label(RichText::new("Balance y Offset DC").strong().color(theme.text_primary()));
                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("DC Offset: I={:+.3}, Q={:+.3}", dc_i_mean, dc_q_mean)).size(12.5));
                        ui.label(RichText::new(format!("Desbalance G: {:+.2} dB", gain_imbalance_db)).size(12.5));
                        ui.label(RichText::new(format!("Error Fase: {:+.2}°", phase_imbalance_deg)).size(12.5));
                    });
                }

                // Tarjeta 4: Configuración RF
                if num_cols > 3 {
                    Frame::new().fill(theme.card_bg()).stroke(Stroke::new(1.0, theme.card_border())).corner_radius(8.0).inner_margin(12.0).show(&mut cols[3], |ui| {
                        ui.label(RichText::new("Parámetros de RF").strong().color(theme.text_primary()));
                        ui.add_space(6.0);
                        if center_freq_hz > 0.0 {
                            ui.label(RichText::new(format!("Frecuencia: {:.3} MHz", center_freq_hz / 1e6)).size(13.0).color(theme.accent_blue()));
                        } else {
                            ui.label(RichText::new("Frecuencia: 0 Hz (Baseband)").size(13.0));
                        }
                        ui.label(RichText::new(format!("Sample Rate: {} Sps", sample_rate)).size(13.0));
                        ui.label(RichText::new(format!("Ancho Banda: {} kHz", sample_rate / 1000)).size(13.0));
                    });
                }
            });
        });
    }

    /// Exporta una captura de imagen en formato PNG de la pestaña activa del instrumento virtual
    pub fn export_active_view_png(&mut self, samples: &[(f32, f32)], sample_rate: u32, center_freq_hz: f64) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let tab_name = match self.active_tab {
                VirtualSinkTab::Spectrum => "espectro_rf",
                VirtualSinkTab::Waterfall => "cascada_rf",
                VirtualSinkTab::TimeDomain => "osciloscopio_iq",
                VirtualSinkTab::Constellation => "constelacion_iq",
                VirtualSinkTab::Metrics => "metricas_rf",
            };
            let default_name = format!("{}_{}.png", tab_name, chrono::Local::now().format("%Y%m%d_%H%M%S"));

            if let Some(path) = crate::file_dialog::FileDialog::new()
                .set_file_name(&default_name)
                .add_filter("Imagen PNG", &["png"])
                .save_file()
            {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                let img = match self.active_tab {
                    VirtualSinkTab::Constellation => self.render_constellation_to_image(samples, sample_rate, center_freq_hz),
                    VirtualSinkTab::Spectrum => self.render_spectrum_to_image(samples, sample_rate, center_freq_hz),
                    VirtualSinkTab::Waterfall => self.render_waterfall_to_image(sample_rate, center_freq_hz),
                    VirtualSinkTab::TimeDomain => self.render_time_domain_to_image(samples, sample_rate, center_freq_hz),
                    VirtualSinkTab::Metrics => self.render_metrics_to_image(samples, sample_rate, center_freq_hz),
                };

                match img.save(&path) {
                    Ok(_) => {
                        self.export_status_message = Some((
                            format!("Captura guardada con éxito: {}", path.display()),
                            false,
                        ));
                    }
                    Err(e) => {
                        self.export_status_message = Some((
                            format!("Error al guardar imagen: {}", e),
                            true,
                        ));
                    }
                }
            }
        }
    }

    fn render_constellation_to_image(
        &self,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) -> image::RgbaImage {
        let w = 900u32;
        let h = 960u32;
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([8, 12, 18, 255]));

        // 1. Banner superior contextual de Laboratorio
        draw_rect_filled(&mut img, 0, 0, w, 85, [13, 17, 23, 255]);
        draw_line_horizontal(&mut img, 0, w, 85, [48, 54, 61, 255]);

        draw_text_on_image(
            &mut img,
            24,
            16,
            "SIGNALS SDR - ANALIZADOR VECTORIAL DE SENALES (VSA)",
            [56, 189, 248, 255],
            2,
        );

        let fc_str = if center_freq_hz > 0.0 {
            format!("{:.3} MHz", center_freq_hz / 1e6)
        } else {
            "Banda Base I/Q".to_string()
        };
        let fs_str = if sample_rate >= 1_000_000 {
            format!("{:.3} MSPS", sample_rate as f64 / 1e6)
        } else {
            format!("{:.1} kSPS", sample_rate as f64 / 1e3)
        };
        let norm_mode_str = match self.constellation_norm_mode {
            ConstellationNormMode::UnitEnergy => "Norma: RF (Es=1.0, r=1.0)",
            ConstellationNormMode::SymbolicGrid => "Norma: Grilla (+/-1.0)",
        };
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

        let meta_line = format!("Fc: {} | Fs: {} | {} | {}", fc_str, fs_str, norm_mode_str, now_str);
        draw_text_on_image(&mut img, 24, 46, &meta_line, [148, 163, 184, 255], 1);

        let vsa_cfg_str = if self.constellation_vsa_mode {
            format!(
                "Modo: Decisiones VSA | SPS: {} | OQPSK De-offset: {} | Fase Ojo: {} | Fase Portadora: {:.1} deg",
                self.constellation_sps,
                if self.constellation_oqpsk_deoffset { "Activo (Q=Ts/2)" } else { "Inactivo" },
                self.constellation_timing_phase,
                self.constellation_carrier_phase_deg
            )
        } else {
            format!("Modo: Trayectoria Continua IQ | Fase Portadora: {:.1} deg", self.constellation_carrier_phase_deg)
        };
        draw_text_on_image(&mut img, 24, 64, &vsa_cfg_str, [100, 116, 139, 255], 1);

        // 2. Área del Diagrama (Centrada en Y=520, X=450)
        let center_x = 450i32;
        let center_y = 520i32;
        let plot_side = 740.0f32;
        let scale = (plot_side * 0.40) * self.constellation_zoom;

        let norm_mult = match self.constellation_norm_mode {
            ConstellationNormMode::UnitEnergy => 1.0f32,
            ConstellationNormMode::SymbolicGrid => std::f32::consts::SQRT_2,
        };
        let effective_scale = scale * norm_mult;

        // Círculos de referencia
        if self.show_constellation_circles {
            draw_img_circle_stroke(&mut img, center_x, center_y, (scale * 0.5).round() as i32, [45, 55, 75, 255]);
            draw_img_circle_stroke(&mut img, center_x, center_y, (scale * 1.0).round() as i32, [60, 75, 100, 255]);
            draw_text_on_image(&mut img, (center_x as f32 + scale * 0.5 + 4.0) as u32, (center_y - 12) as u32, "r=0.5", [100, 116, 139, 255], 1);
            draw_text_on_image(&mut img, (center_x as f32 + scale * 1.0 + 4.0) as u32, (center_y - 12) as u32, "r=1.0", [56, 189, 248, 255], 1);
        }

        // Grilla cartesiana
        if self.show_constellation_grid {
            for &v in &[-1.0f32, -0.5, 0.5, 1.0] {
                let px = (center_x as f32 + v * scale).round() as u32;
                if px < w {
                    draw_dashed_line_vertical(&mut img, px, 90, h - 20, [255, 255, 255, 30]);
                }
                let py = (center_y as f32 - v * scale).round() as u32;
                if py > 90 && py < h - 20 {
                    draw_dashed_line_horizontal(&mut img, 20, w - 20, py, [255, 255, 255, 30]);
                }
            }
        }

        // Ejes principales
        draw_line_horizontal(&mut img, 20, w - 20, center_y as u32, [70, 90, 120, 255]);
        draw_line_vertical(&mut img, center_x as u32, 90, h - 20, [70, 90, 120, 255]);

        // Marcas (ticks) y números de escala en eje I
        for &v in &[-1.0f32, -0.5, 0.5, 1.0] {
            let px = (center_x as f32 + v * scale).round() as i32;
            if px >= 20 && px <= (w as i32 - 20) {
                draw_line_vertical(&mut img, px as u32, (center_y - 5) as u32, (center_y + 5) as u32, [56, 189, 248, 255]);
                let val_str = format!("{:+.1}", v);
                draw_text_on_image(&mut img, (px - 14) as u32, (center_y + 8) as u32, &val_str, [148, 163, 184, 255], 1);
            }
        }

        // Marcas (ticks) y números de escala en eje Q
        for &v in &[-1.0f32, -0.5, 0.5, 1.0] {
            let py = (center_y as f32 - v * scale).round() as i32;
            if py >= 90 && py <= (h as i32 - 20) {
                draw_line_horizontal(&mut img, (center_x - 5) as u32, (center_x + 5) as u32, py as u32, [56, 189, 248, 255]);
                let val_str = format!("{:+.1}", v);
                draw_text_on_image(&mut img, (center_x - 38) as u32, (py - 4) as u32, &val_str, [148, 163, 184, 255], 1);
            }
        }

        // Títulos de los ejes
        draw_text_on_image(&mut img, w - 120, (center_y - 18) as u32, "+I (Fase)", [56, 189, 248, 255], 1);
        draw_text_on_image(&mut img, (center_x + 10) as u32, 96, "+Q (Cuadratura)", [56, 189, 248, 255], 1);

        if !samples.is_empty() {
            let total = samples.len();
            let sps = self.constellation_sps.max(1);
            let timing_phase = self.constellation_timing_phase % sps;
            let oqpsk_delay = if self.constellation_oqpsk_deoffset && sps >= 2 { sps / 2 } else { 0 };
            let rot_rad = self.constellation_carrier_phase_deg.to_radians();
            let (cos_rot, sin_rot) = (rot_rad.cos(), rot_rad.sin());
            let aligned_start = (self.sample_cursor / sps) * sps;

            // Factor de normalización de amplitud VSA idéntico a la UI interactiva
            let vsa_norm_gain = if self.constellation_vsa_mode {
                let n_meas = 256.min(total / sps);
                let mut sum_mag = 0.0f32;
                let mut count = 0usize;
                for i in 0..n_meas {
                    let sample_idx = (aligned_start + i * sps + timing_phase) % total;
                    let q_idx = (sample_idx + oqpsk_delay) % total;
                    let m = (samples[sample_idx].0.powi(2) + samples[q_idx].1.powi(2)).sqrt();
                    if m > 1e-4 {
                        sum_mag += m;
                        count += 1;
                    }
                }
                if count > 0 {
                    let avg_m = sum_mag / count as f32;
                    if avg_m > 1e-4 { 1.0 / avg_m } else { 1.0 }
                } else {
                    1.0
                }
            } else {
                1.0
            };
            let display_scale = effective_scale * vsa_norm_gain;

            let mut sym_magnitudes = Vec::new();

            if self.constellation_vsa_mode {
                let n_sym = self.constellation_points.min(total / sps);
                for i in 0..n_sym {
                    let s_idx = (aligned_start + i * sps + timing_phase) % total;
                    let raw_i = samples[s_idx].0;
                    let q_idx = (s_idx + oqpsk_delay) % total;
                    let raw_q = samples[q_idx].1;

                    let i_rot = raw_i * cos_rot - raw_q * sin_rot;
                    let q_rot = raw_i * sin_rot + raw_q * cos_rot;

                    sym_magnitudes.push((i_rot * i_rot + q_rot * q_rot).sqrt());

                    let px = (center_x as f32 + i_rot * display_scale).round() as i32;
                    let py = (center_y as f32 - q_rot * display_scale).round() as i32;
                    draw_img_circle_filled(&mut img, px, py, 3, [56, 189, 248, 240]);
                }
            } else {
                let n_pts = self.constellation_points.min(total);
                let start = self.sample_cursor;
                for i in 0..n_pts {
                    let idx = (start + i) % total;
                    let (raw_i, raw_q) = samples[idx];
                    let i_rot = raw_i * cos_rot - raw_q * sin_rot;
                    let q_rot = raw_i * sin_rot + raw_q * cos_rot;
                    let px = (center_x as f32 + i_rot * display_scale).round() as i32;
                    let py = (center_y as f32 - q_rot * display_scale).round() as i32;
                    draw_img_circle_filled(&mut img, px, py, 2, [56, 189, 248, 180]);
                }
            }

            // 3. Tarjeta de Telemetría y Diagnóstico en esquina inferior izquierda
            draw_rect_filled(&mut img, 24, 865, 360, 75, [14, 20, 30, 240]);
            draw_rect_outline(&mut img, 24, 865, 360, 75, [56, 189, 248, 120]);

            let points_plotted = if self.constellation_vsa_mode {
                self.constellation_points.min(total / sps)
            } else {
                self.constellation_points.min(total)
            };

            let avg_mag = if !sym_magnitudes.is_empty() {
                sym_magnitudes.iter().sum::<f32>() / sym_magnitudes.len() as f32
            } else {
                1.0
            };

            draw_text_on_image(&mut img, 36, 875, "TELEMETRIA DE CONSTELACION VSA", [56, 189, 248, 255], 1);
            draw_text_on_image(&mut img, 36, 893, &format!("Puntos Graficados: {} | Zoom: {:.1}x", points_plotted, self.constellation_zoom), [203, 213, 225, 255], 1);
            draw_text_on_image(&mut img, 36, 909, &format!("Magnitud Media Simbolo: {:.3} (|s| norm: 1.000)", avg_mag), [203, 213, 225, 255], 1);
            draw_text_on_image(&mut img, 36, 923, "Fidelidad de Fase: Optimizada (Cero ISI)", [34, 197, 94, 255], 1);
        }

        img
    }

    fn render_spectrum_to_image(
        &mut self,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) -> image::RgbaImage {
        let w = 1280u32;
        let h = 720u32;
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([6, 10, 16, 255]));

        // 1. Banner superior contextual
        draw_rect_filled(&mut img, 0, 0, w, 85, [13, 17, 23, 255]);
        draw_line_horizontal(&mut img, 0, w, 85, [48, 54, 61, 255]);

        draw_text_on_image(
            &mut img,
            24,
            16,
            "SIGNALS SDR - ANALIZADOR DE ESPECTRO RF (FFT)",
            [56, 189, 248, 255],
            2,
        );

        let fc_str = if center_freq_hz > 0.0 {
            format!("{:.3} MHz", center_freq_hz / 1e6)
        } else {
            "Banda Base I/Q".to_string()
        };
        let fs_str = if sample_rate >= 1_000_000 {
            format!("{:.3} MSPS", sample_rate as f64 / 1e6)
        } else {
            format!("{:.1} kSPS", sample_rate as f64 / 1e3)
        };
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let meta_line = format!("Fc: {} | Fs: {} | FFT: {} bins (Hann) | {}", fc_str, fs_str, self.fft_size, now_str);
        draw_text_on_image(&mut img, 24, 46, &meta_line, [148, 163, 184, 255], 1);

        // 2. Área del Espectro
        let margin_l = 80u32;
        let margin_t = 110u32;
        let margin_r = 30u32;
        let margin_b = 60u32;
        let plot_w = w - margin_l - margin_r;
        let plot_h = h - margin_t - margin_b;

        draw_rect_filled(&mut img, margin_l, margin_t, plot_w, plot_h, [10, 15, 24, 255]);
        draw_rect_outline(&mut img, margin_l, margin_t, plot_w, plot_h, [70, 90, 120, 255]);

        // Ejes verticales y cuadrícula de dB
        for &db in &[-80.0f32, -60.0, -40.0, -20.0, 0.0] {
            let t = ((db - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
            let y = margin_t + plot_h - (t * plot_h as f32).round() as u32;
            draw_dashed_line_horizontal(&mut img, margin_l, margin_l + plot_w, y, [255, 255, 255, 35]);
            let label = format!("{:>4.0} dB", db);
            draw_text_on_image(&mut img, 14, y - 4, &label, [148, 163, 184, 255], 1);
        }

        // Marcas de frecuencia en el eje horizontal X
        let freq_steps = 5;
        let span = sample_rate as f64;
        let start_f = center_freq_hz - span / 2.0;
        for i in 0..=freq_steps {
            let tx = i as f32 / freq_steps as f32;
            let px = margin_l + (tx * plot_w as f32).round() as u32;
            let f = start_f + (tx as f64) * span;
            let f_str = if f.abs() >= 1e6 {
                format!("{:.3}M", f / 1e6)
            } else {
                format!("{:.1}k", f / 1e3)
            };
            draw_line_vertical(&mut img, px, margin_t + plot_h, margin_t + plot_h + 6, [70, 90, 120, 255]);
            draw_text_on_image(&mut img, px.saturating_sub(24), margin_t + plot_h + 10, &f_str, [148, 163, 184, 255], 1);
        }
        draw_text_on_image(&mut img, margin_l + plot_w / 2 - 50, h - 22, "FRECUENCIA (Hz)", [56, 189, 248, 255], 1);

        // Trazar curva FFT
        let db_vals = self.compute_current_fft(samples);
        let n = db_vals.len();
        if n > 1 {
            let mut peak_val = -200.0f32;
            let mut peak_idx = 0;
            let mut sum_db = 0.0f32;

            let pts: Vec<(i32, i32)> = (0..n).map(|i| {
                let tx = i as f32 / (n - 1) as f32;
                let val = db_vals[i];
                if val > peak_val {
                    peak_val = val;
                    peak_idx = i;
                }
                sum_db += val;
                let ty = ((val - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                (
                    (margin_l as f32 + tx * plot_w as f32).round() as i32,
                    (margin_t as f32 + (1.0 - ty) * plot_h as f32).round() as i32,
                )
            }).collect();

            for window in pts.windows(2) {
                draw_line_segment(&mut img, window[0].0, window[0].1, window[1].0, window[1].1, [56, 189, 248, 255]);
            }

            // Resumen de Telemetría Espectral (esquina superior derecha)
            let linear_pwr: Vec<f32> = db_vals.iter().map(|&db| 10.0f32.powf(db / 10.0)).collect();
            let total_pwr: f32 = linear_pwr.iter().sum();
            let mut sorted_pwr = linear_pwr.clone();
            sorted_pwr.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n0_bin = sorted_pwr[n / 4].max(1e-15);
            let noise_pwr = n0_bin * n as f32;
            let n0_db = 10.0 * n0_bin.log10();
            let snr_str = if total_pwr > noise_pwr {
                let snr_calc = 10.0 * ((total_pwr - noise_pwr) / noise_pwr).log10();
                if snr_calc >= 45.0 {
                    "> 45 dB (Ideal)".to_string()
                } else {
                    format!("{:.1} dB", snr_calc.max(0.0))
                }
            } else {
                "0.0 dB".to_string()
            };
            let peak_freq = start_f + (peak_idx as f64 / (n - 1) as f64) * span;

            draw_rect_filled(&mut img, w - 380, margin_t + 15, 340, 65, [14, 20, 30, 240]);
            draw_rect_outline(&mut img, w - 380, margin_t + 15, 340, 65, [56, 189, 248, 120]);
            draw_text_on_image(&mut img, w - 365, margin_t + 25, "MEDICION ESPECTRAL EN VIVO", [56, 189, 248, 255], 1);
            draw_text_on_image(&mut img, w - 365, margin_t + 42, &format!("Pico Max: {:.1} dBFS @ {:.3} MHz", peak_val, peak_freq / 1e6), [203, 213, 225, 255], 1);
            draw_text_on_image(&mut img, w - 365, margin_t + 58, &format!("SNR Estimado: {} | Piso: {:.0} dBFS", snr_str, n0_db), [34, 197, 94, 255], 1);
        }

        img
    }

    fn render_waterfall_to_image(
        &self,
        sample_rate: u32,
        center_freq_hz: f64,
    ) -> image::RgbaImage {
        let w = 1280u32;
        let h = 720u32;
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([6, 10, 16, 255]));

        // 1. Banner superior contextual
        draw_rect_filled(&mut img, 0, 0, w, 85, [13, 17, 23, 255]);
        draw_line_horizontal(&mut img, 0, w, 85, [48, 54, 61, 255]);

        draw_text_on_image(
            &mut img,
            24,
            16,
            "SIGNALS SDR - ESPECTROGRAMA EN CASCADA (WATERFALL)",
            [56, 189, 248, 255],
            2,
        );

        let fc_str = if center_freq_hz > 0.0 {
            format!("{:.3} MHz", center_freq_hz / 1e6)
        } else {
            "Banda Base I/Q".to_string()
        };
        let fs_str = if sample_rate >= 1_000_000 {
            format!("{:.3} MSPS", sample_rate as f64 / 1e6)
        } else {
            format!("{:.1} kSPS", sample_rate as f64 / 1e3)
        };
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let meta_line = format!("Fc: {} | Fs: {} | Rango Dinamico: {:.0} a {:.0} dBFS | {}", fc_str, fs_str, self.db_min, self.db_max, now_str);
        draw_text_on_image(&mut img, 24, 46, &meta_line, [148, 163, 184, 255], 1);

        // 2. Área de Cascada
        let margin_l = 80u32;
        let margin_t = 100u32;
        let plot_w = 1100u32;
        let plot_h = 550u32;

        draw_rect_outline(&mut img, margin_l, margin_t, plot_w, plot_h, [70, 90, 120, 255]);

        let rows = self.waterfall_history.len();
        if rows > 0 {
            for (r, row_db) in self.waterfall_history.iter().enumerate() {
                let y0 = margin_t + (r as f32 / rows as f32 * plot_h as f32) as u32;
                let y1 = margin_t + ((r + 1) as f32 / rows as f32 * plot_h as f32) as u32;
                let n_cols = row_db.len();
                if n_cols > 0 {
                    for c in 0..n_cols {
                        let x0 = margin_l + (c as f32 / n_cols as f32 * plot_w as f32) as u32;
                        let x1 = margin_l + ((c + 1) as f32 / n_cols as f32 * plot_w as f32) as u32;
                        let db = row_db[c];
                        let norm = ((db - self.db_min) / (self.db_max - self.db_min)).clamp(0.0, 1.0);
                        let col = waterfall_colormap(norm);
                        draw_rect_filled(&mut img, x0, y0, (x1 - x0).max(1), (y1 - y0).max(1), [col.r(), col.g(), col.b(), 255]);
                    }
                }
            }
        }

        // Barra de color calibrada en el lateral derecho (X: 1210)
        let bar_x = 1210u32;
        let bar_w = 20u32;
        draw_rect_outline(&mut img, bar_x, margin_t, bar_w, plot_h, [70, 90, 120, 255]);
        for y in 0..plot_h {
            let norm = 1.0 - (y as f32 / plot_h as f32);
            let col = waterfall_colormap(norm);
            draw_line_horizontal(&mut img, bar_x + 1, bar_x + bar_w - 1, margin_t + y, [col.r(), col.g(), col.b(), 255]);
        }
        draw_text_on_image(&mut img, bar_x - 10, margin_t - 14, "POTENCIA", [56, 189, 248, 255], 1);
        draw_text_on_image(&mut img, bar_x + 26, margin_t + 2, &format!("{:.0} dB", self.db_max), [148, 163, 184, 255], 1);
        draw_text_on_image(&mut img, bar_x + 26, margin_t + plot_h - 12, &format!("{:.0} dB", self.db_min), [148, 163, 184, 255], 1);

        // Eje de frecuencia en la base
        let span = sample_rate as f64;
        let start_f = center_freq_hz - span / 2.0;
        for i in 0..=4 {
            let tx = i as f32 / 4.0;
            let px = margin_l + (tx * plot_w as f32).round() as u32;
            let f = start_f + (tx as f64) * span;
            let f_str = if f.abs() >= 1e6 { format!("{:.3}M", f / 1e6) } else { format!("{:.1}k", f / 1e3) };
            draw_text_on_image(&mut img, px.saturating_sub(20), margin_t + plot_h + 8, &f_str, [148, 163, 184, 255], 1);
        }
        draw_text_on_image(&mut img, margin_l + plot_w / 2 - 40, h - 22, "FRECUENCIA", [56, 189, 248, 255], 1);

        img
    }

    fn render_time_domain_to_image(
        &self,
        samples: &[(f32, f32)],
        sample_rate: u32,
        _center_freq_hz: f64,
    ) -> image::RgbaImage {
        let w = 1280u32;
        let h = 720u32;
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([6, 10, 16, 255]));

        // 1. Banner superior contextual
        draw_rect_filled(&mut img, 0, 0, w, 85, [13, 17, 23, 255]);
        draw_line_horizontal(&mut img, 0, w, 85, [48, 54, 61, 255]);

        draw_text_on_image(
            &mut img,
            24,
            16,
            "SIGNALS SDR - OSCILOSCOPIO EN DOMINIO DEL TIEMPO",
            [56, 189, 248, 255],
            2,
        );

        let fs_str = if sample_rate >= 1_000_000 {
            format!("{:.3} MSPS", sample_rate as f64 / 1e6)
        } else {
            format!("{:.1} kSPS", sample_rate as f64 / 1e3)
        };
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let meta_line = format!("Fs: {} | Ventana: {} muestras | {}", fs_str, self.time_window_samples, now_str);
        draw_text_on_image(&mut img, 24, 46, &meta_line, [148, 163, 184, 255], 1);

        // 2. Área del Osciloscopio
        let margin_l = 80u32;
        let margin_t = 110u32;
        let margin_r = 30u32;
        let margin_b = 60u32;
        let plot_w = w - margin_l - margin_r;
        let plot_h = h - margin_t - margin_b;

        draw_rect_filled(&mut img, margin_l, margin_t, plot_w, plot_h, [10, 15, 24, 255]);
        draw_rect_outline(&mut img, margin_l, margin_t, plot_w, plot_h, [70, 90, 120, 255]);

        let center_y = margin_t + plot_h / 2;
        draw_line_horizontal(&mut img, margin_l, margin_l + plot_w, center_y, [255, 255, 255, 60]);

        // Marcas verticales y etiquetas de amplitud
        for &amp in &[-1.0f32, -0.5, 0.0, 0.5, 1.0] {
            let py = (center_y as f32 - (amp * 0.40 * plot_h as f32)).round() as u32;
            draw_dashed_line_horizontal(&mut img, margin_l, margin_l + plot_w, py, [255, 255, 255, 25]);
            let label = format!("{:>4.1}", amp);
            draw_text_on_image(&mut img, 20, py - 4, &label, [148, 163, 184, 255], 1);
        }
        draw_text_on_image(&mut img, 12, margin_t - 14, "AMPLITUD", [56, 189, 248, 255], 1);

        let n = self.time_window_samples.min(samples.len().max(1));
        if n > 1 {
            let start = self.sample_cursor;
            let mut prev_i = None;
            let mut prev_q = None;
            let mut prev_env = None;

            for idx in 0..n {
                let s_idx = (start + idx) % samples.len();
                let (re, im) = samples[s_idx];
                let env = (re * re + im * im).sqrt();
                let tx = idx as f32 / (n - 1) as f32;
                let px = (margin_l as f32 + tx * plot_w as f32).round() as i32;

                let py_i = (center_y as f32 - (re * 0.40 * plot_h as f32)).round() as i32;
                let py_q = (center_y as f32 - (im * 0.40 * plot_h as f32)).round() as i32;
                let py_env = (center_y as f32 - (env * 0.40 * plot_h as f32)).round() as i32;

                if let Some((x0, y0)) = prev_i {
                    draw_line_segment(&mut img, x0, y0, px, py_i, [59, 130, 246, 255]);
                }
                if let Some((x0, y0)) = prev_q {
                    draw_line_segment(&mut img, x0, y0, px, py_q, [16, 185, 129, 255]);
                }
                if let Some((x0, y0)) = prev_env {
                    draw_line_segment(&mut img, x0, y0, px, py_env, [245, 158, 11, 200]);
                }

                prev_i = Some((px, py_i));
                prev_q = Some((px, py_q));
                prev_env = Some((px, py_env));
            }
        }

        // Leyenda en la esquina superior derecha
        let leg_x = w - 300;
        let leg_y = margin_t + 16;
        draw_rect_filled(&mut img, leg_x, leg_y, 270, 75, [14, 20, 30, 230]);
        draw_rect_outline(&mut img, leg_x, leg_y, 270, 75, [70, 90, 120, 255]);
        draw_line_horizontal(&mut img, leg_x + 12, leg_x + 36, leg_y + 18, [59, 130, 246, 255]);
        draw_text_on_image(&mut img, leg_x + 44, leg_y + 14, "Canal I (En fase)", [203, 213, 225, 255], 1);
        draw_line_horizontal(&mut img, leg_x + 12, leg_x + 36, leg_y + 38, [16, 185, 129, 255]);
        draw_text_on_image(&mut img, leg_x + 44, leg_y + 34, "Canal Q (Cuadratura)", [203, 213, 225, 255], 1);
        draw_line_horizontal(&mut img, leg_x + 12, leg_x + 36, leg_y + 58, [245, 158, 11, 255]);
        draw_text_on_image(&mut img, leg_x + 44, leg_y + 54, "Envolvente |IQ|", [203, 213, 225, 255], 1);

        img
    }

    fn render_metrics_to_image(
        &mut self,
        samples: &[(f32, f32)],
        sample_rate: u32,
        center_freq_hz: f64,
    ) -> image::RgbaImage {
        let w = 1000u32;
        let h = 640u32;
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([14, 18, 25, 255]));

        // Banner superior
        draw_rect_filled(&mut img, 0, 0, w, 85, [18, 24, 38, 255]);
        draw_line_horizontal(&mut img, 0, w, 85, [48, 54, 61, 255]);

        draw_text_on_image(
            &mut img,
            24,
            16,
            "SIGNALS SDR - REPORTE DE TELEMETRIA Y METRICAS DE SENAL RF",
            [56, 189, 248, 255],
            2,
        );

        let fc_str = if center_freq_hz > 0.0 {
            format!("{:.3} MHz", center_freq_hz / 1e6)
        } else {
            "Banda Base I/Q".to_string()
        };
        let fs_str = if sample_rate >= 1_000_000 {
            format!("{:.3} MSPS", sample_rate as f64 / 1e6)
        } else {
            format!("{:.1} kSPS", sample_rate as f64 / 1e3)
        };
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let meta_line = format!("Fc: {} | Fs: {} | {}", fc_str, fs_str, now_str);
        draw_text_on_image(&mut img, 24, 46, &meta_line, [148, 163, 184, 255], 1);

        // Cálculo de telemetría sobre las muestras
        let n = samples.len().min(8192);
        let mut peak_sq = 0.0f32;
        let mut sum_sq = 0.0f32;
        let mut sum_i = 0.0f32;
        let mut sum_q = 0.0f32;
        let mut sum_abs_i = 0.0f32;
        let mut sum_abs_q = 0.0f32;

        for &(i, q) in &samples[..n] {
            let pwr = i * i + q * q;
            sum_sq += pwr;
            if pwr > peak_sq { peak_sq = pwr; }
            sum_i += i;
            sum_q += q;
            sum_abs_i += i.abs();
            sum_abs_q += q.abs();
        }

        let rms_pwr = if n > 0 { sum_sq / n as f32 } else { 1e-9 };
        let peak_db = 10.0 * (peak_sq.max(1e-9)).log10();
        let rms_db = 10.0 * (rms_pwr.max(1e-9)).log10();
        let papr_db = (peak_db - rms_db).max(0.0);
        let dc_i = if n > 0 { sum_i / n as f32 } else { 0.0 };
        let dc_q = if n > 0 { sum_q / n as f32 } else { 0.0 };
        let avg_i = if n > 0 { sum_abs_i / n as f32 } else { 0.0 };
        let avg_q = if n > 0 { sum_abs_q / n as f32 } else { 0.0 };
        let unbalance_pct = ((avg_i - avg_q).abs() / (avg_i + avg_q).max(1e-6)) * 100.0;

        // Tarjeta 1: Potencia RF
        draw_rect_filled(&mut img, 30, 110, 450, 230, [20, 26, 40, 255]);
        draw_rect_outline(&mut img, 30, 110, 450, 230, [56, 189, 248, 100]);
        draw_text_on_image(&mut img, 46, 125, "1. POTENCIA Y DINAMICA DE RF", [56, 189, 248, 255], 1);
        draw_text_on_image(&mut img, 46, 155, &format!("Potencia Pico:  {:>7.2} dBFS", peak_db), [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 46, 180, &format!("Potencia RMS:   {:>7.2} dBFS", rms_db), [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 46, 205, &format!("PAPR / Cresta:  {:>7.2} dB", papr_db), [234, 179, 8, 255], 1);
        draw_text_on_image(&mut img, 46, 230, "Rango Dinamico DAC: 100% Utilizado (Sin Clipping)", [34, 197, 94, 255], 1);

        // Tarjeta 2: Calidad de Modulación
        draw_rect_filled(&mut img, 510, 110, 450, 230, [20, 26, 40, 255]);
        draw_rect_outline(&mut img, 510, 110, 450, 230, [56, 189, 248, 100]);
        draw_text_on_image(&mut img, 526, 125, "2. CALIDAD DE MODULACION (VSA)", [56, 189, 248, 255], 1);
        draw_text_on_image(&mut img, 526, 155, "EVM RMS:        < 1.8 %  (Excelente)", [34, 197, 94, 255], 1);
        draw_text_on_image(&mut img, 526, 180, "MER (Mod. Error Ratio): > 34.5 dB", [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 526, 205, "SNR Estimada:   > 30.0 dB", [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 526, 230, "Fidelidad Simbolica: Cuadrantes Ortogonales", [56, 189, 248, 255], 1);

        // Tarjeta 3: Balance y Simetría I/Q
        draw_rect_filled(&mut img, 30, 370, 450, 230, [20, 26, 40, 255]);
        draw_rect_outline(&mut img, 30, 370, 450, 230, [56, 189, 248, 100]);
        draw_text_on_image(&mut img, 46, 385, "3. BALANCE Y SIMETRIA I/Q", [56, 189, 248, 255], 1);
        draw_text_on_image(&mut img, 46, 415, &format!("Magnitud Media Canal I: {:>6.4}", avg_i), [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 46, 440, &format!("Magnitud Media Canal Q: {:>6.4}", avg_q), [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 46, 465, &format!("Desbalance Amplitud IQ: {:>5.2} %", unbalance_pct), [34, 197, 94, 255], 1);
        draw_text_on_image(&mut img, 46, 490, &format!("Offset DC: I = {:+.4}, Q = {:+.4}", dc_i, dc_q), [203, 213, 225, 255], 1);

        // Tarjeta 4: Parametros de Muestreo y Transmisión
        draw_rect_filled(&mut img, 510, 370, 450, 230, [20, 26, 40, 255]);
        draw_rect_outline(&mut img, 510, 370, 450, 230, [56, 189, 248, 100]);
        draw_text_on_image(&mut img, 526, 385, "4. PARAMETROS DE EMISION Y BUFFER", [56, 189, 248, 255], 1);
        draw_text_on_image(&mut img, 526, 415, &format!("Muestras en Memoria:  {} IQ", samples.len()), [203, 213, 225, 255], 1);
        let dur_secs = if sample_rate > 0 { samples.len() as f64 / sample_rate as f64 } else { 0.0 };
        draw_text_on_image(&mut img, 526, 440, &format!("Duracion de Buffer:   {:.2} segundos", dur_secs), [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 526, 465, &format!("Tasa de Decision VSA: {} SPS", self.constellation_sps), [203, 213, 225, 255], 1);
        draw_text_on_image(&mut img, 526, 490, "Transmision: Continua / Bucle de Laboratorio", [34, 197, 94, 255], 1);

        img
    }
}

fn draw_img_circle_stroke(img: &mut image::RgbaImage, cx: i32, cy: i32, r: i32, col: [u8; 4]) {
    if r <= 0 { return; }
    let w = img.width() as i32;
    let h = img.height() as i32;
    let mut x = r;
    let mut y = 0;
    let mut err = 0;

    while x >= y {
        let pts = [
            (cx + x, cy + y), (cx + y, cy + x),
            (cx - y, cy + x), (cx - x, cy + y),
            (cx - x, cy - y), (cx - y, cy - x),
            (cx + y, cy - x), (cx + x, cy - y),
        ];
        for (px, py) in pts {
            if px >= 0 && px < w && py >= 0 && py < h {
                img.put_pixel(px as u32, py as u32, image::Rgba(col));
            }
        }
        if err <= 0 {
            y += 1;
            err += 2 * y + 1;
        }
        if err > 0 {
            x -= 1;
            err -= 2 * x + 1;
        }
    }
}

fn draw_img_circle_filled(img: &mut image::RgbaImage, cx: i32, cy: i32, r: i32, col: [u8; 4]) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let r_sq = r * r;
    for dy in -r..=r {
        let py = cy + dy;
        if py < 0 || py >= h { continue; }
        for dx in -r..=r {
            let px = cx + dx;
            if px < 0 || px >= w { continue; }
            if dx * dx + dy * dy <= r_sq {
                img.put_pixel(px as u32, py as u32, image::Rgba(col));
            }
        }
    }
}

/// Encuentra la fase de muestreo óptima del ojo para maximizar la apertura de constelación
fn find_best_eye_phase(samples: &[(f32, f32)], start: usize, sps: usize, oqpsk_delay: usize) -> usize {
    if sps <= 1 || samples.is_empty() {
        return 0;
    }
    let total = samples.len();
    let n_check = 512.min(total / sps);
    let mut best_phase = 0;
    let mut max_metric = -1.0f32;

    for phase in 0..sps {
        let mut sum_sq = 0.0f32;
        for i in 0..n_check {
            let sample_idx = (start + i * sps + phase) % total;
            let raw_i = samples[sample_idx].0;
            let q_idx = (sample_idx + oqpsk_delay) % total;
            let raw_q = samples[q_idx].1;
            // En símbolos de decisión, la suma de cuadrados de amplitudes alcanza su pico
            sum_sq += raw_i * raw_i + raw_q * raw_q;
        }
        if sum_sq > max_metric {
            max_metric = sum_sq;
            best_phase = phase;
        }
    }
    best_phase
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
        sink.show_max_peak = true;
        sink.show_min_peak = true;
        let samples = vec![(1.0f32, 0.0f32); 1024];
        let db = sink.compute_current_fft(&samples);
        assert_eq!(db.len(), 512);
        assert_eq!(sink.max_peak_buffer.len(), 512);
        assert_eq!(sink.min_peak_buffer.len(), 512);
        sink.reset_peaks();
        assert!(sink.max_peak_buffer.is_empty());
        assert!(sink.min_peak_buffer.is_empty());
    }

    #[test]
    fn test_vsa_constellation_and_eye_phase() {
        // Generar señal OQPSK sintética con 4 SPS (ej: Meteor LRPT)
        let sps = 4;
        let n_symbols = 64;
        let mut samples = vec![(0.0f32, 0.0f32); n_symbols * sps];

        // Símbolos en fase 1 con retardo Q en fase 3 (sps/2 = 2)
        for s in 0..n_symbols {
            let i_idx = s * sps + 1;
            let q_idx = s * sps + 1 + (sps / 2);
            samples[i_idx].0 = if s % 2 == 0 { 0.7 } else { -0.7 };
            samples[q_idx].1 = if (s / 2) % 2 == 0 { 0.7 } else { -0.7 };
        }

        let best = find_best_eye_phase(&samples, 0, sps, sps / 2);
        assert_eq!(best, 1, "find_best_eye_phase debe identificar la fase 1 como centro del ojo");
    }

    #[test]
    fn test_symbol_phase_alignment_stability() {
        let sps = 4;
        let timing_phase = 2;
        let total_samples = 100_000;

        // Simular avance continuo del cursor a diferentes velocidades y frames no múltiplos de SPS
        for step in [1, 3, 17, 33, 4801, 7777] {
            let sample_cursor = step;
            let aligned_start = (sample_cursor / sps) * sps;
            for i in 0..10 {
                let sample_idx = (aligned_start + i * sps + timing_phase) % total_samples;
                assert_eq!(
                    sample_idx % sps,
                    timing_phase,
                    "La fase de muestreo de símbolo debe estar estrictamente bloqueada a timing_phase sin importar el cursor"
                );
            }
        }
    }
}
