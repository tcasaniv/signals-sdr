use crate::theme::AppTheme;
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};

/// Convierte una imagen en escala de grises de `image` a un `egui::ColorImage`
pub fn gray_image_to_color_image(img: &image::GrayImage) -> egui::ColorImage {
    let size = [img.width() as usize, img.height() as usize];
    egui::ColorImage::from_gray(size, img.as_raw())
}

/// Convierte una `image::DynamicImage` a un `egui::ColorImage`
pub fn dynamic_image_to_color_image(img: &image::DynamicImage) -> egui::ColorImage {
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
}

/// Convierte una ruta de archivo local o URL web en un URI compatible con egui::Image
pub fn path_to_uri(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") || path.starts_with("file://") {
        path.to_string()
    } else {
        format!("file://{}", path.replace('\\', "/"))
    }
}

/// Tarjeta informativa y pedagógica para explicar el protocolo/sistema de modulación
pub fn info_card(ui: &mut Ui, theme: &AppTheme, title: &str, description: &str) {
    Frame::new()
        .fill(theme.info_card_bg())
        .stroke(Stroke::new(1.0_f32, theme.info_card_border()))
        .corner_radius(10.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 5.0, theme.accent_blue());
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(title)
                                .strong()
                                .size(15.5)
                                .color(theme.accent_blue()),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(6.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(description)
                            .size(13.0)
                            .color(theme.text_secondary()),
                    )
                    .wrap(),
                );
            });
        });
}

