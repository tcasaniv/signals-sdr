use crate::theme::AppTheme;
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};
use std::sync::{Arc, Mutex};

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
    if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else if let Some(stripped) = path.strip_prefix("file:///") {
        format!("file:///{}", stripped.replace('\\', "/"))
    } else if let Some(stripped) = path.strip_prefix("file://") {
        // En Windows, file://C:/... interpreta 'C:' como host de red provocando 'os error 123'. Corregir a file:///
        format!("file:///{}", stripped.replace('\\', "/"))
    } else {
        let normalized = path.replace('\\', "/");
        if normalized.starts_with('/') {
            format!("file://{}", normalized)
        } else {
            format!("file:///{}", normalized)
        }
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

    let step_two_str = format!("2. {}", clean_step_two);
    let steps: [(egui_icons::MaterialIcon, &str); 4] = [
        (crate::icons::ICON_FOLDER_OPEN, "1. Elegir fuente"),
        (crate::icons::ICON_TUNE, &step_two_str),
        (crate::icons::ICON_WAVES, "3. Efectos de canal"),
        (crate::icons::ICON_SETTINGS_INPUT_ANTENNA, "4. Transmisión"),
    ];

    let avail_w = ui.available_width();
    if avail_w < 520.0 {
        // En pantallas móviles / estrechas, renderizar en cuadrícula 2x2 tipo control segmentado responsivo sin flechas rotas
        let col_w = ((avail_w - 6.0) / 2.0).max(120.0);
        for row in 0..2 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for col in 0..2 {
                    let idx = row * 2 + col;
                    let (icon, label) = steps[idx];
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

                    let btn = egui::Button::new(
                        RichText::new(format!("{} {}", icon.codepoint, label))
                            .size(12.0)
                            .strong()
                            .color(text_color),
                    )
                    .fill(bg_color)
                    .stroke(if is_selected {
                        Stroke::NONE
                    } else {
                        Stroke::new(1.0_f32, theme.card_border())
                    })
                    .corner_radius(8.0)
                    .min_size(Vec2::new(col_w, 32.0));

                    if ui.add(btn).clicked() {
                        *current_step = idx;
                    }
                }
            });
            if row == 0 {
                ui.add_space(4.0);
            }
        }
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            for (idx, (icon, label)) in steps.iter().enumerate() {
                if idx > 0 {
                    ui.label(RichText::new(crate::icons::ICON_ARROW_FORWARD.codepoint).size(13.0).color(theme.text_secondary()));
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
                    RichText::new(format!("  {} {}  ", icon.codepoint, label))
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
    }

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

/// Estructura para gestionar la carga y vista previa en memoria de imágenes (archivo local o URL)
/// compatible con WebAssembly y Desktop sin errores de lectura.
#[derive(Clone, Default)]
pub struct ImageSlot {
    /// Nombre para mostrar (ej: "satelite.png" o URL)
    pub display_name: Option<String>,
    /// Texto actual ingresado en la caja de URL
    pub url_input: String,
    /// Bytes crudos de la imagen
    pub bytes: Option<Vec<u8>>,
    /// Imagen decodificada para procesar en el pipeline
    pub dynamic_image: Option<image::DynamicImage>,
    /// Imagen egui para vista previa directa en memoria
    pub color_image: Option<egui::ColorImage>,
    /// Estado de carga asíncrona
    pub is_loading: bool,
    /// Mensaje de error si la carga o decodificación falló
    pub error_msg: Option<String>,
    /// Canal interno para recibir resultados asíncronos
    async_receiver: Option<Arc<Mutex<Option<Result<(String, Vec<u8>), String>>>>>,
}

impl ImageSlot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retorna true si hay una imagen lista para usar
    pub fn is_some(&self) -> bool {
        self.dynamic_image.is_some() || self.bytes.is_some()
    }

    /// Retorna una referencia a la imagen decodificada
    pub fn get_image(&self) -> Option<&image::DynamicImage> {
        self.dynamic_image.as_ref()
    }

    /// Limpia la imagen cargada
    pub fn clear(&mut self) {
        self.display_name = None;
        self.bytes = None;
        self.dynamic_image = None;
        self.color_image = None;
        self.error_msg = None;
        self.is_loading = false;
        self.async_receiver = None;
    }

    /// Asigna bytes y decodifica la imagen
    pub fn set_bytes(&mut self, name: String, data: Vec<u8>) {
        match image::load_from_memory(&data) {
            Ok(dyn_img) => {
                let color_img = dynamic_image_to_color_image(&dyn_img);
                self.display_name = Some(name);
                self.dynamic_image = Some(dyn_img);
                self.color_image = Some(color_img);
                self.bytes = Some(data);
                self.error_msg = None;
                self.is_loading = false;
            }
            Err(e) => {
                self.error_msg = Some(format!("Error al decodificar imagen: {}", e));
                self.is_loading = false;
            }
        }
    }

    /// Comprueba si hay tareas asíncronas pendientes completadas
    pub fn poll(&mut self) {
        let res = if let Some(ref rx) = self.async_receiver {
            rx.lock().ok().and_then(|mut guard| guard.take())
        } else {
            None
        };

        if let Some(res) = res {
            self.async_receiver = None;
            self.is_loading = false;
            match res {
                Ok((name, data)) => {
                    self.set_bytes(name, data);
                }
                Err(e) => {
                    if e != "Cancelado" {
                        self.error_msg = Some(e);
                    }
                }
            }
        }
    }

    /// Inicia la apertura del selector de archivos del sistema / navegador
    pub fn trigger_pick_file(&mut self) {
        self.is_loading = true;
        self.error_msg = None;
        let slot = Arc::new(Mutex::new(None));
        self.async_receiver = Some(slot.clone());

        crate::file_dialog::pick_file_async("Imágenes", &["png", "jpg", "jpeg", "bmp", "webp"], move |name, bytes| {
            if let Ok(mut guard) = slot.lock() {
                *guard = Some(Ok((name, bytes)));
            }
        });
    }

    /// Inicia la descarga HTTP/HTTPS desde la URL especificada
    pub fn trigger_load_url(&mut self, url: String) {
        let clean_url = url.trim().to_string();
        if clean_url.is_empty() {
            return;
        }
        self.is_loading = true;
        self.error_msg = None;
        let slot = Arc::new(Mutex::new(None));
        self.async_receiver = Some(slot.clone());

        let slot_success = slot.clone();
        let slot_err = slot.clone();
        let name_success = clean_url.clone();

        crate::web_utils::fetch_url_bytes(
            clean_url,
            move |bytes| {
                if let Ok(mut guard) = slot_success.lock() {
                    *guard = Some(Ok((name_success, bytes)));
                }
            },
            move |err| {
                if let Ok(mut guard) = slot_err.lock() {
                    *guard = Some(Err(format!("Error descargando URL: {}", err)));
                }
            },
        );
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ImageInputResponse {
    pub clicked: bool,
    pub removed: bool,
}

/// Tarjeta para cargar imagen con vista previa interactiva real en memoria, fila de URL y botón de archivo local
pub fn image_input_card(
    ui: &mut Ui,
    theme: &AppTheme,
    title: &str,
    slot: &mut ImageSlot,
    color: ChannelColor,
) -> ImageInputResponse {
    let mut resp = ImageInputResponse::default();
    slot.poll();

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

                    if slot.is_some() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let remove_label = format!("{} Quitar", crate::icons::ICON_CLOSE.codepoint);
                            if ui.button(RichText::new(remove_label).size(11.5).color(theme.accent_red())).clicked() {
                                slot.clear();
                                resp.removed = true;
                            }
                        });
                    }
                });

                ui.add_space(10.0);

                if slot.is_loading {
                    let loading_box = Frame::new()
                        .fill(if *theme == AppTheme::Dark { Color32::from_rgb(16, 20, 26) } else { Color32::from_rgb(241, 245, 249) })
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .corner_radius(8.0)
                        .inner_margin(12.0);

                    loading_box.show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(24.0);
                            ui.spinner();
                            ui.add_space(8.0);
                            ui.label(RichText::new("Cargando / descargando imagen...").size(12.5).color(theme.text_secondary()));
                            ui.add_space(24.0);
                        });
                    });
                } else if let Some(ref color_img) = slot.color_image {
                    let preview_height = 150.0;
                    let img_box = Frame::new()
                        .fill(if *theme == AppTheme::Dark { Color32::from_rgb(16, 20, 26) } else { Color32::from_rgb(241, 245, 249) })
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .corner_radius(8.0)
                        .inner_margin(6.0);

                    img_box.show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            let tex = ui.ctx().load_texture(
                                format!("tex_{}_{}", title, color_img.size[0]),
                                color_img.clone(),
                                egui::TextureOptions::LINEAR,
                            );
                            let img_widget = egui::Image::new(&tex)
                                .max_height(preview_height)
                                .corner_radius(6.0)
                                .sense(egui::Sense::click());
                            let img_resp = ui.add(img_widget)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text("Clic para ampliar imagen en el visor");
                            if img_resp.clicked() {
                                resp.clicked = true;
                            }
                        });
                    });

                    ui.add_space(6.0);
                    let file_name = slot.display_name.as_deref().unwrap_or(title);
                    let short_name = if file_name.len() > 32 {
                        format!("{}...{}", &file_name[..18], &file_name[file_name.len() - 10..])
                    } else {
                        file_name.to_string()
                    };
                    ui.label(RichText::new(format!("Imagen: {}", short_name)).size(12.0).color(theme.text_secondary()));
                } else {
                    // Zona de clic para buscar archivo
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
                        slot.trigger_pick_file();
                    }
                }

                if let Some(ref err) = slot.error_msg {
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("Aviso: {}", err)).size(12.0).color(theme.accent_red()));
                }

                ui.add_space(10.0);

                // Fila de URL y botón Cargar URL
                ui.horizontal(|ui| {
                    let btn_w = 90.0;
                    let avail = ui.available_width();
                    let edit_w = (avail - btn_w - 8.0).max(100.0);

                    ui.add(
                        egui::TextEdit::singleline(&mut slot.url_input)
                            .hint_text(format!("URL {}", title))
                            .desired_width(edit_w),
                    );

                    let url_btn = egui::Button::new(RichText::new("Cargar URL").size(12.0))
                        .min_size(Vec2::new(btn_w, 28.0));
                    if ui.add(url_btn).clicked() {
                        let url = slot.url_input.clone();
                        slot.trigger_load_url(url);
                    }
                });

                ui.add_space(6.0);

                // Botón de ancho completo para Archivo Local
                let local_btn = egui::Button::new(
                    RichText::new(format!("Explorar Archivo Local ({})", title)).strong().size(12.5),
                )
                .fill(theme.surface_subtle())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .min_size(Vec2::new(ui.available_width(), 32.0));

                if ui.add(local_btn).clicked() {
                    slot.trigger_pick_file();
                }
            });
        });

    resp
}

