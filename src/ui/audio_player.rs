use crate::icons;
use crate::theme::AppTheme;
use egui::{Color32, Frame, Pos2, Rect, RichText, Stroke, Ui, Vec2};
use std::fs::File;
use std::io::BufReader;

pub struct AudioPlayerWidget {
    pub is_playing: bool,
    pub current_time_sec: f32,
    pub total_duration_sec: f32,
    loaded_path: Option<String>,
    player: Option<rodio::Player>,
    _device_sink: Option<rodio::MixerDeviceSink>,
}

impl Default for AudioPlayerWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayerWidget {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            current_time_sec: 0.0,
            total_duration_sec: 0.0,
            loaded_path: None,
            player: None,
            _device_sink: None,
        }
    }

    pub fn start_playback(&mut self, path: &str) {
        self.stop_playback();
        if let Ok(device_sink) = rodio::DeviceSinkBuilder::open_default_sink() {
            let player = rodio::Player::connect_new(&device_sink.mixer());
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                if let Ok(source) = rodio::Decoder::try_from(reader) {
                    player.append(source);
                    self.player = Some(player);
                    self._device_sink = Some(device_sink);
                    self.is_playing = true;
                    self.loaded_path = Some(path.to_string());
                }
            }
        }
    }

    pub fn stop_playback(&mut self) {
        if let Some(player) = &self.player {
            player.stop();
        }
        self.player = None;
        self._device_sink = None;
        self.is_playing = false;
        self.current_time_sec = 0.0;
    }

    pub fn render(&mut self, ui: &mut Ui, theme: &AppTheme, audio_path: Option<&str>) {
        // Si el archivo cambió, detener reproducción previa
        if let Some(path) = audio_path {
            if self.loaded_path.as_deref() != Some(path) && self.is_playing {
                self.stop_playback();
                self.loaded_path = Some(path.to_string());
            }
        } else if self.loaded_path.is_some() {
            self.stop_playback();
            self.loaded_path = None;
        }

        // Actualizar progreso si está reproduciendo
        if let Some(player) = &self.player {
            if player.empty() {
                self.is_playing = false;
                self.current_time_sec = 0.0;
                self.player = None;
                self._device_sink = None;
            } else if self.is_playing {
                self.current_time_sec = player.get_pos().as_secs_f32();
                ui.ctx().request_repaint();
            }
        }

        Frame::new()
            .fill(theme.card_bg())
            .stroke(Stroke::new(1.0_f32, theme.card_border()))
            .corner_radius(8.0)
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

                        // Barra de progreso visual interactiva
                        let available_w = ui.available_width() - 50.0;
                        let (bar_rect, bar_resp) = ui.allocate_exact_size(
                            Vec2::new(available_w.max(60.0), 12.0),
                            egui::Sense::click_and_drag(),
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

                        // Permitir clic para buscar posición (seek)
                        if bar_resp.clicked() || bar_resp.dragged() {
                            if let Some(pos) = bar_resp.interact_pointer_pos() {
                                let click_ratio = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                                let seek_sec = click_ratio * self.total_duration_sec;
                                if let Some(player) = &self.player {
                                    let _ = player.try_seek(std::time::Duration::from_secs_f32(seek_sec));
                                }
                                self.current_time_sec = seek_sec;
                            }
                        }

                        ui.label(RichText::new(total_str).size(12.0).color(theme.text_secondary()));
                    });

                    ui.add_space(6.0);

                    // Controles de reproducción con iconos vectoriales
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        // Botón Reproducir / Pausa (reproducción interna en la app)
                        let play_btn_size = Vec2::new(32.0, 28.0);
                        let (play_rect, play_resp) = ui.allocate_exact_size(play_btn_size, egui::Sense::click());
                        let play_fill = if play_resp.hovered() {
                            theme.card_border()
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(play_rect, 4.0, play_fill);
                        ui.painter().rect_stroke(play_rect, 4.0, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);

                        if self.is_playing {
                            // Icono de Pausa
                            let c = play_rect.center();
                            let s = Stroke::new(2.0_f32, theme.accent_blue());
                            ui.painter().line_segment([Pos2::new(c.x - 4.0, c.y - 7.0), Pos2::new(c.x - 4.0, c.y + 7.0)], s);
                            ui.painter().line_segment([Pos2::new(c.x + 4.0, c.y - 7.0), Pos2::new(c.x + 4.0, c.y + 7.0)], s);
                        } else {
                            // Icono de Play
                            icons::draw_play_icon(ui, play_rect, theme.accent_blue());
                        }

                        if play_resp.clicked() {
                            if let Some(path) = audio_path {
                                if self.is_playing {
                                    if let Some(player) = &self.player {
                                        player.pause();
                                    }
                                    self.is_playing = false;
                                } else if let Some(player) = &self.player {
                                    if !player.empty() {
                                        player.play();
                                        self.is_playing = true;
                                    } else {
                                        self.start_playback(path);
                                    }
                                } else {
                                    self.start_playback(path);
                                }
                            }
                        }

                        // Botón Detener
                        let (stop_rect, stop_resp) = ui.allocate_exact_size(play_btn_size, egui::Sense::click());
                        let stop_fill = if stop_resp.hovered() {
                            theme.card_border()
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(stop_rect, 4.0, stop_fill);
                        ui.painter().rect_stroke(stop_rect, 4.0, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);
                        icons::draw_stop_icon(ui, stop_rect, theme.text_secondary());

                        if stop_resp.clicked() {
                            self.stop_playback();
                        }

                        // Botón Abrir en Reproductor Predeterminado (software externo)
                        if let Some(path) = audio_path {
                            ui.add_space(10.0);
                            let open_player_btn = egui::Button::new(
                                RichText::new("Abrir en Reproductor Predeterminado").size(12.0),
                            )
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .min_size(Vec2::new(0.0, 28.0));

                            if ui.add(open_player_btn).clicked() {
                                crate::utils::open_path_or_url(path);
                            }
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