/// Selector / Barra de navegación de 4 pasos (1. Elegir fuente, 2. Acción, 3. Efectos de canal, 4. Transmisión)
pub fn step_bar(
    ui: &mut Ui,
    theme: &AppTheme,
    current_step: &mut usize,
    step_two_label: &str,
) {
    let clean_step_two = step_two_label
        .trim_start_matches("2.")
        .trim();

    let steps = [
        ("1", "Elegir fuente"),
        ("2", clean_step_two),
        ("3", "Efectos de canal"),
        ("4", "Transmisión"),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        for (idx, (num, label)) in steps.iter().enumerate() {
            if idx > 0 {
                ui.label(RichText::new("->").size(13.0).color(theme.text_secondary()));
            }

            let is_selected = *current_step == idx;
            let text_color = if is_selected {
                Color32::WHITE
            } else {
                theme.text_secondary()
            };

            let bg_color = if is_selected {
                theme.accent_blue()
            } else if *theme == AppTheme::Dark {
                Color32::from_rgb(26, 32, 42)
            } else {
                Color32::from_rgb(238, 242, 248)
            };

            let pill_btn = egui::Button::new(
                RichText::new(format!("  {} - {}  ", num, label))
                    .size(13.0)
                    .strong()
                    .color(text_color),
            )
            .fill(bg_color)
            .stroke(if is_selected {
                Stroke::NONE
            } else {
                Stroke::new(1.0_f32, theme.card_border())
            })
            .corner_radius(16.0);

            if ui.add(pill_btn).clicked() {
                *current_step = idx;
            }
        }
    });

    ui.add_space(8.0);
    let line_rect = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [line_rect.left_top(), egui::Pos2::new(line_rect.right(), line_rect.top())],
        Stroke::new(1.0_f32, theme.card_border()),
    );
    ui.add_space(8.0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelColor {
    Blue,
    Green,
    Neutral,
}

/// Tarjeta para cargar imagen con vista previa interactiva real, fila de URL y botón de archivo local
pub fn image_input_card(
    ui: &mut Ui,
    theme: &AppTheme,
    title: &str,
    current_path: &mut Option<String>,
    url_input: &mut String,
    color: ChannelColor,
) {
    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(10.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Cabecera: Punto de color distintivo + Título + Botón Quitar si existe imagen
                ui.horizontal(|ui| {
                    let dot_color = match color {
                        ChannelColor::Blue => Color32::from_rgb(37, 99, 235),
                        ChannelColor::Green => Color32::from_rgb(16, 185, 129),
                        ChannelColor::Neutral => theme.accent_blue(),
                    };
                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 4.5, dot_color);
                    ui.add_space(4.0);

                    ui.label(RichText::new(title).strong().size(14.5).color(theme.text_primary()));

                    if current_path.is_some() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("x Quitar").size(11.5).color(theme.accent_red())).clicked() {
                                *current_path = None;
                            }
                        });
                    }
                });

                ui.add_space(10.0);

                // Cuadro de imagen real o zona de carga estilizada
                if let Some(ref path) = current_path {
                    let uri = path_to_uri(path);
                    let preview_height = 150.0;

                    let img_box = Frame::new()
                        .fill(if *theme == AppTheme::Dark { Color32::from_rgb(16, 20, 26) } else { Color32::from_rgb(241, 245, 249) })
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .corner_radius(8.0)
                        .inner_margin(6.0);

                    let img_resp = img_box.show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            let img_widget = egui::Image::new(&uri)
                                .max_height(preview_height)
                                .corner_radius(6.0)
                                .sense(egui::Sense::click());
                            ui.add(img_widget)
                        }).inner
                    }).inner;

                    if img_resp.clicked() {
                        crate::utils::open_path_or_url(path);
                    }
                    if img_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        img_resp.on_hover_text("Clic para abrir imagen en el visor del sistema");
                    }

                    ui.add_space(6.0);

                    // Fila de metadatos del archivo y botón para abrir
                    ui.horizontal(|ui| {
                        let file_name = std::path::Path::new(path)
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.clone());

                        let short_name = if file_name.len() > 28 {
                            format!("{}...{}", &file_name[..16], &file_name[file_name.len() - 8..])
                        } else {
                            file_name
                        };

                        ui.label(RichText::new(format!("Archivo: {}", short_name)).size(12.0).color(theme.text_secondary()));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let open_btn = egui::Button::new(RichText::new("Abrir en visor").size(11.5));
                            if ui.add(open_btn).clicked() {
                                crate::utils::open_path_or_url(path);
                            }
                        });
                    });
                } else {
                    // Zona interactiva cuando no hay imagen cargada
                    let drop_height = 135.0;
                    let (rect, drop_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), drop_height),
                        egui::Sense::click(),
                    );

                    let is_hovered = drop_resp.hovered();
                    if is_hovered {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    let drop_bg = if is_hovered {
                        if *theme == AppTheme::Dark { Color32::from_rgb(32, 40, 52) } else { Color32::from_rgb(235, 244, 255) }
                    } else if *theme == AppTheme::Dark {
                        Color32::from_rgb(20, 25, 34)
                    } else {
                        Color32::from_rgb(247, 250, 253)
                    };

                    let border_stroke = if is_hovered {
                        Stroke::new(1.5_f32, theme.accent_blue())
                    } else {
                        Stroke::new(1.0_f32, theme.card_border())
                    };

                    ui.painter().rect(rect, 8.0, drop_bg, border_stroke, egui::StrokeKind::Inside);

                    ui.painter().text(
                        rect.center() - egui::vec2(0.0, 16.0),
                        egui::Align2::CENTER_CENTER,
                        format!("Seleccionar {}", title),
                        egui::FontId::proportional(14.0),
                        if is_hovered { theme.accent_blue() } else { theme.text_primary() },
                    );
                    ui.painter().text(
                        rect.center() + egui::vec2(0.0, 10.0),
                        egui::Align2::CENTER_CENTER,
                        "Haz clic para buscar archivo local",
                        egui::FontId::proportional(11.5),
                        theme.text_secondary(),
                    );

                    if drop_resp.clicked() {
                        if let Some(file) = rfd::FileDialog::new()
                            .add_filter("Imágenes", &["png", "jpg", "jpeg", "bmp", "webp"])
                            .pick_file()
                        {
                            *current_path = Some(file.display().to_string());
                        }
                    }
                }

                ui.add_space(10.0);

                // Fila de URL y botón Cargar URL con padding estilizado
                ui.horizontal(|ui| {
                    let btn_w = 90.0;
                    let avail = ui.available_width();
                    let edit_w = (avail - btn_w - 8.0).max(100.0);

                    ui.add(
                        egui::TextEdit::singleline(url_input)
                            .hint_text(format!("URL {}", title))
                            .desired_width(edit_w),
                    );

                    let url_btn = egui::Button::new(RichText::new("Cargar URL").size(12.0))
                        .min_size(Vec2::new(btn_w, 28.0));
                    if ui.add(url_btn).clicked() {
                        if !url_input.trim().is_empty() {
                            *current_path = Some(url_input.trim().to_string());
                        }
                    }
                });

                ui.add_space(6.0);

                // Botón de ancho completo para Archivo Local con altura cómoda
                let local_btn = egui::Button::new(
                    RichText::new(format!("Explorar Archivo Local ({})", title)).strong().size(12.5),
                )
                .fill(theme.surface_subtle())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .min_size(Vec2::new(ui.available_width(), 32.0));

                if ui.add(local_btn).clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Imágenes", &["png", "jpg", "jpeg", "bmp", "webp"])
                        .pick_file()
                    {
                        *current_path = Some(file.display().to_string());
                    }
                }
            });
        });
}

