use egui::{Color32, Visuals};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTheme {
    Dark,
    Light,
}

impl AppTheme {
    pub fn apply(&self, ctx: &egui::Context) {
        match self {
            AppTheme::Dark => {
                let mut visuals = Visuals::dark();
                visuals.window_fill = Color32::from_rgb(15, 18, 23);
                visuals.panel_fill = Color32::from_rgb(18, 22, 28);
                visuals.faint_bg_color = Color32::from_rgb(26, 32, 42);
                visuals.extreme_bg_color = Color32::from_rgb(10, 12, 16);
                visuals.override_text_color = Some(Color32::from_rgb(240, 245, 255));
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 30, 38);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(32, 40, 52);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 54, 70);
                visuals.widgets.active.bg_fill = Color32::from_rgb(52, 68, 88);
                ctx.set_visuals(visuals);
            }
            AppTheme::Light => {
                let mut visuals = Visuals::light();
                visuals.window_fill = Color32::from_rgb(248, 250, 252);
                visuals.panel_fill = Color32::from_rgb(241, 245, 249);
                visuals.faint_bg_color = Color32::from_rgb(235, 240, 246);
                visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255);
                visuals.override_text_color = Some(Color32::from_rgb(15, 23, 42));
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(241, 245, 249);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(226, 232, 240);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(203, 213, 225);
                visuals.widgets.active.bg_fill = Color32::from_rgb(191, 219, 254);
                ctx.set_visuals(visuals);
            }
        }

        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            ctx.style_mut_of(theme, |style| {
                style.wrap_mode = Some(egui::TextWrapMode::Wrap);
                style.spacing.button_padding = egui::vec2(12.0, 7.0);
                style.spacing.item_spacing = egui::vec2(10.0, 8.0);
                style.visuals.widgets.noninteractive.corner_radius = 8.0.into();
                style.visuals.widgets.inactive.corner_radius = 8.0.into();
                style.visuals.widgets.hovered.corner_radius = 8.0.into();
                style.visuals.widgets.active.corner_radius = 8.0.into();
                style.visuals.widgets.open.corner_radius = 8.0.into();
            });
        }
    }

    /// Superficie estándar de tarjetas y contenedores elevados (M3 Surface Container)
    pub fn card_bg(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(24, 30, 40),
            AppTheme::Light => Color32::from_rgb(255, 255, 255),
        }
    }

    pub fn surface_container(&self) -> Color32 {
        self.card_bg()
    }

    /// Superficie baja para rails, headers y barras de herramientas (M3 Surface Container Low)
    pub fn surface_subtle(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(17, 21, 28),
            AppTheme::Light => Color32::from_rgb(241, 245, 249),
        }
    }

    pub fn surface_container_low(&self) -> Color32 {
        self.surface_subtle()
    }

    /// Superficie alta para inputs de búsqueda, chips y estados hover (M3 Surface Container High)
    pub fn surface_container_high(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(34, 42, 56),
            AppTheme::Light => Color32::from_rgb(226, 232, 240),
        }
    }

    /// Fondo base de canvas / viewport (M3 Background)
    pub fn bg_canvas(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(12, 15, 20),
            AppTheme::Light => Color32::from_rgb(248, 250, 252),
        }
    }

    /// Borde sutil de 1px (M3 Outline Variant)
    pub fn card_border(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(44, 54, 70),
            AppTheme::Light => Color32::from_rgb(220, 228, 238),
        }
    }

    pub fn border_subtle(&self) -> Color32 {
        self.card_border()
    }

    /// Píldora de selección activa (M3 Secondary Container / Pill Indicator)
    pub fn active_pill(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(32, 56, 88),
            AppTheme::Light => Color32::from_rgb(219, 234, 254),
        }
    }

    pub fn accent_blue(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(88, 166, 255),
            AppTheme::Light => Color32::from_rgb(37, 99, 235),
        }
    }

    pub fn active_indicator(&self) -> Color32 {
        self.accent_blue()
    }

    pub fn info_card_bg(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(20, 26, 36),
            AppTheme::Light => Color32::from_rgb(240, 246, 254),
        }
    }

    pub fn info_card_border(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(38, 58, 88),
            AppTheme::Light => Color32::from_rgb(190, 215, 245),
        }
    }

    pub fn text_primary(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(240, 245, 255),
            AppTheme::Light => Color32::from_rgb(15, 23, 42),
        }
    }

    pub fn text_secondary(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(150, 165, 185),
            AppTheme::Light => Color32::from_rgb(100, 116, 139),
        }
    }

    pub fn apply_to_context(&self, ctx: &egui::Context) {
        self.apply(ctx);
    }

    pub fn accent_green(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(52, 211, 153),
            AppTheme::Light => Color32::from_rgb(16, 185, 129),
        }
    }

    pub fn accent_red(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(248, 113, 113),
            AppTheme::Light => Color32::from_rgb(239, 68, 68),
        }
    }
}
