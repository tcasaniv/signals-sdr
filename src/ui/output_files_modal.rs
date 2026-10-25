/// Modal "Archivos Generados" que reemplaza el botón "Abrir Carpeta de Salida" en web.
/// Muestra los archivos generados con opciones de descarga, apertura inline (audio/imagen).
use crate::theme::AppTheme;
use crate::ui::image_modal::ImageViewerModal;
use egui::{Color32, Frame, Pos2, RichText, Stroke, Vec2};

/// Tipo de archivo generado
#[derive(Clone, PartialEq)]
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

    fn icon(&self) -> &'static str {
        match self.kind {
            OutputFileType::Image => "🖼",
            OutputFileType::Audio => "🎵",
            OutputFileType::Binary => "📄",
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
    /// Índice del archivo de audio seleccionado para reproducir inline (WASM)
    inline_audio_index: Option<usize>,
    /// URL del blob de audio activo (WASM)
    #[cfg(target_arch = "wasm32")]
    active_audio_blob_url: Option<String>,
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
            inline_audio_index: None,
            #[cfg(target_arch = "wasm32")]
            active_audio_blob_url: None,
        }
    }

    /// Abre el modal con la lista de archivos
    pub fn open_with_files(&mut self, title: impl Into<String>, files: Vec<OutputFile>) {
        self.open = true;
        self.title = title.into();
        self.files = files;
        self.inline_audio_index = None;
    }

    /// Agrega un archivo a la lista (útil para ir acumulando durante el pipeline)
    pub fn push_file(&mut self, file: OutputFile) {
        // Evitar duplicados por nombre
        if let Some(existing) = self.files.iter_mut().find(|f| f.name == file.name) {
            *existing = file;
        } else {
            self.files.push(file);
        }
    }

    /// Renderiza el modal sobre toda la interfaz
    pub fn show(&mut self, ctx: &egui::Context, theme: &AppTheme, image_modal: &mut ImageViewerModal) {
        if !self.open {
            return;
        }

        let screen_rect = ctx.input(|i| i.viewport().inner_rect.unwrap_or(egui::Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))));

        // Overlay oscuro
        egui::Area::new(egui::Id::new("output_modal_overlay"))
            .fixed_pos(Pos2::ZERO)
            .order(egui::Order::Background)
            .interactable(true)
            .show(ctx, |ui| {
                let overlay_resp = ui.allocate_rect(screen_rect, egui::Sense::click());
                ui.painter().rect_filled(screen_rect, 0.0, Color32::from_black_alpha(180));
                if overlay_resp.clicked() {
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
                    .fill(theme.surface_container_low())
                    .stroke(Stroke::new(1.5_f32, theme.accent_blue()))
                    .corner_radius(12.0)
                    .shadow(egui::Shadow {
                        offset: [0, 4].into(),
                        blur: 16,
                        spread: 0,
                        color: Color32::from_black_alpha(100),
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
                                    ui.label(
                                        RichText::new(format!("📁 {} ({} archivos)", self.title, self.files.len()))
                                            .strong()
                                            .size(14.0)
                                            .color(theme.text_primary()),
                                    );
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let close_btn = egui::Button::new(
                                            RichText::new("✕  Cerrar").size(13.0).color(theme.text_secondary()),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                        .corner_radius(6.0)
                                        .min_size(Vec2::new(0.0, 28.0));
                                        if ui.add(close_btn).clicked() {
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
                                ui.set_min_width(modal_width);

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
                                let mut play_audio_idx: Option<usize> = None;

                                for (idx, file) in self.files.iter().enumerate() {
                                    ui.add_space(4.0);
                                    Frame::new()
                                        .fill(theme.card_bg())
                                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                        .corner_radius(8.0)
                                        .inner_margin(12.0)
                                        .show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                // Icono + nombre
                                                ui.label(
                                                    RichText::new(format!("{} {}", file.icon(), file.name))
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
                                                // Botón Descargar (siempre disponible)
                                                let dl_btn = egui::Button::new(
                                                    RichText::new("⬇ Descargar").size(12.5).color(Color32::WHITE),
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
                                                    let view_btn = egui::Button::new(
                                                        RichText::new("🖼 Ver").size(12.5),
                                                    )
                                                    .fill(theme.surface_subtle())
                                                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                                    .corner_radius(6.0)
                                                    .min_size(Vec2::new(0.0, 28.0));
                                                    if ui.add(view_btn).clicked() {
                                                        open_image_idx = Some(idx);
                                                    }
                                                }

                                                // Botón Reproducir (audio)
                                                if file.kind == OutputFileType::Audio {
                                                    ui.add_space(4.0);
                                                    let is_playing = self.inline_audio_index == Some(idx);
                                                    let play_label = if is_playing { "⏹ Detener" } else { "▶ Reproducir" };
                                                    let play_btn = egui::Button::new(
                                                        RichText::new(play_label).size(12.5),
                                                    )
                                                    .fill(if is_playing { theme.surface_subtle() } else { theme.surface_subtle() })
                                                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                                    .corner_radius(6.0)
                                                    .min_size(Vec2::new(0.0, 28.0));
                                                    if ui.add(play_btn).clicked() {
                                                        play_audio_idx = Some(idx);
                                                    }
                                                }
                                            });
                                        });
                                }

                                // Procesar acciones recolectadas (evita borrow mutuo)
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

                                if let Some(idx) = play_audio_idx {
                                    if self.inline_audio_index == Some(idx) {
                                        // Detener
                                        self.inline_audio_index = None;
                                        self.stop_inline_audio();
                                    } else {
                                        // Reproducir
                                        self.inline_audio_index = Some(idx);
                                        let bytes_opt = self.files.get(idx).map(|f| f.bytes.clone());
                                        if let Some(bytes) = bytes_opt {
                                            self.play_inline_audio(&bytes, ctx);
                                        }
                                    }
                                }
                            });
                    });
            });
    }

    fn download_file(file: &OutputFile) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = crate::web_utils::trigger_browser_download(&file.name, &file.bytes, file.mime_type());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(dir) = dirs::download_dir().or_else(dirs::home_dir) {
                let path = dir.join(&file.name);
                let _ = std::fs::write(&path, &file.bytes);
            }
        }
    }

    fn play_inline_audio(&mut self, _bytes: &[u8], _ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        {
            // Revocar URL anterior si existe
            if let Some(old_url) = self.active_audio_blob_url.take() {
                web_sys::Url::revoke_object_url(&old_url).ok();
            }

            if let (Some(window), Ok(blob_url)) = (
                web_sys::window(),
                crate::web_utils::create_audio_blob_url(_bytes),
            ) {
                if let Some(_doc) = window.document() {
                    // Crear elemento <audio> y reproducirlo via JS eval
                    let script = format!(
                        r#"
                        (function() {{
                            var old = document.getElementById('_sdr_inline_audio');
                            if (old) {{ old.pause(); old.remove(); }}
                            var a = document.createElement('audio');
                            a.id = '_sdr_inline_audio';
                            a.src = '{}';
                            a.controls = false;
                            a.style.display = 'none';
                            document.body.appendChild(a);
                            a.play();
                        }})()
                        "#,
                        blob_url
                    );
                    let _ = js_sys::eval(&script);
                    self.active_audio_blob_url = Some(blob_url);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // En Desktop usamos rodio desde bytes
            let bytes_owned = _bytes.to_vec();
            std::thread::spawn(move || {
                use std::io::Cursor;
                if let Ok(device_sink) = rodio::DeviceSinkBuilder::open_default_sink() {
                    let player = rodio::Player::connect_new(&device_sink.mixer());
                    let cursor = Cursor::new(bytes_owned);
                    if let Ok(source) = rodio::Decoder::try_from(cursor) {
                        player.append(source);
                        // Esperar a que termine
                        while !player.empty() {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                }
            });
        }
    }

    fn stop_inline_audio(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(url) = self.active_audio_blob_url.take() {
                web_sys::Url::revoke_object_url(&url).ok();
            }
            let _ = js_sys::eval(
                r#"(function(){var a=document.getElementById('_sdr_inline_audio');if(a){a.pause();a.remove();}})()"#,
            );
        }
    }
}