/// Tarjeta interactiva para productos intermedios del pipeline con renderizado de imagen real
pub fn intermediate_preview_card(
    ui: &mut Ui,
    theme: &AppTheme,
    title: &str,
    description: &str,
    file_path: Option<&std::path::Path>,
    is_ready: bool,
    color: ChannelColor,
    color_image: Option<&egui::ColorImage>,
) {
    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(10.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Cabecera con título e indicador de estado tipo badge
                ui.horizontal(|ui| {
                    let dot_color = match color {
                        ChannelColor::Blue => Color32::from_rgb(37, 99, 235),
                        ChannelColor::Green => Color32::from_rgb(16, 185, 129),
                        ChannelColor::Neutral => theme.accent_blue(),
                    };
                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 4.5, dot_color);
                    ui.add_space(4.0);

                    ui.label(RichText::new(title).strong().size(14.0).color(theme.text_primary()));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (badge_text, badge_color) = if is_ready {
                            ("LISTO", Color32::from_rgb(34, 197, 94))
                        } else {
                            ("PENDIENTE", Color32::from_rgb(156, 163, 175))
                        };
                        ui.label(RichText::new(badge_text).size(11.0).strong().color(badge_color));
                    });
                });

                ui.add_space(4.0);
                ui.label(RichText::new(description).size(11.5).color(theme.text_secondary()));
                ui.add_space(8.0);

                // Cuadro interactivo con renderizado de imagen real si está generada
                if is_ready && (color_image.is_some() || file_path.is_some()) {
                    let path_str_opt = file_path.map(|p| p.to_string_lossy().to_string());
                    let preview_height = 140.0;

                    let img_box = Frame::new()
                        .fill(if *theme == AppTheme::Dark { Color32::from_rgb(16, 20, 26) } else { Color32::from_rgb(241, 245, 249) })
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .corner_radius(8.0)
                        .inner_margin(6.0);

                    let img_resp = img_box.show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            if let Some(ci) = color_image {
                                let texture = ui.ctx().load_texture(
                                    format!("interm_tex_{}", title),
                                    ci.clone(),
                                    egui::TextureOptions::LINEAR,
                                );
                                let img_widget = egui::Image::new(&texture)
                                    .max_height(preview_height)
                                    .corner_radius(6.0)
                                    .sense(egui::Sense::click());
                                ui.add(img_widget)
                            } else if let Some(ref path_str) = path_str_opt {
                                let uri = path_to_uri(path_str);
                                let img_widget = egui::Image::new(&uri)
                                    .max_height(preview_height)
                                    .corner_radius(6.0)
                                    .sense(egui::Sense::click());
                                ui.add(img_widget)
                            } else {
                                ui.label("Imagen disponible")
                            }
                        }).inner
                    }).inner;

                    if let Some(ref path_str) = path_str_opt {
                        if img_resp.clicked() {
                            crate::utils::open_path_or_url(path_str);
                        }
                        if img_resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            img_resp.on_hover_text("Clic para abrir imagen en el visor del sistema");
                        }

                        ui.add_space(8.0);

                        let open_btn = egui::Button::new(RichText::new("Abrir Imagen en Visor Predeterminado").size(12.0))
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .min_size(Vec2::new(ui.available_width(), 28.0));

                        if ui.add(open_btn).clicked() {
                            crate::utils::open_path_or_url(path_str);
                        }
                    }
                } else {
                    // Placeholder visual cuando la fase aún no se ha ejecutado
                    let pending_height = 80.0;
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), pending_height),
                        egui::Sense::hover(),
                    );

                    let bg = if *theme == AppTheme::Dark { Color32::from_rgb(20, 24, 32) } else { Color32::from_rgb(245, 247, 250) };
                    ui.painter().rect(rect, 8.0, bg, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);

                    ui.painter().text(
                        rect.center() - egui::vec2(0.0, 8.0),
                        egui::Align2::CENTER_CENTER,
                        "...",
                        egui::FontId::proportional(20.0),
                        theme.text_secondary(),
                    );
                    ui.painter().text(
                        rect.center() + egui::vec2(0.0, 12.0),
                        egui::Align2::CENTER_CENTER,
                        "Pendiente de ejecución en esta fase",
                        egui::FontId::proportional(11.5),
                        theme.text_secondary(),
                    );
                }
            });
        });
}

