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

                // Cuadro de previsualización con color característico (Azul A / Verde B estilo APT-Tx)
                let preview_height = 120.0;
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), preview_height),
                    egui::Sense::hover(),
                );

                let fill = match color {
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

                ui.painter().rect_filled(rect, 6.0, fill);

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
