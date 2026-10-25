/// Modal "Archivos Generados" que reemplaza el botón "Abrir Carpeta de Salida" en web.
/// Muestra los archivos generados con opciones de descarga, apertura inline (audio/imagen) y reproductor interactivo.
use crate::theme::AppTheme;
use crate::ui::audio_player::AudioPlayerWidget;
use crate::ui::image_modal::ImageViewerModal;
use egui::{Color32, Frame, Pos2, RichText, Stroke, Vec2};

/// Tipo de archivo generado
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFileType {
    Image,
    Audio,
    Binary,
}

/// Un archivo generado con sus bytes en memoria
#[derive(Clone)]
pub struct OutputFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub kind: OutputFileType,
    pub description: String,
}

impl OutputFile {
    pub fn image(name: impl Into<String>, bytes: Vec<u8>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bytes,
            kind: OutputFileType::Image,
            description: description.into(),
        }
    }

    pub fn audio(name: impl Into<String>, bytes: Vec<u8>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bytes,
            kind: OutputFileType::Audio,
            description: description.into(),
        }
    }

    pub fn binary(name: impl Into<String>, bytes: Vec<u8>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bytes,
            kind: OutputFileType::Binary,
            description: description.into(),
        }
    }

    fn icon(&self) -> egui_icons::MaterialIcon {
        match self.kind {
            OutputFileType::Image => crate::icons::ICON_IMAGE,
            OutputFileType::Audio => crate::icons::ICON_AUDIOTRACK,
            OutputFileType::Binary => crate::icons::ICON_DESCRIPTION,
        }
    }

    #[allow(dead_code)]
    fn mime_type(&self) -> &'static str {
        match self.kind {
            OutputFileType::Image => "image/png",
            OutputFileType::Audio => "audio/wav",
            OutputFileType::Binary => "application/octet-stream",
        }
    }

    fn size_label(&self) -> String {
        let bytes = self.bytes.len();
        if bytes >= 1_048_576 {
            format!("{:.1} MB", bytes as f64 / 1_048_576.0)
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }
}

pub struct OutputFilesModal {
    pub open: bool,
    pub title: String,
    pub files: Vec<OutputFile>,
    pub audio_player: AudioPlayerWidget,
    /// Índice del archivo de audio actualmente seleccionado en el reproductor
    pub inline_audio_index: Option<usize>,
}

impl Default for OutputFilesModal {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFilesModal {
    pub fn new() -> Self {
        Self {
            open: false,
            title: "Archivos Generados".to_string(),
            files: Vec::new(),
            audio_player: AudioPlayerWidget::new(),
            inline_audio_index: None,
        }
    }

    /// Abre el modal con la lista de archivos
    pub fn open_with_files(&mut self, title: impl Into<String>, files: Vec<OutputFile>) {
        self.open = true;
        self.title = title.into();
        self.files = files;
        self.inline_audio_index = None;
        self.audio_player.stop_playback();
    }

    /// Agrega un archivo a la lista (útil para ir acumulando durante el pipeline)
    pub fn push_file(&mut self, file: OutputFile) {
        if let Some(existing) = self.files.iter_mut().find(|f| f.name == file.name) {
            *existing = file;
        } else {
            self.files.push(file);
        }
    }

    /// Sincroniza la lista con los archivos reales presentes en la carpeta de salida del pipeline
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sync_from_directory(&mut self, dir_path: &std::path::Path) {
        if !dir_path.exists() || !dir_path.is_dir() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("archivo").to_string();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

                    if !self.files.iter().any(|f| f.name == file_name) {
                        if let Ok(bytes) = std::fs::read(&path) {
                            let kind = match ext.as_str() {
                                "png" | "jpg" | "jpeg" | "bmp" => OutputFileType::Image,
                                "wav" | "mp3" | "flac" => OutputFileType::Audio,
                                _ => OutputFileType::Binary,
                            };
                            let desc = match ext.as_str() {
                                "wav" => "Señal de audio o modulación I/Q en disco",
                                "png" | "jpg" => "Imagen o espectrograma exportado en disco",
                                "bin" | "raw" => "Muestras de datos binarios en disco",
                                _ => "Archivo generado en disco",
                            };
                            self.files.push(OutputFile {
                                name: file_name,
                                bytes,
                                kind,
                                description: desc.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn sync_from_directory(&mut self, _dir_path: &std::path::Path) {}

    /// Renderiza el modal sobre toda la interfaz
    pub fn show(&mut self, ctx: &egui::Context, theme: &AppTheme, image_modal: &mut ImageViewerModal) {
        if !self.open {
            return;
        }

        let screen_rect = ctx.input(|i| {
            i.viewport()
                .inner_rect
                .unwrap_or(egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0)))
        });

        // Overlay oscuro
        egui::Area::new(egui::Id::new("output_modal_overlay"))
            .fixed_pos(Pos2::ZERO)
            .order(egui::Order::Background)
            .interactable(true)
            .show(ctx, |ui| {
                let overlay_resp = ui.allocate_rect(screen_rect, egui::Sense::click());
                ui.painter().rect_filled(screen_rect, 0.0, Color32::from_black_alpha(180));
                if overlay_resp.clicked() {
                    self.audio_player.stop_playback();
                    self.open = false;
                }
            });

        // Ventana modal
        let modal_width = (screen_rect.width() * 0.88).min(900.0);
        let modal_height = (screen_rect.height() * 0.80).min(600.0);

        egui::Area::new(egui::Id::new("output_modal_window"))
            .fixed_pos(Pos2::new(
                (screen_rect.width() - modal_width) * 0.5,
                (screen_rect.height() - modal_height) * 0.5,
            ))
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme.surface_container())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .corner_radius(12.0)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(140),
                    })
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(modal_width, modal_height));
                        ui.set_max_size(Vec2::new(modal_width, modal_height));

