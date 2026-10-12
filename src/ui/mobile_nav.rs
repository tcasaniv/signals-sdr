use crate::icons;
use crate::pipelines::PipelineId;
use crate::theme::AppTheme;
use crate::ui::desktop_nav::NavSection;
use egui::{Color32, Frame, Rect, RichText, Sense, Stroke, Ui, Vec2};

pub struct MobileNavState {
    pub showing_detail: bool,
}

impl Default for MobileNavState {
    fn default() -> Self {
        Self {
            showing_detail: false,
        }
    }
}

/// Renderiza la barra superior para dispositivos móviles (con botón atrás si está en detalle)
pub fn render_mobile_header(
    ui: &mut Ui,
    theme: &AppTheme,
    state: &mut MobileNavState,
    active_section: NavSection,
    selected_pipeline: PipelineId,
    dark_mode: &mut bool,
) {
    Frame::new()
        .fill(theme.surface_subtle())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if active_section == NavSection::Pipelines && state.showing_detail {
                    // Botón de retorno (flecha atrás vectorial)
                    let (back_rect, back_resp) = ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
                    if back_resp.clicked() {
                        state.showing_detail = false;
                    }
                    if back_resp.hovered() {
                        ui.painter().rect_filled(back_rect, 4.0, theme.card_bg());
                    }
                    icons::draw_back_arrow(ui, back_rect, theme.text_primary());

                    ui.add_space(6.0);
                    ui.label(RichText::new(selected_pipeline.title()).strong().size(15.0).color(theme.text_primary()));
                } else if active_section == NavSection::Pipelines {
                    ui.label(RichText::new("Signals SDR").strong().size(15.0).color(theme.accent_blue()));
                    ui.label(RichText::new("Generador").size(12.0).color(theme.text_secondary()));
                } else {
                    ui.label(RichText::new("Ajustes y Rutas").strong().size(15.0).color(theme.text_primary()));
                }

                // Alineación al extremo derecho para el switch de tema
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_label = if *dark_mode { "Luz" } else { "Oscuro" };
                    if ui.button(RichText::new(theme_label).size(11.0)).clicked() {
                        *dark_mode = !*dark_mode;
                    }
                });
            });
        });
}

/// Renderiza la barra de navegación inferior móvil (con iconos de Satélite y Ajustes)
pub fn render_mobile_bottom_bar(
    ui: &mut Ui,
    theme: &AppTheme,
    active_section: &mut NavSection,
    mobile_state: &mut MobileNavState,
) {
    Frame::new()
        .fill(theme.surface_subtle())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.set_height(52.0);
            ui.horizontal_centered(|ui| {
                let half_width = ui.available_width() / 2.0;

                // Pestaña Señales
                let is_pipe = *active_section == NavSection::Pipelines;
                let (rect_pipe, resp_pipe) = ui.allocate_exact_size(Vec2::new(half_width - 8.0, 44.0), Sense::click());
                if resp_pipe.clicked() {
                    *active_section = NavSection::Pipelines;
                }

                let pipe_bg = if is_pipe {
                    theme.accent_blue().gamma_multiply(0.18)
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect_pipe, 8.0, pipe_bg);
                let pipe_color = if is_pipe { theme.accent_blue() } else { theme.text_secondary() };
                
                let icon_rect = Rect::from_center_size(
                    egui::pos2(rect_pipe.center().x, rect_pipe.center().y - 6.0),
                    Vec2::new(24.0, 24.0),
                );
                icons::draw_satellite_icon(ui, icon_rect, pipe_color);
                ui.painter().text(
                    egui::pos2(rect_pipe.center().x, rect_pipe.bottom() - 10.0),
                    egui::Align2::CENTER_CENTER,
                    "Señales",
                    egui::FontId::proportional(11.0),
                    pipe_color,
                );

                // Pestaña Ajustes
                let is_sett = *active_section == NavSection::Settings;
                let (rect_sett, resp_sett) = ui.allocate_exact_size(Vec2::new(half_width - 8.0, 44.0), Sense::click());
                if resp_sett.clicked() {
                    *active_section = NavSection::Settings;
                    mobile_state.showing_detail = false;
                }

                let sett_bg = if is_sett {
                    theme.accent_blue().gamma_multiply(0.18)
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect_sett, 8.0, sett_bg);
                let sett_color = if is_sett { theme.accent_blue() } else { theme.text_secondary() };

                let sett_icon_rect = Rect::from_center_size(
                    egui::pos2(rect_sett.center().x, rect_sett.center().y - 6.0),
                    Vec2::new(24.0, 24.0),
                );
                icons::draw_settings_icon(ui, sett_icon_rect, sett_color);
                ui.painter().text(
                    egui::pos2(rect_sett.center().x, rect_sett.bottom() - 10.0),
                    egui::Align2::CENTER_CENTER,
                    "Ajustes",
                    egui::FontId::proportional(11.0),
                    sett_color,
                );
            });
        });
}

/// Renderiza la lista móvil en cuadrícula/tarjetas cuando el usuario está en el catálogo de señales
pub fn render_mobile_pipeline_grid(
    ui: &mut Ui,
    theme: &AppTheme,
    selected_pipeline: &mut PipelineId,
    mobile_state: &mut MobileNavState,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new("Seleccione una Señal para Generar:").strong().size(14.0).color(theme.text_primary()));
        ui.add_space(8.0);

        let pipelines = PipelineId::all();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for &pipe in pipelines {
                let card_frame = Frame::new()
                    .fill(theme.card_bg())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .corner_radius(8.0)
                    .inner_margin(12.0);

                let card_resp = card_frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(pipe.title()).strong().size(14.0).color(theme.accent_blue()));
                            ui.label(RichText::new(format!("({})", pipe.category())).size(11.0).color(theme.text_secondary()));
                        });
                        ui.add_space(4.0);
                        ui.label(RichText::new(pipe.description()).size(12.0).color(theme.text_secondary()));
                    });
                });

                if card_resp.response.interact(Sense::click()).clicked() {
                    *selected_pipeline = pipe;
                    mobile_state.showing_detail = true;
                }

                ui.add_space(6.0);
            }
        });
    });
}
