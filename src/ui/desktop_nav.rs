use crate::icons;
use crate::pipelines::PipelineId;
use crate::theme::AppTheme;
use egui::{Color32, Frame, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Pipelines,
    Settings,
}

pub struct DesktopNavState {
    pub active_section: NavSection,
    pub search_query: String,
}

impl Default for DesktopNavState {
    fn default() -> Self {
        Self {
            active_section: NavSection::Pipelines,
            search_query: String::new(),
        }
    }
}

/// Renderiza la barra vertical izquierda (Rail) y la lista de selección de señales en escritorio
pub fn render_desktop_sidebar(
    ui: &mut Ui,
    theme: &AppTheme,
    state: &mut DesktopNavState,
    selected_pipeline: &mut PipelineId,
    dark_mode: &mut bool,
) {
    ui.horizontal(|ui| {
        // 1. Icon Rail estrecho (ancho fijo 64px)
        Frame::none()
            .fill(theme.surface_subtle())
            .stroke(Stroke::new(1.0, theme.card_border()))
            .show(ui, |ui| {
                ui.set_width(64.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(14.0);
                    // Logo o monograma de la aplicación
                    ui.label(RichText::new("SIGNALS").strong().size(11.0).color(theme.accent_blue()));
                    ui.label(RichText::new("SDR").strong().size(13.0).color(theme.text_primary()));
                    ui.add_space(18.0);

                    // Botón Sección: Satélites y Señales
                    let is_pipe = state.active_section == NavSection::Pipelines;
                    let (rect_pipe, resp_pipe) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), Sense::click());
                    if resp_pipe.clicked() {
                        state.active_section = NavSection::Pipelines;
                    }

                    let pipe_bg = if is_pipe {
                        theme.accent_blue().gamma_multiply(0.2)
                    } else if resp_pipe.hovered() {
                        theme.card_bg()
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect_pipe, 8.0, pipe_bg);
                    if is_pipe {
                        ui.painter().rect_stroke(rect_pipe, 8.0, Stroke::new(1.5, theme.accent_blue()), egui::StrokeKind::Inside);
                    }
                    let pipe_color = if is_pipe { theme.accent_blue() } else { theme.text_secondary() };
                    icons::draw_satellite_icon(ui, rect_pipe.shrink(10.0), pipe_color);
                    ui.label(RichText::new("Señales").size(10.0).color(pipe_color));

                    ui.add_space(12.0);

                    // Botón Sección: Ajustes
                    let is_sett = state.active_section == NavSection::Settings;
                    let (rect_sett, resp_sett) = ui.allocate_exact_size(Vec2::new(44.0, 44.0), Sense::click());
                    if resp_sett.clicked() {
                        state.active_section = NavSection::Settings;
                    }

                    let sett_bg = if is_sett {
                        theme.accent_blue().gamma_multiply(0.2)
                    } else if resp_sett.hovered() {
                        theme.card_bg()
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect_sett, 8.0, sett_bg);
                    if is_sett {
                        ui.painter().rect_stroke(rect_sett, 8.0, Stroke::new(1.5, theme.accent_blue()), egui::StrokeKind::Inside);
                    }
                    let sett_color = if is_sett { theme.accent_blue() } else { theme.text_secondary() };
                    icons::draw_settings_icon(ui, rect_sett.shrink(10.0), sett_color);
                    ui.label(RichText::new("Ajustes").size(10.0).color(sett_color));

                    // Espaciador flexible hacia abajo
                    ui.add_space(ui.available_height() - 50.0);

                    // Interruptor Tema Claro / Oscuro (sin emojis)
                    let theme_label = if *dark_mode { "Luz" } else { "Oscuro" };
                    if ui.button(RichText::new(theme_label).size(11.0)).clicked() {
                        *dark_mode = !*dark_mode;
                    }
                    ui.add_space(10.0);
                });
            });

        // 2. Panel intermedio con buscador y lista de pipelines (solo si está en Pipelines)
        if state.active_section == NavSection::Pipelines {
            Frame::none()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0, theme.card_border()))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_width(260.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Transmisores Disponibles").strong().size(14.0).color(theme.text_primary()));
                        ui.add_space(6.0);

                        // Buscador
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut state.search_query)
                                    .hint_text("Buscar por nombre o banda...")
                                    .desired_width(235.0),
                            );
                        });

                        ui.add_space(8.0);
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let query = state.search_query.trim().to_lowercase();
                            let pipelines = PipelineId::all();

                            let mut current_category = "";

                            for &pipe in pipelines {
                                // Filtro de búsqueda
                                if !query.is_empty() {
                                    let match_title = pipe.title().to_lowercase().contains(&query);
                                    let match_desc = pipe.description().to_lowercase().contains(&query);
                                    let match_cat = pipe.category().to_lowercase().contains(&query);
                                    if !match_title && !match_desc && !match_cat {
                                        continue;
                                    }
                                }

                                // Encabezado de categoría si cambia
                                if pipe.category() != current_category {
                                    current_category = pipe.category();
                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new(current_category)
                                            .size(11.0)
                                            .strong()
                                            .color(theme.accent_blue()),
                                    );
                                }

                                let is_active = *selected_pipeline == pipe;
                                let btn_frame = Frame::none()
                                    .fill(if is_active {
                                        theme.accent_blue().gamma_multiply(0.15)
                                    } else {
                                        theme.surface_subtle()
                                    })
                                    .stroke(Stroke::new(
                                        if is_active { 1.5 } else { 1.0 },
                                        if is_active { theme.accent_blue() } else { theme.card_border() },
                                    ))
                                    .rounding(6.0)
                                    .inner_margin(8.0);

                                let card_resp = btn_frame.show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(pipe.title())
                                                    .strong()
                                                    .size(13.0)
                                                    .color(if is_active { theme.accent_blue() } else { theme.text_primary() }),
                                            );
                                        });
                                        ui.label(
                                            RichText::new(pipe.description())
                                                .size(10.5)
                                                .color(theme.text_secondary()),
                                        );
                                    });
                                });

                                if card_resp.response.interact(Sense::click()).clicked() {
                                    *selected_pipeline = pipe;
                                }

                                ui.add_space(4.0);
                            }
                        });
                    });
                });
        }
    });
}
