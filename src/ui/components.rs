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
                ui.label(
                    RichText::new(title)
                        .strong()
                        .size(15.0)
                        .color(theme.accent_blue()),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(description)
                        .size(13.0)
                        .color(theme.text_secondary()),
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

    ui.horizontal(|ui| {
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

/// Tarjeta para cargar imagen con botón de archivo local y URL
pub fn image_input_card(
    ui: &mut Ui,
    theme: &AppTheme,
    title: &str,
    current_path: &mut Option<String>,
    url_input: &mut String,
) {
    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).strong().size(14.0).color(theme.text_primary()));
                ui.add_space(6.0);

                // Cuadro de estado / miniatura
                let preview_height = 80.0;
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), preview_height),
                    egui::Sense::hover(),
                );

                let fill = if *theme == AppTheme::Dark {
                    Color32::from_rgb(35, 42, 54)
                } else {
                    Color32::from_rgb(230, 236, 244)
                };

                ui.painter().rect_filled(rect, 6.0, fill);

                if let Some(ref path) = current_path {
                    let display = if path.len() > 35 {
                        format!("...{}", &path[path.len() - 32..])
                    } else {
                        path.clone()
                    };
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        display,
                        egui::FontId::proportional(12.0),
                        theme.text_primary(),
                    );
                } else {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Sin imagen seleccionada",
                        egui::FontId::proportional(12.0),
                        theme.text_secondary(),
                    );
                }

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.button("Archivo Local").clicked() {
                        if let Some(file) = rfd::FileDialog::new()
                            .add_filter("Imágenes", &["png", "jpg", "jpeg", "bmp", "webp"])
                            .pick_file()
                        {
                            *current_path = Some(file.display().to_string());
                        }
                    }

                    ui.text_edit_singleline(url_input);
                    if ui.button("Cargar URL").clicked() {
                        if !url_input.is_empty() {
                            *current_path = Some(url_input.clone());
                        }
                    }
                });
            });
        });
}
