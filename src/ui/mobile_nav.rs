use crate::icons;
use crate::pipelines::PipelineId;
use crate::theme::AppTheme;
use crate::ui::desktop_nav::{
    render_m3_filter_chips, render_m3_search_bar, render_pipeline_card, DesktopNavState,
    NavSection,
};
use egui::{Color32, Frame, Margin, Rect, RichText, Sense, Ui, Vec2};

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
) {
    ui.horizontal(|ui| {
        if active_section == NavSection::Pipelines && state.showing_detail {
            // Botón de retorno (flecha atrás vectorial)
            let (back_rect, back_resp) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::click());
            if back_resp.clicked() {
                state.showing_detail = false;
            }
            if back_resp.hovered() {
                ui.painter().rect_filled(back_rect, 17.0, theme.surface_container_high());
            }
            icons::draw_back_arrow(ui, back_rect, theme.text_primary());

            ui.add_space(6.0);
            ui.label(RichText::new(selected_pipeline.title()).strong().size(15.0).color(theme.text_primary()));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                Frame::new()
                    .fill(theme.surface_container_high())
                    .corner_radius(4.0)
                    .inner_margin(Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(selected_pipeline.frequency_badge())
                                .monospace()
                                .size(10.5)
                                .color(theme.accent_blue()),
                        );
                    });
            });
        } else if active_section == NavSection::Pipelines {
            ui.add(
                egui::Image::new(egui::include_image!("../../assets/icon_256.png"))
                    .fit_to_exact_size(Vec2::new(26.0, 26.0))
                    .corner_radius(6.0),
            );
            ui.add_space(4.0);
            ui.label(RichText::new("Signals SDR").strong().size(15.0).color(theme.accent_blue()));
            ui.label(RichText::new("Transmisores").size(12.0).color(theme.text_secondary()));
        } else {
            ui.label(RichText::new("Ajustes y Rutas").strong().size(15.0).color(theme.text_primary()));
        }
    });
}

/// Renderiza la barra de navegación inferior móvil estilo Material 3 con indicador píldora
pub fn render_mobile_bottom_bar(
    ui: &mut Ui,
    theme: &AppTheme,
    active_section: &mut NavSection,
    mobile_state: &mut MobileNavState,
) {
    ui.set_height(56.0);
    ui.horizontal_centered(|ui| {
        let half_width = ui.available_width() / 2.0;

        // Pestaña Señales
        let is_pipe = *active_section == NavSection::Pipelines;
        let (rect_pipe, resp_pipe) = ui.allocate_exact_size(Vec2::new(half_width - 8.0, 48.0), Sense::click());
        if resp_pipe.clicked() {
            *active_section = NavSection::Pipelines;
        }

        // Píldora M3
        let pill_pipe = Rect::from_center_size(
            egui::pos2(rect_pipe.center().x, rect_pipe.center().y - 7.0),
            Vec2::new(56.0, 28.0),
        );
        let pipe_bg = if is_pipe {
            theme.active_pill()
        } else if resp_pipe.hovered() {
            theme.surface_container_high()
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(pill_pipe, 14.0, pipe_bg);

        let pipe_color = if is_pipe { theme.accent_blue() } else { theme.text_secondary() };
        let icon_rect = Rect::from_center_size(
            pill_pipe.center(),
            Vec2::new(20.0, 20.0),
        );
        icons::draw_satellite_icon(ui, icon_rect, pipe_color);
        ui.painter().text(
            egui::pos2(rect_pipe.center().x, rect_pipe.bottom() - 4.0),
            egui::Align2::CENTER_CENTER,
            "Señales",
            egui::FontId::proportional(11.0),
            pipe_color,
        );

        // Pestaña Ajustes
        let is_sett = *active_section == NavSection::Settings;
        let (rect_sett, resp_sett) = ui.allocate_exact_size(Vec2::new(half_width - 8.0, 48.0), Sense::click());
        if resp_sett.clicked() {
            *active_section = NavSection::Settings;
            mobile_state.showing_detail = false;
        }

        let pill_sett = Rect::from_center_size(
            egui::pos2(rect_sett.center().x, rect_sett.center().y - 7.0),
            Vec2::new(56.0, 28.0),
        );
        let sett_bg = if is_sett {
            theme.active_pill()
        } else if resp_sett.hovered() {
            theme.surface_container_high()
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(pill_sett, 14.0, sett_bg);

        let sett_color = if is_sett { theme.accent_blue() } else { theme.text_secondary() };
        let sett_icon_rect = Rect::from_center_size(
            pill_sett.center(),
            Vec2::new(20.0, 20.0),
        );
        icons::draw_settings_icon(ui, sett_icon_rect, sett_color);
        ui.painter().text(
            egui::pos2(rect_sett.center().x, rect_sett.bottom() - 4.0),
            egui::Align2::CENTER_CENTER,
            "Ajustes",
            egui::FontId::proportional(11.0),
            sett_color,
        );
    });
}

/// Renderiza la lista móvil con buscador, chips y tarjetas unificadas M3
pub fn render_mobile_pipeline_grid(
    ui: &mut Ui,
    theme: &AppTheme,
    selected_pipeline: &mut PipelineId,
    mobile_state: &mut MobileNavState,
    nav_state: &mut DesktopNavState,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new("Transmisores Disponibles").strong().size(14.5).color(theme.text_primary()));
        ui.add_space(6.0);

        // Barra de búsqueda M3 integrada en móvil
        render_m3_search_bar(ui, theme, &mut nav_state.search_query);
        ui.add_space(6.0);

        // Chips de filtro rápido por categoría
        render_m3_filter_chips(ui, theme, &mut nav_state.filter_category);
        ui.add_space(8.0);

        let pipelines = PipelineId::all();
        let query = nav_state.search_query.trim().to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut matched_count = 0;
            for &pipe in pipelines {
                // Filtro por chip
                if !nav_state.filter_category.matches(pipe) {
                    continue;
                }

                // Filtro por texto
                if !query.is_empty() {
                    let match_title = pipe.title().to_lowercase().contains(&query);
                    let match_desc = pipe.description().to_lowercase().contains(&query);
                    let match_cat = pipe.category().to_lowercase().contains(&query);
                    let match_badge = pipe.frequency_badge().to_lowercase().contains(&query);
                    if !match_title && !match_desc && !match_cat && !match_badge {
                        continue;
                    }
                }

                matched_count += 1;
                let is_active = *selected_pipeline == pipe;
                // Tarjeta idéntica compartida entre Desktop y Mobile
                if render_pipeline_card(ui, theme, pipe, is_active).clicked() {
                    *selected_pipeline = pipe;
                    mobile_state.showing_detail = true;
                }

                ui.add_space(6.0);
            }

            if matched_count == 0 {
                ui.add_space(16.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No se encontraron transmisores").size(12.0).color(theme.text_secondary()));
                    ui.label(RichText::new("Prueba con otros términos de búsqueda").size(11.0).color(theme.text_secondary()));
                });
            }
        });
    });
}