                        // Encabezado
                        Frame::new()
                            .fill(theme.surface_subtle())
                            .inner_margin(egui::Margin::symmetric(16, 10))
                            .corner_radius(12.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let header_text = format!(
                                        "{} {} ({} archivos)",
                                        crate::icons::ICON_FOLDER_OPEN.codepoint,
                                        self.title,
                                        self.files.len()
                                    );
                                    ui.label(
                                        RichText::new(header_text)
                                            .strong()
                                            .size(14.0)
                                            .color(theme.text_primary()),
                                    );
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let close_label = format!(
                                            "{} Cerrar",
                                            crate::icons::ICON_CLOSE.codepoint
                                        );
                                        let close_btn = egui::Button::new(
                                            RichText::new(close_label).size(13.0).color(theme.text_secondary()),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                        .corner_radius(6.0)
                                        .min_size(Vec2::new(0.0, 28.0));
                                        if ui.add(close_btn).clicked() {
                                            self.audio_player.stop_playback();
                                            self.open = false;
                                        }
                                    });
                                });
                            });

                        // Lista de archivos
                        egui::ScrollArea::vertical()
                            .id_salt("output_modal_scroll")
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                Frame::new()
                                    .inner_margin(egui::Margin::symmetric(16, 12))
                                    .show(ui, |ui| {
                                        ui.set_min_width(modal_width - 32.0);

                                        if self.files.is_empty() {
                                            ui.add_space(40.0);
                                            ui.centered_and_justified(|ui| {
                                                ui.label(
                                                    RichText::new("No hay archivos generados aún.")
                                                        .size(15.0)
                                                        .color(theme.text_secondary()),
                                                );
                                            });
                                            return;
                                        }

                                        let mut open_image_idx: Option<usize> = None;
                                        let mut download_idx: Option<usize> = None;
                                        let mut select_audio_idx: Option<usize> = None;

                                        for (idx, file) in self.files.iter().enumerate() {
                                            ui.add_space(4.0);
                                            Frame::new()
                                                .fill(theme.card_bg())
                                                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                                .corner_radius(8.0)
                                                .inner_margin(12.0)
                                                .show(ui, |ui| {
                                                    ui.horizontal_wrapped(|ui| {
                                                        // Icono Material + nombre
                                                        ui.label(
                                                            RichText::new(format!("{} {}", file.icon().codepoint, file.name))
                                                                .strong()
                                                                .size(13.5)
                                                                .color(theme.text_primary()),
                                                        );
                                                        ui.label(
                                                            RichText::new(format!("({})", file.size_label()))
                                                                .size(12.0)
                                                                .color(theme.text_secondary()),
                                                        );
                                                    });

                                                    if !file.description.is_empty() {
                                                        ui.label(
                                                            RichText::new(&file.description)
                                                                .size(12.0)
                                                                .color(theme.text_secondary()),
                                                        );
                                                    }

                                                    ui.add_space(6.0);
                                                    ui.horizontal_wrapped(|ui| {
                                                        // Botón Descargar (Material Design)
                                                        let dl_label = format!("{} Descargar", crate::icons::ICON_DOWNLOAD.codepoint);
                                                        let dl_btn = egui::Button::new(
                                                            RichText::new(dl_label).size(12.5).color(Color32::WHITE),
                                                        )
                                                        .fill(theme.accent_blue())
                                                        .corner_radius(6.0)
                                                        .min_size(Vec2::new(0.0, 28.0));
                                                        if ui.add(dl_btn).clicked() {
                                                            download_idx = Some(idx);
                                                        }

                                                        // Botón Ver (imágenes)
                                                        if file.kind == OutputFileType::Image {
                                                            ui.add_space(4.0);
                                                            let view_label = format!("{} Ver", crate::icons::ICON_VISIBILITY.codepoint);
                                                            let view_btn = egui::Button::new(
                                                                RichText::new(view_label).size(12.5),
                                                            )
                                                            .fill(theme.surface_subtle())
                                                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                                            .corner_radius(6.0)
                                                            .min_size(Vec2::new(0.0, 28.0));
                                                            if ui.add(view_btn).clicked() {
                                                                open_image_idx = Some(idx);
                                                            }
                                                        }

                                                        // Botón Reproductor (audio)
                                                        if file.kind == OutputFileType::Audio {
                                                            ui.add_space(4.0);
                                                            let is_current = self.inline_audio_index == Some(idx);
                                                            let btn_icon = if is_current && self.audio_player.is_playing {
                                                                crate::icons::ICON_PAUSE.codepoint
                                                            } else {
                                                                crate::icons::ICON_PLAY_ARROW.codepoint
                                                            };
                                                            let play_label = format!("{} Reproductor", btn_icon);
                                                            let play_btn = egui::Button::new(
                                                                RichText::new(play_label).size(12.5),
                                                            )
                                                            .fill(if is_current { theme.accent_blue() } else { theme.surface_subtle() })
                                                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                                            .corner_radius(6.0)
                                                            .min_size(Vec2::new(0.0, 28.0));
                                                            if ui.add(play_btn).clicked() {
                                                                select_audio_idx = Some(idx);
                                                            }
                                                        }
                                                    });

                                                    // Si es el audio seleccionado, mostrar reproductor integrado interactivo
                                                    if file.kind == OutputFileType::Audio && self.inline_audio_index == Some(idx) {
                                                        ui.add_space(8.0);
                                                        self.audio_player.render(ui, theme, None);
                                                    }
                                                });
                                        }

                                        // Procesar acciones recolectadas
                                        if let Some(idx) = download_idx {
                                            if let Some(file) = self.files.get(idx) {
                                                Self::download_file(file);
                                            }
                                        }

                                        if let Some(idx) = open_image_idx {
                                            if let Some(file) = self.files.get(idx) {
                                                if let Ok(img) = image::load_from_memory(&file.bytes) {
                                                    let rgba = img.to_rgba8();
                                                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                                        [rgba.width() as usize, rgba.height() as usize],
                                                        rgba.as_raw(),
                                                    );
                                                    image_modal.open_with_color_image(
                                                        &file.name,
                                                        color_img,
                                                        &file.name,
                                                        Some(file.bytes.clone()),
                                                    );
                                                }
                                            }
                                        }

                                        if let Some(idx) = select_audio_idx {
                                            if self.inline_audio_index == Some(idx) {
                                                if self.audio_player.is_playing {
                                                    self.audio_player.pause_playback();
                                                } else {
                                                    self.audio_player.start_playback();
                                                }
                                            } else {
                                                self.inline_audio_index = Some(idx);
                                                if let Some(file) = self.files.get(idx) {
                                                    self.audio_player.load_bytes(&file.name, file.bytes.clone());
                                                    self.audio_player.start_playback();
                                                }
                                            }
                                        }
                                    });
                            });
                    });
            });
    }

    fn download_file(file: &OutputFile) {
        let ext = file.name.rsplit('.').next().unwrap_or("bin");
        let filter_name = format!("Archivo {} (*.{})", ext.to_uppercase(), ext);
        let exts = [ext];
        let _ = crate::file_dialog::save_or_download_file(
            &file.name,
            &filter_name,
            &exts,
            file.mime_type(),
            &file.bytes,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_file_properties() {
        let file = OutputFile::image("test.png", vec![1, 2, 3], "Test image");
        assert_eq!(file.name, "test.png");
        assert_eq!(file.kind, OutputFileType::Image);
        assert_eq!(file.size_label(), "3 B");
    }

    #[test]
    fn test_push_and_deduplicate() {
        let mut modal = OutputFilesModal::new();
        modal.push_file(OutputFile::audio("audio.wav", vec![0; 100], "V1"));
        assert_eq!(modal.files.len(), 1);
        assert_eq!(modal.files[0].description, "V1");

        modal.push_file(OutputFile::audio("audio.wav", vec![0; 200], "V2"));
        assert_eq!(modal.files.len(), 1);
        assert_eq!(modal.files[0].description, "V2");
        assert_eq!(modal.files[0].bytes.len(), 200);
    }

    #[test]
    fn test_sync_directory_non_existent() {
        let mut modal = OutputFilesModal::new();
        let path = std::path::Path::new("non_existent_folder_xyz_1234");
        modal.sync_from_directory(path);
        assert_eq!(modal.files.len(), 0);
    }
}
