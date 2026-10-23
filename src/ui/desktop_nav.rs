use crate::icons;
use crate::pipelines::PipelineId;
use crate::theme::AppTheme;
use egui::{Color32, Frame, Margin, RichText, Sense, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Pipelines,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipelineFilterCategory {
    #[default]
    All,
    Satellites,
    Sstv,
    Analog,
    Iq,
}

impl PipelineFilterCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "Todos",
            Self::Satellites => "Satélites",
            Self::Sstv => "SSTV",
            Self::Analog => "Analógicas",
            Self::Iq => "I/Q DSP",
        }
    }

    pub fn matches(&self, pipe: PipelineId) -> bool {
        match self {
            Self::All => true,
            Self::Satellites => matches!(pipe, PipelineId::NoaaApt | PipelineId::MeteorLrpt | PipelineId::NoaaHrpt),
            Self::Sstv => matches!(pipe, PipelineId::IssSstv | PipelineId::GenericSstv),
            Self::Analog => matches!(pipe, PipelineId::GenericFm | PipelineId::GenericAm),
            Self::Iq => matches!(pipe, PipelineId::GenericIq),
        }
    }
}

pub struct DesktopNavState {
    pub active_section: NavSection,
    pub search_query: String,
    pub filter_category: PipelineFilterCategory,
    pub catalog_width: f32,
}

impl Default for DesktopNavState {
    fn default() -> Self {
        Self {
            active_section: NavSection::Pipelines,
            search_query: String::new(),
            filter_category: PipelineFilterCategory::All,
            catalog_width: 290.0,
        }
    }
}

/// Renderiza la barra vertical izquierda (Navigation Rail de Material 3)
pub fn render_desktop_rail(
    ui: &mut Ui,
    theme: &AppTheme,
    state: &mut DesktopNavState,
    dark_mode: &mut bool,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        // Logotipo de Signals SDR
        ui.add(
            egui::Image::new(egui::include_image!("../../assets/icon_256.png"))
                .fit_to_exact_size(Vec2::new(36.0, 36.0))
                .corner_radius(8.0),
        );
        ui.add_space(4.0);
        ui.label(RichText::new("SIGNALS").strong().size(9.5).color(theme.accent_blue()));
        ui.label(RichText::new("SDR").strong().size(11.0).color(theme.text_primary()));
        ui.add_space(16.0);

        // Destino 1: Señales (M3 Active Pill)
        let is_pipe = state.active_section == NavSection::Pipelines;
        render_rail_item(
            ui,
            theme,
            icons::ICON_SATELLITE_ALT.codepoint,
            "Señales",
            is_pipe,
            || state.active_section = NavSection::Pipelines,
        );

        ui.add_space(12.0);

        // Destino 2: Ajustes (M3 Active Pill)
        let is_sett = state.active_section == NavSection::Settings;
        render_rail_item(
            ui,
            theme,
            icons::ICON_SETTINGS.codepoint,
            "Ajustes",
            is_sett,
            || state.active_section = NavSection::Settings,
        );

        // Espaciador flexible para empujar el interruptor de tema al pie del rail
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(10.0);
            let (mode_rect, mode_resp) = ui.allocate_exact_size(Vec2::new(40.0, 40.0), Sense::click());
            if mode_resp.clicked() {
                *dark_mode = !*dark_mode;
            }
            let mode_bg = if mode_resp.hovered() {
                theme.surface_container_high()
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(mode_rect, 20.0, mode_bg);

            let icon_str = if *dark_mode {
                icons::ICON_LIGHT_MODE.codepoint
            } else {
                icons::ICON_DARK_MODE.codepoint
            };
            let icon_col = if mode_resp.hovered() {
                theme.accent_blue()
            } else {
                theme.text_secondary()
            };
            ui.painter().text(
                mode_rect.center(),
                egui::Align2::CENTER_CENTER,
                icon_str,
                egui::FontId::proportional(20.0),
                icon_col,
            );
            mode_resp.on_hover_text(if *dark_mode {
                "Cambiar a Modo Claro"
            } else {
                "Cambiar a Modo Oscuro"
            });
        });
    });
}

