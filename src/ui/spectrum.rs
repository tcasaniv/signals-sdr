use crate::theme::AppTheme;
use egui::{Color32, Frame, Pos2, RichText, Stroke, Ui, Vec2};

/// Visualizador gráfico de forma de onda (osciloscopio) y espectro de señal
pub fn render_signal_visualizer(
    ui: &mut Ui,
    theme: &AppTheme,
    samples: &[(f32, f32)],
    sample_rate: u32,
) {
    Frame::new()
        .fill(theme.card_bg())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(8.0)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Visualizador de Señal I/Q")
                            .strong()
                            .size(14.0)
                            .color(theme.text_primary()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} muestras | {:.2}s a {} Hz",
                                samples.len(),
                                samples.len() as f32 / sample_rate as f32,
                                sample_rate
                            ))
                            .size(12.0)
                            .color(theme.text_secondary()),
                        );
                    });
                });

                ui.add_space(6.0);

                let display_h = 100.0;
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), display_h),
                    egui::Sense::hover(),
                );

                let bg_fill = if *theme == AppTheme::Dark {
                    Color32::from_rgb(15, 20, 28)
                } else {
                    Color32::from_rgb(238, 244, 252)
                };
                ui.painter().rect_filled(rect, 4.0, bg_fill);
                ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0_f32, theme.card_border()), egui::StrokeKind::Inside);

                if samples.is_empty() {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Genera o carga una señal para visualizar su forma de onda",
                        egui::FontId::proportional(12.0),
                        theme.text_secondary(),
                    );
                    return;
                }

                // Línea central de cero
                let center_y = rect.center().y;
                ui.painter().line_segment(
                    [Pos2::new(rect.left(), center_y), Pos2::new(rect.right(), center_y)],
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(128, 128, 128, 60)),
                );

                // Dibujar forma de onda de I y Q
                let n_points = (rect.width() as usize).min(samples.len()).max(10);
                let step = samples.len() / n_points;
                let i_color = theme.accent_blue();
                let q_color = Color32::from_rgb(240, 140, 40); // Naranja suave para Q

                let mut prev_i_pt = None;
                let mut prev_q_pt = None;

                for x_idx in 0..n_points {
                    let s_idx = (x_idx * step).min(samples.len() - 1);
                    let (i_val, q_val) = samples[s_idx];

                    let x_pos = rect.left() + (x_idx as f32 / n_points as f32) * rect.width();
                    let y_i = center_y - (i_val.clamp(-1.0, 1.0) * (display_h * 0.42));
                    let y_q = center_y - (q_val.clamp(-1.0, 1.0) * (display_h * 0.42));

                    let curr_i_pt = Pos2::new(x_pos, y_i);
                    let curr_q_pt = Pos2::new(x_pos, y_q);

                    if let Some(prev) = prev_i_pt {
                        ui.painter().line_segment([prev, curr_i_pt], Stroke::new(1.2_f32, i_color));
                    }
                    if let Some(prev) = prev_q_pt {
                        ui.painter().line_segment([prev, curr_q_pt], Stroke::new(1.0_f32, q_color));
                    }

                    prev_i_pt = Some(curr_i_pt);
                    prev_q_pt = Some(curr_q_pt);
                }

                // Leyenda
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Canal I (En fase)").size(11.0).color(i_color));
                    ui.add_space(10.0);
                    ui.label(RichText::new("Canal Q (Cuadratura)").size(11.0).color(q_color));
                });
            });
        });
}