/// Opciones de configuración para una sección desplegable tipo Google Slides
pub struct AccordionOptions<'a> {
    pub title: &'a str,
    pub badge: Option<(&'a str, Color32)>,
    pub toggle_enabled: Option<&'a mut bool>,
    pub default_open: bool,
}

impl<'a> AccordionOptions<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            badge: None,
            toggle_enabled: None,
            default_open: true,
        }
    }

    pub fn with_toggle(mut self, enabled: &'a mut bool) -> Self {
        self.toggle_enabled = Some(enabled);
        self
    }

    pub fn with_badge(mut self, text: &'a str, color: Color32) -> Self {
        self.badge = Some((text, color));
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
}

/// Sección desplegable tipo acordeón con estética de "Opciones de Formato" de Google Slides:
/// - Chevron a la izquierda (rotación suave al expandir/colapsar)
/// - Título destacado y badge informativo opcional
/// - Checkbox independiente a la derecha para activar/desactivar (si aplica)
/// - Clic en la cabecera abre/cierra; clic en el checkbox activa/desactiva sin forzar colapso
pub fn accordion_section<R>(
    ui: &mut Ui,
    theme: &AppTheme,
    id_salt: impl std::hash::Hash,
    options: AccordionOptions,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let id = ui.make_persistent_id(id_salt);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, options.default_open);
    let is_open = state.is_open();

    let mut result = None;

    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            let row_height = 28.0;
            let full_avail_w = ui.available_width();

            let (header_rect, header_resp) = ui.allocate_exact_size(
                Vec2::new(full_avail_w, row_height),
                egui::Sense::click(),
            );

            let is_hovered = header_resp.hovered();
            if is_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                let hover_bg = if *theme == AppTheme::Dark {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 6)
                } else {
                    Color32::from_rgba_unmultiplied(0, 0, 0, 6)
                };
                ui.painter().rect_filled(header_rect, 6.0, hover_bg);
            }

            // Chevron a la izquierda
            let chevron_center = egui::pos2(header_rect.left() + 12.0, header_rect.center().y);
            let chevron_stroke = Stroke::new(1.8_f32, if is_hovered { theme.accent_blue() } else { theme.text_secondary() });
            if is_open {
                // Chevron hacia abajo '⌵'
                ui.painter().line_segment([
                    chevron_center + egui::vec2(-4.5, -2.5),
                    chevron_center + egui::vec2(0.0, 2.5),
                ], chevron_stroke);
                ui.painter().line_segment([
                    chevron_center + egui::vec2(0.0, 2.5),
                    chevron_center + egui::vec2(4.5, -2.5),
                ], chevron_stroke);
            } else {
                // Chevron hacia la derecha '›'
                ui.painter().line_segment([
                    chevron_center + egui::vec2(-2.5, -4.5),
                    chevron_center + egui::vec2(2.5, 0.0),
                ], chevron_stroke);
                ui.painter().line_segment([
                    chevron_center + egui::vec2(2.5, 0.0),
                    chevron_center + egui::vec2(-2.5, 4.5),
                ], chevron_stroke);
            }

            // Título
            let title_pos = egui::pos2(header_rect.left() + 26.0, header_rect.center().y);
            let title_galley = ui.painter().layout_no_wrap(
                options.title.to_string(),
                egui::FontId::proportional(13.5),
                theme.text_primary(),
            );
            let title_w = title_galley.size().x;
            ui.painter().galley(
                egui::pos2(title_pos.x, title_pos.y - title_galley.size().y / 2.0),
                title_galley,
                theme.text_primary(),
            );

            // Badge informativo opcional
            if let Some((badge_text, badge_color)) = options.badge {
                let badge_pos_x = title_pos.x + title_w + 12.0;
                let badge_galley = ui.painter().layout_no_wrap(
                    format!("[ {} ]", badge_text),
                    egui::FontId::proportional(11.5),
                    badge_color,
                );
                ui.painter().galley(
                    egui::pos2(badge_pos_x, header_rect.center().y - badge_galley.size().y / 2.0),
                    badge_galley,
                    badge_color,
                );
            }

            // Checkbox opcional a la derecha
            let mut checkbox_clicked = false;
            if let Some(enabled_ref) = options.toggle_enabled {
                let cb_size = 18.0;
                let cb_rect = egui::Rect::from_center_size(
                    egui::pos2(header_rect.right() - 14.0, header_rect.center().y),
                    Vec2::splat(cb_size),
                );

                let cb_resp = ui.interact(cb_rect, id.with("cb"), egui::Sense::click());
                if cb_resp.clicked() {
                    *enabled_ref = !*enabled_ref;
                    checkbox_clicked = true;
                }
                if cb_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                let is_checked = *enabled_ref;
                let cb_bg = if is_checked {
                    Color32::from_rgb(234, 179, 8) // Amarillo/Dorado Google Slides
                } else if cb_resp.hovered() {
                    theme.surface_subtle()
                } else {
                    Color32::TRANSPARENT
                };

                let cb_border = if is_checked {
                    Color32::from_rgb(202, 138, 4)
                } else {
                    theme.card_border()
                };

                ui.painter().rect(cb_rect, 3.5, cb_bg, Stroke::new(1.5_f32, cb_border), egui::StrokeKind::Inside);

                if is_checked {
                    let check_stroke = Stroke::new(2.2_f32, Color32::WHITE);
                    ui.painter().line_segment([
                        cb_rect.center() + egui::vec2(-4.0, 0.0),
                        cb_rect.center() + egui::vec2(-1.0, 3.5),
                    ], check_stroke);
                    ui.painter().line_segment([
                        cb_rect.center() + egui::vec2(-1.0, 3.5),
                        cb_rect.center() + egui::vec2(4.5, -3.5),
                    ], check_stroke);
                }
            }

            // Si se hace clic en la cabecera (fuera del checkbox), alternar apertura
            if header_resp.clicked() && !checkbox_clicked {
                state.toggle(ui);
            }

            // Contenido interior si está desplegado
            state.show_body_unindented(ui, |ui| {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                result = Some(add_contents(ui));
            });
        });

    result
}