fn render_rail_item(
    ui: &mut Ui,
    theme: &AppTheme,
    icon: &str,
    label: &str,
    is_active: bool,
    mut on_click: impl FnMut(),
) {
    let (item_rect, item_resp) = ui.allocate_exact_size(Vec2::new(56.0, 52.0), Sense::click());
    if item_resp.clicked() {
        on_click();
    }

    // Píldora M3 (52x30, radio 15)
    let pill_rect = egui::Rect::from_center_size(
        egui::pos2(item_rect.center().x, item_rect.top() + 16.0),
        Vec2::new(48.0, 30.0),
    );

    let pill_bg = if is_active {
        theme.active_pill()
    } else if item_resp.hovered() {
        theme.surface_container_high()
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(pill_rect, 15.0, pill_bg);

    let icon_color = if is_active {
        theme.accent_blue()
    } else {
        theme.text_secondary()
    };

    ui.painter().text(
        pill_rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(20.0),
        icon_color,
    );

    ui.painter().text(
        egui::pos2(item_rect.center().x, item_rect.bottom() - 6.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(10.5),
        if is_active { theme.accent_blue() } else { theme.text_secondary() },
    );
}

/// Renderiza la barra de búsqueda estilo Material 3 con icono y botón de borrado
pub fn render_m3_search_bar(
    ui: &mut Ui,
    theme: &AppTheme,
    search_query: &mut String,
) {
    let container_frame = Frame::new()
        .fill(theme.surface_container_high())
        .stroke(Stroke::new(1.0_f32, theme.card_border()))
        .corner_radius(16.0)
        .inner_margin(Margin::symmetric(10, 6));

    container_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

            // Icono de búsqueda
            ui.label(
                RichText::new(icons::ICON_SEARCH.codepoint)
                    .size(15.0)
                    .color(theme.text_secondary()),
            );

            // Entrada de texto flexible y delimitada sin provocar desbordamiento
            let has_query = !search_query.is_empty();
            let clear_btn_width = if has_query { 22.0 } else { 0.0 };
            let edit_width = (ui.available_width() - clear_btn_width).max(30.0);

            ui.add_sized(
                [edit_width, 18.0],
                egui::TextEdit::singleline(search_query)
                    .hint_text("Buscar señales...")
                    .frame(Frame::NONE),
            );

            // Botón de limpiar si hay texto ingresado
            if has_query {
                let (btn_rect, btn_resp) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::click());
                if btn_resp.clicked() {
                    search_query.clear();
                }
                let clear_color = if btn_resp.hovered() {
                    theme.text_primary()
                } else {
                    theme.text_secondary()
                };
                ui.painter().text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    icons::ICON_CLOSE.codepoint,
                    egui::FontId::proportional(13.0),
                    clear_color,
                );
            }
        });
    });
}

/// Renderiza los chips de filtrado rápido estilo Material 3
pub fn render_m3_filter_chips(
    ui: &mut Ui,
    theme: &AppTheme,
    current_filter: &mut PipelineFilterCategory,
) {
    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(5.0, 0.0);
                let categories = [
                    PipelineFilterCategory::All,
                    PipelineFilterCategory::Satellites,
                    PipelineFilterCategory::Sstv,
                    PipelineFilterCategory::Analog,
                    PipelineFilterCategory::Iq,
                ];

                for cat in categories {
                    let is_selected = *current_filter == cat;
                    let bg_color = if is_selected {
                        theme.active_pill()
                    } else {
                        theme.card_bg()
                    };
                    let stroke = Stroke::new(
                        1.0_f32,
                        if is_selected { theme.accent_blue() } else { theme.card_border() },
                    );
                    let text_color = if is_selected {
                        theme.accent_blue()
                    } else {
                        theme.text_secondary()
                    };

                    let chip_frame = Frame::new()
                        .fill(bg_color)
                        .stroke(stroke)
                        .corner_radius(12.0)
                        .inner_margin(Margin::symmetric(9, 5));

                    let resp = chip_frame.show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(cat.label())
                                    .size(11.0)
                                    .strong()
                                    .color(text_color),
                            )
                            .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    });

                    if resp.response.interact(Sense::click()).clicked() {
                        *current_filter = cat;
                    }
                }
            });
        });
}

