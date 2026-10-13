use crate::theme::AppTheme;
use egui::{Color32, Frame, RichText, Stroke, Ui, Vec2};

/// Tarjeta informativa y pedagógica para explicar el protocolo/sistema de modulación
pub fn info_card(ui: &mut Ui, theme: &AppTheme, title: &str, description: &str) {
    Frame::new()
        .fill(theme.info_card_bg())
        .stroke(Stroke::new(1.0_f32, theme.info_card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(title)
                            .strong()
                            .size(15.0)
                            .color(theme.accent_blue()),
                    )
                    .wrap(),
                );
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(description)
                            .size(12.5)
                            .color(theme.text_secondary()),
                    )
                    .wrap(),
                );
            });
        });
}

/// Selector / Barra de navegación de 3 pasos (1. Elegir fuente, 2. Acción, 3. Transmisión)
pub fn step_bar(
    ui: &mut Ui,
    theme: &AppTheme,
    current_step: &mut usize,
    step_two_label: &str,
) {
    let steps = ["1. Elegir fuente", step_two_label, "3. Transmisión"];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        for (idx, &label) in steps.iter().enumerate() {
            let is_selected = *current_step == idx;
            let text_color = if is_selected {
                theme.accent_blue()
            } else {
                theme.text_secondary()
            };

            let bg_color = if is_selected {
                if *theme == AppTheme::Dark {
                    Color32::from_rgb(30, 48, 70)
                } else {
                    Color32::from_rgb(225, 238, 255)
                }
            } else {
                Color32::TRANSPARENT
            };

            let btn = egui::Button::new(
                RichText::new(label)
                    .size(13.0)
                    .strong()
                    .color(text_color),
            )
            .fill(bg_color)
            .stroke(if is_selected {
                Stroke::new(1.0_f32, theme.accent_blue())
            } else {
                Stroke::NONE
            })
            .corner_radius(6.0);

            if ui.add(btn).clicked() {
                *current_step = idx;
            }
        }
    });

    ui.add_space(6.0);
    // Línea divisoria sutil
    let line_rect = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [line_rect.left_top(), egui::Pos2::new(line_rect.right(), line_rect.top())],
        Stroke::new(1.0_f32, theme.card_border()),
    );
    ui.add_space(6.0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelColor {
    Blue,
    Green,
    Neutral,
}

/// Tarjeta para cargar imagen con vista previa estilizada, fila de URL y botón de archivo local
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
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Título centrado
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(title).strong().size(14.0).color(theme.text_primary()));
                });
                ui.add_space(8.0);

                // Cuadro de previsualización interactivo con color característico (Azul A / Verde B estilo APT-Tx)
                let preview_height = 120.0;
                let (rect, mut response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), preview_height),
                    egui::Sense::click(),
                );

                if response.clicked() {
                    if let Some(ref path) = current_path {
                        crate::utils::open_path_or_url(path);
                    }
                }

                let is_hovered = response.hovered();
                if is_hovered && current_path.is_some() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    let tip = if let Some(ref p) = current_path {
                        format!("Clic para abrir '{}' en la aplicación predeterminada", p)
                    } else {
                        "Selecciona un archivo local o URL".to_string()
                    };
                    response = response.on_hover_text(tip);
                }

                let mut fill = match color {
                    ChannelColor::Blue => Color32::from_rgb(26, 126, 219),
                    ChannelColor::Green => Color32::from_rgb(46, 150, 68),
                    ChannelColor::Neutral => {
                        if *theme == AppTheme::Dark {
                            Color32::from_rgb(35, 42, 54)
                        } else {
                            Color32::from_rgb(220, 228, 238)
                        }
                    }
                };

                if response.hovered() && current_path.is_some() {
                    fill = fill.gamma_multiply(1.15);
                }

                ui.painter().rect_filled(rect, 6.0, fill);

                if response.hovered() && current_path.is_some() {
                    ui.painter().rect_stroke(rect, 6.0, Stroke::new(2.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
                }

                let display_text = if let Some(ref path) = current_path {
                    std::path::Path::new(path)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.clone())
                } else {
                    format!("{} Original", title)
                };

                let text_color = match color {
                    ChannelColor::Blue | ChannelColor::Green => Color32::WHITE,
                    ChannelColor::Neutral => theme.text_primary(),
                };

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    display_text,
                    egui::FontId::proportional(13.5),
                    text_color,
                );

                if current_path.is_some() {
                    ui.painter().text(
                        rect.left_bottom() + egui::vec2(8.0, -8.0),
                        egui::Align2::LEFT_BOTTOM,
                        "↗ Clic para abrir",
                        egui::FontId::proportional(11.0),
                        Color32::from_white_alpha(220),
                    );
                }

                ui.add_space(8.0);

                // Fila de URL y botón Cargar URL con soporte para wrap
                ui.horizontal_wrapped(|ui| {
                    let btn_w = 85.0;
                    let avail = ui.available_width();
                    let edit_w = if avail > 180.0 { avail - btn_w - 8.0 } else { avail - 4.0 };

                    ui.add(
                        egui::TextEdit::singleline(url_input)
                            .hint_text(format!("URL {}", title))
                            .desired_width(edit_w),
                    );

                    let url_btn = egui::Button::new(RichText::new("Cargar URL").size(12.0));
                    if ui.add_sized([btn_w, 24.0], url_btn).clicked() {
                        if !url_input.trim().is_empty() {
                            *current_path = Some(url_input.trim().to_string());
                        }
                    }
                });

                ui.add_space(6.0);

                // Botón de ancho completo para Archivo Local
                let local_btn = egui::Button::new(RichText::new(format!("Archivo Local {}", title)).strong().size(12.5))
                    .fill(theme.surface_subtle())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()));

                if ui.add_sized([ui.available_width(), 26.0], local_btn).clicked() {
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

/// Tarjeta interactiva para productos intermedios del pipeline (Imágenes preprocesadas, APT compuesta, etc.)
pub fn intermediate_preview_card(
    ui: &mut Ui,
    theme: &AppTheme,
    title: &str,
    description: &str,
    file_path: Option<&std::path::Path>,
    is_ready: bool,
    color: ChannelColor,
) {
    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Cabecera con título e indicador de estado
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).strong().size(13.5).color(theme.text_primary()));
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
                ui.add_space(6.0);

                // Cuadro interactivo de previsualización
                let preview_height = 100.0;
                let (rect, mut response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), preview_height),
                    egui::Sense::click(),
                );

                if response.clicked() && is_ready {
                    if let Some(path) = file_path {
                        if let Some(path_str) = path.to_str() {
                            crate::utils::open_path_or_url(path_str);
                        }
                    }
                }

                let is_hovered = response.hovered();
                if is_hovered && is_ready {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    response = response.on_hover_text("Clic para abrir imagen en la aplicación predeterminada");
                }

                let mut fill = match color {
                    ChannelColor::Blue => Color32::from_rgb(26, 126, 219),
                    ChannelColor::Green => Color32::from_rgb(46, 150, 68),
                    ChannelColor::Neutral => {
                        if *theme == AppTheme::Dark {
                            Color32::from_rgb(30, 36, 46)
                        } else {
                            Color32::from_rgb(225, 232, 240)
                        }
                    }
                };

                if !is_ready {
                    fill = fill.gamma_multiply(0.4);
                } else if response.hovered() {
                    fill = fill.gamma_multiply(1.15);
                }

                ui.painter().rect_filled(rect, 6.0, fill);

                if response.hovered() && is_ready {
                    ui.painter().rect_stroke(rect, 6.0, Stroke::new(2.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
                }

                let center_text = if is_ready {
                    if let Some(path) = file_path {
                        path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Generado".to_string())
                    } else {
                        "Generado".to_string()
                    }
                } else {
                    "Sin procesar".to_string()
                };

                let text_color = if is_ready { Color32::WHITE } else { theme.text_secondary() };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    center_text,
                    egui::FontId::proportional(13.0),
                    text_color,
                );

                if is_ready {
                    ui.painter().text(
                        rect.left_bottom() + egui::vec2(8.0, -6.0),
                        egui::Align2::LEFT_BOTTOM,
                        "↗ Clic para abrir",
                        egui::FontId::proportional(10.5),
                        Color32::from_white_alpha(220),
                    );
                }

                ui.add_space(6.0);

                if is_ready && file_path.is_some() {
                    let path = file_path.unwrap();
                    let open_btn = egui::Button::new(RichText::new("Abrir Imagen en Visor").size(11.5))
                        .fill(theme.surface_subtle())
                        .stroke(Stroke::new(1.0_f32, theme.card_border()));
                    if ui.add_sized([ui.available_width(), 24.0], open_btn).clicked() {
                        if let Some(p) = path.to_str() {
                            crate::utils::open_path_or_url(p);
                        }
                    }
                }
            });
        });
}