/// Tarjeta interactiva para productos intermedios del pipeline con renderizado de imagen real
#[derive(Debug, Clone, Copy, Default)]
pub struct PreviewCardResponse {
    pub view_fullscreen: bool,
    pub download: bool,
}

pub fn intermediate_preview_card(
    ui: &mut Ui,
    theme: &AppTheme,
    title: &str,
    description: &str,
    file_path: Option<&std::path::Path>,
    is_ready: bool,
    color: ChannelColor,
    color_image: Option<&egui::ColorImage>,
) -> PreviewCardResponse {
    let mut action = PreviewCardResponse::default();
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

                    if img_resp.clicked() {
                        action.view_fullscreen = true;
                    }
                    if img_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        img_resp.on_hover_text("Clic para ampliar imagen a pantalla completa con zoom");
                    }

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        let half_w = (ui.available_width() - 8.0) * 0.5;
                        let view_label = format!("{} Ampliar", crate::icons::ICON_VISIBILITY.codepoint);
                        let view_btn = egui::Button::new(RichText::new(view_label).size(12.0))
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(half_w, 28.0));

                        if ui.add(view_btn).clicked() {
                            action.view_fullscreen = true;
                        }

                        let dl_label = format!("{} Descargar", crate::icons::ICON_DOWNLOAD.codepoint);
                        let dl_btn = egui::Button::new(RichText::new(dl_label).size(12.0).color(Color32::WHITE))
                            .fill(theme.accent_blue())
                            .corner_radius(6.0)
                            .min_size(Vec2::new(half_w, 28.0));

                        if ui.add(dl_btn).clicked() {
                            action.download = true;
                        }
                    });
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
    action
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
    id_salt: impl std::hash::Hash + std::fmt::Debug,
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
                // Chevron hacia abajo
                ui.painter().line_segment([
                    chevron_center + egui::vec2(-4.5, -2.5),
                    chevron_center + egui::vec2(0.0, 2.5),
                ], chevron_stroke);
                ui.painter().line_segment([
                    chevron_center + egui::vec2(0.0, 2.5),
                    chevron_center + egui::vec2(4.5, -2.5),
                ], chevron_stroke);
            } else {
                // Chevron hacia la derecha
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