/// Tarjeta unificada de señal M3 compartida entre Desktop y Mobile
pub fn render_pipeline_card(
    ui: &mut Ui,
    theme: &AppTheme,
    pipe: PipelineId,
    is_active: bool,
) -> egui::Response {
    let bg_color = if is_active {
        theme.active_pill()
    } else {
        theme.card_bg()
    };

    let stroke = Stroke::new(
        if is_active { 1.5_f32 } else { 1.0_f32 },
        if is_active { theme.accent_blue() } else { theme.card_border() },
    );

    let frame = Frame::new()
        .fill(bg_color)
        .stroke(stroke)
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(10, 8));

    let frame_resp = frame.show(ui, |ui| {
        ui.vertical(|ui| {
            // Cabecera de la tarjeta: Icono + Título + Badge de frecuencia
            ui.horizontal(|ui| {
                let pipe_icon = match pipe {
                    PipelineId::NoaaApt | PipelineId::MeteorLrpt | PipelineId::NoaaHrpt => {
                        icons::ICON_SATELLITE_ALT.codepoint
                    }
                    PipelineId::IssSstv | PipelineId::GenericSstv => icons::ICON_IMAGE.codepoint,
                    PipelineId::GenericFm | PipelineId::GenericAm => icons::ICON_GRAPHIC_EQ.codepoint,
                    PipelineId::GenericIq => icons::ICON_ANALYTICS.codepoint,
                };

                let icon_color = if is_active {
                    theme.accent_blue()
                } else {
                    theme.text_secondary()
                };

                let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
                let icon_bg = if is_active {
                    theme.accent_blue().gamma_multiply(0.20)
                } else {
                    theme.surface_container_high()
                };
                ui.painter().rect_filled(icon_rect, 6.0, icon_bg);
                ui.painter().text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    pipe_icon,
                    egui::FontId::proportional(15.0),
                    icon_color,
                );

                ui.add_space(4.0);

                let badge_text = pipe.frequency_badge();
                let badge_bg = if is_active {
                    theme.accent_blue().gamma_multiply(0.22)
                } else {
                    theme.surface_container_high()
                };
                let badge_fg = if is_active {
                    theme.accent_blue()
                } else {
                    theme.text_secondary()
                };

                // Badge ocupa ~72px (e.g. "137.1 MHz" o "437.8 MHz")
                let badge_w = 72.0;
                let title_w = (ui.available_width() - badge_w - 6.0).max(30.0);

                ui.add_sized(
                    [title_w, 20.0],
                    egui::Label::new(
                        RichText::new(pipe.title())
                            .strong()
                            .size(13.0)
                            .color(if is_active {
                                theme.accent_blue()
                            } else {
                                theme.text_primary()
                            }),
                    )
                    .truncate(),
                );

                Frame::new()
                    .fill(badge_bg)
                    .corner_radius(4.0)
                    .inner_margin(Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(badge_text)
                                .monospace()
                                .size(10.0)
                                .color(badge_fg),
                        );
                    });
            });

            ui.add_space(4.0);

            // Descripción sin redundancia de nombre de categoría
            ui.add(
                egui::Label::new(
                    RichText::new(pipe.description())
                        .size(11.0)
                        .color(theme.text_secondary()),
                )
                .wrap(),
            );
        });
    });

    let resp = frame_resp.response.interact(Sense::click());
    if resp.hovered() && !is_active {
        ui.painter().rect_stroke(
            frame_resp.response.rect,
            10.0,
            Stroke::new(1.0_f32, theme.accent_blue().gamma_multiply(0.40)),
            egui::StrokeKind::Inside,
        );
    }
    resp
}

/// Renderiza el catálogo de transmisores disponibles en Desktop
pub fn render_desktop_pipeline_list(
    ui: &mut Ui,
    theme: &AppTheme,
    state: &mut DesktopNavState,
    selected_pipeline: &mut PipelineId,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Transmisores").strong().size(14.5).color(theme.text_primary()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let count = PipelineId::all().len();
                ui.label(RichText::new(format!("{} disp.", count)).size(10.5).color(theme.text_secondary()));
            });
        });
        ui.add_space(6.0);

        // Barra de búsqueda M3
        render_m3_search_bar(ui, theme, &mut state.search_query);
        ui.add_space(6.0);

        // Chips de filtro rápido
        render_m3_filter_chips(ui, theme, &mut state.filter_category);
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
            let query = state.search_query.trim().to_lowercase();
            let pipelines = PipelineId::all();

            let mut matched_count = 0;
            for &pipe in pipelines {
                // Filtro por categoría chip
                if !state.filter_category.matches(pipe) {
                    continue;
                }

                // Filtro de texto de búsqueda
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
                if render_pipeline_card(ui, theme, pipe, is_active).clicked() {
                    *selected_pipeline = pipe;
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
