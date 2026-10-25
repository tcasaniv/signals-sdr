/// Modal de visualización de imagen a pantalla completa con zoom, pan y descarga.
/// Funciona tanto en Desktop como en WASM (web).
use crate::theme::AppTheme;
use egui::{Color32, Frame, Pos2, Rect, RichText, Stroke, Vec2};

pub struct ImageViewerModal {
    /// Si el modal está visible
    pub open: bool,
    /// Título descriptivo mostrado en el encabezado
    pub title: String,
    /// Datos crudos de imagen (PNG/JPG) para la descarga directa en web
    pub image_bytes: Option<Vec<u8>>,
    /// Nombre sugerido para la descarga
    pub download_name: String,
    /// Nivel de zoom (1.0 = 100%)
    zoom: f32,
    /// Desplazamiento de panorama
    pan: Vec2,
    /// Textura egui cacheada para renderizado
    texture: Option<egui::TextureHandle>,
    /// Datos de imagen decodificados para cargar la textura
    pending_color_image: Option<egui::ColorImage>,
}

impl Default for ImageViewerModal {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageViewerModal {
    pub fn new() -> Self {
        Self {
            open: false,
            title: "Vista de Imagen".to_string(),
            image_bytes: None,
            download_name: "imagen.png".to_string(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            texture: None,
            pending_color_image: None,
        }
    }

    /// Abre el modal con una `egui::ColorImage` y bytes opcionales para descarga.
    pub fn open_with_color_image(
        &mut self,
        title: impl Into<String>,
        color_image: egui::ColorImage,
        download_name: impl Into<String>,
        image_bytes: Option<Vec<u8>>,
    ) {
        self.open = true;
        self.title = title.into();
        self.download_name = download_name.into();
        self.image_bytes = image_bytes;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.texture = None;
        self.pending_color_image = Some(color_image);
    }

    /// Cierra el modal y libera la textura.
    pub fn close(&mut self) {
        self.open = false;
        self.texture = None;
        self.pending_color_image = None;
    }

    /// Renderiza el modal. Debe llamarse al final del frame para que quede encima de todo.
    pub fn show(&mut self, ctx: &egui::Context, theme: &AppTheme) {
        if !self.open {
            return;
        }

        // Cargar textura si hay imagen pendiente
        if let Some(color_image) = self.pending_color_image.take() {
            self.texture = Some(ctx.load_texture(
                &self.title,
                color_image,
                egui::TextureOptions::LINEAR,
            ));
        }

        // Overlay oscuro semitransparente detrás del modal
        let screen_rect = ctx.input(|i| i.viewport().inner_rect.unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))));
        egui::Area::new(egui::Id::new("image_modal_overlay"))
            .fixed_pos(Pos2::ZERO)
            .order(egui::Order::Background)
            .interactable(true)
            .show(ctx, |ui| {
                let overlay_resp = ui.allocate_rect(screen_rect, egui::Sense::click());
                ui.painter()
                    .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(180));
                // Cerrar al hacer clic fuera del modal
                if overlay_resp.clicked() {
                    self.open = false;
                }
            });

        // Modal principal
        let modal_width = (screen_rect.width() * 0.92).min(1200.0);
        let modal_height = (screen_rect.height() * 0.90).min(800.0);

        egui::Area::new(egui::Id::new("image_modal_window"))
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
                    .inner_margin(0.0)
                    .shadow(egui::Shadow {
                        offset: [0, 4].into(),
                        blur: 16,
                        spread: 0,
                        color: Color32::from_black_alpha(120),
                    })
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(modal_width, modal_height));
                        ui.set_max_size(Vec2::new(modal_width, modal_height));

                        // --- Encabezado ---
                        let is_narrow = modal_width < 550.0;
                        let header_h = if is_narrow { 78.0 } else { 52.0 };

                        Frame::new()
                            .fill(theme.surface_subtle())
                            .inner_margin(egui::Margin::symmetric(14, 8))
                            .corner_radius(12.0)
                            .show(ui, |ui| {
                                if is_narrow {
                                    ui.vertical(|ui| {
                                        // Fila 1: Título y botón cerrar
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} {}",
                                                    crate::icons::ICON_IMAGE.codepoint,
                                                    self.title
                                                ))
                                                .strong()
                                                .size(13.5)
                                                .color(theme.text_primary()),
                                            );

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let close_btn = egui::Button::new(
                                                    RichText::new(crate::icons::ICON_CLOSE.codepoint).size(16.0).color(theme.text_secondary()),
                                                )
                                                .fill(Color32::TRANSPARENT)
                                                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(30.0, 26.0));
                                                if ui.add(close_btn).clicked() {
                                                    self.open = false;
                                                }
                                            });
                                        });

                                        ui.add_space(4.0);

                                        // Fila 2: Descarga y controles de zoom
                                        ui.horizontal(|ui| {
                                            if self.image_bytes.is_some() {
                                                let dl_label = format!("{} Descargar", crate::icons::ICON_DOWNLOAD.codepoint);
                                                let dl_btn = egui::Button::new(
                                                    RichText::new(dl_label).size(11.5).color(Color32::WHITE),
                                                )
                                                .fill(theme.accent_blue())
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(0.0, 26.0));
                                                if ui.add(dl_btn).clicked() {
                                                    self.trigger_download();
                                                }
                                                ui.add_space(4.0);
                                            }

                                            let zoom_out_label = crate::icons::ICON_ZOOM_OUT.codepoint.to_string();
                                            let zoom_out = egui::Button::new(RichText::new(zoom_out_label).size(13.0))
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(26.0, 26.0));
                                            if ui.add(zoom_out).clicked() {
                                                self.zoom = (self.zoom / 1.25).max(0.1);
                                            }

                                            ui.label(
                                                RichText::new(format!("{:.0}%", self.zoom * 100.0))
                                                    .size(11.5)
                                                    .monospace()
                                                    .color(theme.text_secondary()),
                                            );

                                            let zoom_in_label = crate::icons::ICON_ZOOM_IN.codepoint.to_string();
                                            let zoom_in = egui::Button::new(RichText::new(zoom_in_label).size(13.0))
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(26.0, 26.0));
                                            if ui.add(zoom_in).clicked() {
                                                self.zoom = (self.zoom * 1.25).min(8.0);
                                            }

                                            let fit_label = format!("{} Ajustar", crate::icons::ICON_FIT_SCREEN.codepoint);
                                            let fit_btn = egui::Button::new(RichText::new(fit_label).size(11.5))
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(0.0, 26.0));
                                            if ui.add(fit_btn).clicked() {
                                                self.zoom = 1.0;
                                                self.pan = Vec2::ZERO;
                                            }
                                        });
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} {}",
                                                crate::icons::ICON_IMAGE.codepoint,
                                                self.title
                                            ))
                                            .strong()
                                            .size(14.0)
                                            .color(theme.text_primary()),
                                        );

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                // Botón cerrar
                                                let close_label = format!(
                                                    "{} Cerrar",
                                                    crate::icons::ICON_CLOSE.codepoint
                                                );
                                                let close_btn = egui::Button::new(
                                                    RichText::new(close_label)
                                                        .size(13.0)
                                                        .color(theme.text_secondary()),
                                                )
                                                .fill(Color32::TRANSPARENT)
                                                .stroke(Stroke::new(
                                                    1.0_f32,
                                                    theme.card_border(),
                                                ))
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(0.0, 28.0));
                                                if ui.add(close_btn).clicked() {
                                                    self.open = false;
                                                }

                                                // Botón descargar
                                                if self.image_bytes.is_some() {
                                                    ui.add_space(6.0);
                                                    let dl_label = format!(
                                                        "{} Descargar",
                                                        crate::icons::ICON_DOWNLOAD.codepoint
                                                    );
                                                    let dl_btn = egui::Button::new(
                                                        RichText::new(dl_label)
                                                            .size(13.0)
                                                            .color(Color32::WHITE),
                                                    )
                                                    .fill(theme.accent_blue())
                                                    .corner_radius(6.0)
                                                    .min_size(Vec2::new(0.0, 28.0));
                                                    if ui.add(dl_btn).clicked() {
                                                        self.trigger_download();
                                                    }
                                                }

                                                // Controles de zoom
                                                ui.add_space(12.0);
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Zoom: {:.0}%",
                                                        self.zoom * 100.0
                                                    ))
                                                    .size(12.0)
                                                    .color(theme.text_secondary()),
                                                );

                                                let zoom_in_label = crate::icons::ICON_ZOOM_IN.codepoint.to_string();
                                                let zoom_in = egui::Button::new(
                                                    RichText::new(zoom_in_label).size(15.0),
                                                )
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(30.0, 28.0));
                                                if ui.add(zoom_in).clicked() {
                                                    self.zoom = (self.zoom * 1.25).min(8.0);
                                                }

                                                let zoom_out_label = crate::icons::ICON_ZOOM_OUT.codepoint.to_string();
                                                let zoom_out = egui::Button::new(
                                                    RichText::new(zoom_out_label).size(15.0),
                                                )
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(30.0, 28.0));
                                                if ui.add(zoom_out).clicked() {
                                                    self.zoom = (self.zoom / 1.25).max(0.1);
                                                }

                                                let fit_label = format!(
                                                    "{} Ajustar",
                                                    crate::icons::ICON_FIT_SCREEN.codepoint
                                                );
                                                let fit_btn = egui::Button::new(
                                                    RichText::new(fit_label).size(12.0),
                                                )
                                                .corner_radius(6.0)
                                                .min_size(Vec2::new(0.0, 28.0));
                                                if ui.add(fit_btn).clicked() {
                                                    self.zoom = 1.0;
                                                    self.pan = Vec2::ZERO;
                                                }
                                            },
                                        );
                                    });
                                }
                            });

                        // --- Área de visualización ---
                        let avail_h = modal_height - header_h;
                        egui::ScrollArea::both()
                            .id_salt("image_modal_scroll")
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(modal_width, avail_h));

                                if let Some(tex) = &self.texture {
                                    let tex_size = tex.size_vec2();
                                    let container =
                                        Vec2::new(modal_width - 16.0, avail_h - 16.0);

                                    // Zoom-to-fit inicial
                                    let fit_zoom = (container.x / tex_size.x)
                                        .min(container.y / tex_size.y)
                                        .min(1.0);
                                    let effective_zoom = self.zoom * fit_zoom;

                                    let display_size = tex_size * effective_zoom;

                                    // Centrar la imagen
                                    let offset_x =
                                        ((container.x - display_size.x) * 0.5).max(0.0);
                                    let offset_y =
                                        ((container.y - display_size.y) * 0.5).max(0.0);

                                    ui.add_space(offset_y);
                                    ui.horizontal(|ui| {
                                        ui.add_space(offset_x);

                                        let img_resp = ui.add(
                                            egui::Image::new(tex)
                                                .fit_to_exact_size(display_size)
                                                .sense(egui::Sense::click_and_drag()),
                                        );

                                        // Zoom con rueda del ratón
                                        let scroll_delta =
                                            ui.ctx().input(|i| i.smooth_scroll_delta.y);
                                        if img_resp.hovered() && scroll_delta.abs() > 0.5 {
                                            self.zoom =
                                                (self.zoom * (1.0 + scroll_delta * 0.005))
                                                    .clamp(0.1, 8.0);
                                        }

                                        // Pan con arrastre
                                        if img_resp.dragged() {
                                            self.pan += img_resp.drag_delta();
                                        }
                                    });
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(
                                            RichText::new("Sin imagen disponible")
                                                .size(16.0)
                                                .color(theme.text_secondary()),
                                        );
                                    });
                                }
                            });
                    });
            });
    }

    /// Descarga el archivo de imagen en el navegador (WASM) o abre diálogo nativo en Desktop.
    fn trigger_download(&self) {
        if let Some(bytes) = &self.image_bytes {
            let _ = crate::file_dialog::save_or_download_file(
                &self.download_name,
                "Imagen (*.png;*.jpg;*.jpeg)",
                &["png", "jpg", "jpeg"],
                "image/png",
                bytes,
            );
        }
    }
}
