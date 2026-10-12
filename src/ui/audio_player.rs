use crate::icons;
use crate::theme::AppTheme;
use egui::{Color32, Frame, Pos2, Rect, Response, RichText, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Default)]
pub struct AudioPlayerWidget {
    pub is_playing: bool,
    pub current_time_sec: f32,
    pub total_duration_sec: f32,
}

impl AudioPlayerWidget {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            current_time_sec: 0.0,
            total_duration_sec: 0.0,
        }
    }

    pub fn render(&mut self, ui: &mut Ui, theme: &AppTheme, audio_path: Option<&str>) {
        Frame::none()
            .fill(theme.card_bg())
            .stroke(Stroke::new(1.0, theme.card_border()))
            .rounding(8.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if let Some(path) = audio_path {
                        let name = std::path::Path::new(path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(path);
                        ui.label(
                            RichText::new(format!("Audio cargado: {}", name))
                                .size(13.0)
                                .color(theme.text_primary()),
                        );
                    } else {
                        ui.label(
                            RichText::new("Sin archivo de audio seleccionado")
                                .size(13.0)
                                .color(theme.text_secondary()),
                        );
                    }

                    ui.add_space(6.0);

                    // Barra de progreso y tiempos
                    let curr_str = format_time(self.current_time_sec);
                    let total_str = format_time(self.total_duration_sec);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(curr_str).size(12.0).color(theme.text_secondary()));

                        // Barra de progreso visual
                        let available_w = ui.available_width() - 50.0;
                        let (bar_rect, _) = ui.allocate_exact_size(
                            Vec2::new(available_w.max(60.0), 12.0),
                            egui::Sense::hover(),
                        );

                        let bg_fill = if *theme == AppTheme::Dark {
                            Color32::from_rgb(35, 42, 54)
                        } else {
                            Color32::from_rgb(220, 228, 238)
                        };
                        ui.painter().rect_filled(bar_rect, 4.0, bg_fill);

                        let ratio = if self.total_duration_sec > 0.0 {
                            (self.current_time_sec / self.total_duration_sec).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };

                        if ratio > 0.0 {
                            let prog_rect = Rect::from_min_size(
                                bar_rect.min,
                                Vec2::new(bar_rect.width() * ratio, bar_rect.height()),
                            );
                            ui.painter().rect_filled(prog_rect, 4.0, theme.accent_blue());
                        }

                        ui.label(RichText::new(total_str).size(12.0).color(theme.text_secondary()));
                    });

                    ui.add_space(6.0);

                    // Controles de reproducción con iconos vectoriales
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        // Botón Reproducir / Pausa
                        let play_btn_size = Vec2::new(32.0, 28.0);
                        let (play_rect, play_resp) = ui.allocate_exact_size(play_btn_size, egui::Sense::click());
                        let play_fill = if play_resp.hovered() {
                            theme.card_border()
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(play_rect, 4.0, play_fill);
                        ui.painter().rect_stroke(play_rect, 4.0, Stroke::new(1.0, theme.card_border()), egui::StrokeKind::Inside);

                        if self.is_playing {
                            // Pausa
                            let c = play_rect.center();
                            let s = Stroke::new(2.0, theme.accent_blue());
                            ui.painter().line_segment([Pos2::new(c.x - 4.0, c.y - 7.0), Pos2::new(c.x - 4.0, c.y + 7.0)], s);
                            ui.painter().line_segment([Pos2::new(c.x + 4.0, c.y - 7.0), Pos2::new(c.x + 4.0, c.y + 7.0)], s);
                        } else {
                            icons::draw_play_icon(ui, play_rect, theme.accent_blue());
                        }

                        if play_resp.clicked() {
                            self.is_playing = !self.is_playing;
                        }

                        // Botón Detener
                        let (stop_rect, stop_resp) = ui.allocate_exact_size(play_btn_size, egui::Sense::click());
                        let stop_fill = if stop_resp.hovered() {
                            theme.card_border()
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(stop_rect, 4.0, stop_fill);
                        ui.painter().rect_stroke(stop_rect, 4.0, Stroke::new(1.0, theme.card_border()), egui::StrokeKind::Inside);
                        icons::draw_stop_icon(ui, stop_rect, theme.text_secondary());

                        if stop_resp.clicked() {
                            self.is_playing = false;
                            self.current_time_sec = 0.0;
                        }
                    });
                });
            });
    }
}

fn format_time(sec: f32) -> String {
    let total_s = sec.floor() as u32;
    let mins = total_s / 60;
    let secs = total_s % 60;
    format!("{:02}:{:02}", mins, secs)
}
