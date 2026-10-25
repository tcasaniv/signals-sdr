use crate::icons;
use crate::theme::AppTheme;
use egui::{Color32, Frame, Rect, RichText, Stroke, Ui, Vec2};

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::io::BufReader;

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
mod rodio {
    use std::time::Duration;

    pub struct Player;
    impl Player {
        pub fn empty(&self) -> bool {
            true
        }
        pub fn get_pos(&self) -> Duration {
            Duration::ZERO
        }
        pub fn try_seek(&self, _pos: Duration) -> Result<(), ()> {
            Ok(())
        }
        pub fn pause(&self) {}
        pub fn play(&self) {}
        pub fn stop(&self) {}
    }

    pub struct MixerDeviceSink;
}

pub struct AudioPlayerWidget {
    pub is_playing: bool,
    pub current_time_sec: f32,
    pub total_duration_sec: f32,
    pub loaded_name: Option<String>,
    pub loaded_path: Option<String>,
    pub loaded_bytes: Option<Vec<u8>>,
    #[cfg(target_arch = "wasm32")]
    blob_url: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    player: Option<rodio::Player>,
    #[cfg(not(target_arch = "wasm32"))]
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
            loaded_name: None,
            loaded_path: None,
            loaded_bytes: None,
            #[cfg(target_arch = "wasm32")]
            blob_url: None,
            #[cfg(not(target_arch = "wasm32"))]
            player: None,
            #[cfg(not(target_arch = "wasm32"))]
            _device_sink: None,
        }
    }

    /// Carga un archivo de audio desde su ruta en disco (Desktop)
    pub fn load_path(&mut self, path: &str) {
        self.stop_playback();
        self.loaded_path = Some(path.to_string());
        self.loaded_name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(reader) = hound::WavReader::new(std::io::Cursor::new(&bytes)) {
                    let spec = reader.spec();
                    if spec.sample_rate > 0 {
                        self.total_duration_sec = reader.duration() as f32 / spec.sample_rate as f32;
                    }
                }
                self.loaded_bytes = Some(bytes);
            }
        }
        self.current_time_sec = 0.0;
    }

    /// Carga un archivo de audio directamente desde memoria (WebAssembly y Desktop)
    pub fn load_bytes(&mut self, name: &str, bytes: Vec<u8>) {
        self.stop_playback();
        self.loaded_name = Some(name.to_string());
        self.loaded_path = None;

        // Auto-cálculo de duración si es formato WAV
        if let Ok(reader) = hound::WavReader::new(std::io::Cursor::new(&bytes)) {
            let spec = reader.spec();
            if spec.sample_rate > 0 {
                self.total_duration_sec = reader.duration() as f32 / spec.sample_rate as f32;
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(old) = self.blob_url.take() {
                web_sys::Url::revoke_object_url(&old).ok();
            }
            if let Ok(url) = crate::web_utils::create_audio_blob_url(&bytes) {
                self.blob_url = Some(url);
            }
        }

        self.loaded_bytes = Some(bytes);
        self.current_time_sec = 0.0;
    }

    /// Retorna true si hay un recurso de audio cargado
    pub fn has_audio(&self) -> bool {
        self.loaded_path.is_some() || self.loaded_bytes.is_some()
    }

    /// Inicia o reanuda la reproducción
    pub fn start_playback(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            if self.blob_url.is_none() {
                if let Some(ref bytes) = self.loaded_bytes {
                    if let Ok(url) = crate::web_utils::create_audio_blob_url(bytes) {
                        self.blob_url = Some(url);
                    }
                }
            }

            if let Some(ref url) = self.blob_url {
                crate::web_utils::web_audio_play(url, self.current_time_sec);
                self.is_playing = true;
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.player.is_none() {
                if let Ok(device_sink) = rodio::DeviceSinkBuilder::open_default_sink() {
                    let player = rodio::Player::connect_new(&device_sink.mixer());
                    let mut loaded = false;

                    if let Some(ref path) = self.loaded_path {
                        if let Ok(file) = File::open(path) {
                            let reader = BufReader::new(file);
                            if let Ok(source) = rodio::Decoder::try_from(reader) {
                                player.append(source);
                                loaded = true;
                            }
                        }
                    } else if let Some(ref bytes) = self.loaded_bytes {
                        let cursor = std::io::Cursor::new(bytes.clone());
                        if let Ok(source) = rodio::Decoder::try_from(cursor) {
                            player.append(source);
                            loaded = true;
                        }
                    }

                    if loaded {
                        if self.current_time_sec > 0.0 {
                            let _ = player.try_seek(std::time::Duration::from_secs_f32(self.current_time_sec));
                        }
                        self.player = Some(player);
                        self._device_sink = Some(device_sink);
                        self.is_playing = true;
                    }
                }
            } else if let Some(player) = &self.player {
                player.play();
                self.is_playing = true;
            }
        }
    }

    /// Pausa la reproducción manteniendo la posición actual
    pub fn pause_playback(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            crate::web_utils::web_audio_pause();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(player) = &self.player {
                player.pause();
            }
        }
        self.is_playing = false;
    }

    /// Detiene la reproducción de forma instantánea y resetea a 00:00
    pub fn stop_playback(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            crate::web_utils::web_audio_stop();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(player) = &self.player {
                player.stop();
            }
            self.player = None;
            self._device_sink = None;
        }

        self.is_playing = false;
        self.current_time_sec = 0.0;
    }

    /// Alias para stop_playback
    pub fn stop(&mut self) {
        self.stop_playback();
    }

    /// Busca una posición específica en segundos
    pub fn seek(&mut self, sec: f32) {
        let target_sec = sec.clamp(0.0, self.total_duration_sec.max(0.0));
        self.current_time_sec = target_sec;

        #[cfg(target_arch = "wasm32")]
        {
            crate::web_utils::web_audio_seek(target_sec);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(player) = &self.player {
                let _ = player.try_seek(std::time::Duration::from_secs_f32(target_sec));
            }
        }
    }

    /// Renderiza la interfaz del reproductor interactivo
    pub fn render(&mut self, ui: &mut Ui, theme: &AppTheme, audio_path: Option<&str>) {
        // Sincronización automática de ruta si se pasa un nuevo audio externo
        if let Some(path) = audio_path {
            if self.loaded_path.as_deref() != Some(path) && self.loaded_name.as_deref() != Some(path) {
                self.load_path(path);
            }
        }

        // En WebAssembly, consultar estado en tiempo real del elemento de audio web
        #[cfg(target_arch = "wasm32")]
        if self.is_playing {
            if let Some((playing, current, duration)) = crate::web_utils::web_audio_status() {
                self.is_playing = playing;
                self.current_time_sec = current;
                if duration > 0.0 && self.total_duration_sec <= 0.0 {
                    self.total_duration_sec = duration;
                }
            }
            ui.ctx().request_repaint();
        }

        // En Desktop, sincronizar con el player de rodio
        #[cfg(not(target_arch = "wasm32"))]
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
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Cabecera: Icono de Material Design y Nombre del audio
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(icons::ICON_AUDIOTRACK.codepoint)
                                .size(15.0)
                                .color(theme.accent_blue()),
                        );
                        ui.add_space(4.0);

                        let title_str = if let Some(ref name) = self.loaded_name {
                            format!("Audio: {}", name)
                        } else if let Some(path) = audio_path {
                            let name = std::path::Path::new(path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(path);
                            format!("Audio: {}", name)
                        } else {
                            "Sin archivo de audio seleccionado".to_string()
                        };

                        ui.label(
                            RichText::new(title_str)
                                .size(13.0)
                                .strong()
                                .color(if self.has_audio() || audio_path.is_some() {
                                    theme.text_primary()
                                } else {
                                    theme.text_secondary()
                                }),
                        );
                    });

                    ui.add_space(8.0);

                    // Barra de progreso y tiempos
                    let curr_str = format_time(self.current_time_sec);
                    let total_str = format_time(self.total_duration_sec);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(curr_str)
                                .size(12.0)
                                .monospace()
                                .color(theme.text_secondary()),
                        );

                        // Barra de progreso interactiva
                        let available_w = (ui.available_width() - 56.0).max(60.0);
                        let (bar_rect, bar_resp) = ui.allocate_exact_size(
                            Vec2::new(available_w, 10.0),
                            egui::Sense::click_and_drag(),
                        );

                        let bg_fill = if *theme == AppTheme::Dark {
                            Color32::from_rgb(32, 40, 52)
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

                        // Seek interactivo con clic o arrastre
                        if bar_resp.clicked() || bar_resp.dragged() {
                            if let Some(pos) = bar_resp.interact_pointer_pos() {
                                let click_ratio =
                                    ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                                let seek_sec = click_ratio * self.total_duration_sec;
                                self.seek(seek_sec);
                            }
                        }

                        ui.label(
                            RichText::new(total_str)
                                .size(12.0)
                                .monospace()
                                .color(theme.text_secondary()),
                        );
                    });

                    ui.add_space(8.0);

                    // Controles de reproducción con iconos de Material Design (egui_icons)
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        let btn_size = Vec2::new(36.0, 28.0);

                        // Botón Reproducir / Pausa
                        let play_icon = if self.is_playing {
                            icons::ICON_PAUSE.codepoint
                        } else {
                            icons::ICON_PLAY_ARROW.codepoint
                        };

                        let play_btn = egui::Button::new(
                            RichText::new(play_icon)
                                .size(16.0)
                                .color(if self.is_playing {
                                    theme.accent_blue()
                                } else {
                                    Color32::WHITE
                                }),
                        )
                        .fill(if self.is_playing {
                            theme.surface_subtle()
                        } else {
                            theme.accent_blue()
                        })
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .corner_radius(6.0)
                        .min_size(btn_size);

                        if ui.add(play_btn).clicked() {
                            if self.is_playing {
                                self.pause_playback();
                            } else {
                                self.start_playback();
                            }
                        }

                        // Botón Detener (Garantía de apagado y reset)
                        let stop_btn = egui::Button::new(
                            RichText::new(icons::ICON_STOP.codepoint)
                                .size(16.0)
                                .color(theme.text_secondary()),
                        )
                        .fill(theme.surface_subtle())
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .corner_radius(6.0)
                        .min_size(btn_size);

                        if ui.add(stop_btn).clicked() {
                            self.stop_playback();
                        }

                        // Botón Abrir en Reproductor Predeterminado (solo Desktop nativo)
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(path) = self.loaded_path.as_deref().or(audio_path) {
                            ui.add_space(8.0);
                            let open_btn_text = format!(
                                "{} Abrir en Reproductor Predeterminado",
                                icons::ICON_OPEN_IN_NEW.codepoint
                            );
                            let open_player_btn = egui::Button::new(
                                RichText::new(open_btn_text).size(12.0),
                            )
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0)
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
    let total_s = sec.max(0.0).floor() as u32;
    let mins = total_s / 60;
    let secs = total_s % 60;
    format!("{:02}:{:02}", mins, secs)
}
