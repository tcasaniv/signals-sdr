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
                visuals.window_fill = Color32::from_rgb(20, 24, 30);
                visuals.panel_fill = Color32::from_rgb(15, 18, 23);
                visuals.faint_bg_color = Color32::from_rgb(26, 32, 40);
                visuals.extreme_bg_color = Color32::from_rgb(10, 12, 16);
                visuals.override_text_color = Some(Color32::from_rgb(230, 235, 245));
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(25, 30, 38);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(32, 38, 48);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 54, 68);
                visuals.widgets.active.bg_fill = Color32::from_rgb(55, 68, 86);
                ctx.set_visuals(visuals);
            }
            AppTheme::Light => {
                let mut visuals = Visuals::light();
                visuals.window_fill = Color32::from_rgb(248, 250, 252);
                visuals.panel_fill = Color32::from_rgb(241, 245, 249);
                visuals.faint_bg_color = Color32::from_rgb(235, 240, 246);
                visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255);
                visuals.override_text_color = Some(Color32::from_rgb(30, 41, 59));
                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(238, 242, 246);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(226, 232, 240);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(203, 213, 225);
                visuals.widgets.active.bg_fill = Color32::from_rgb(180, 195, 215);
                ctx.set_visuals(visuals);
            }
        }
    }

    pub fn card_bg(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(26, 32, 42),
            AppTheme::Light => Color32::from_rgb(255, 255, 255),
        }
    }

    pub fn card_border(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(48, 58, 72),
            AppTheme::Light => Color32::from_rgb(215, 225, 235),
        }
    }

    pub fn accent_blue(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(80, 160, 255),
            AppTheme::Light => Color32::from_rgb(37, 99, 235),
        }
    }

    pub fn active_indicator(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(60, 140, 240),
            AppTheme::Light => Color32::from_rgb(37, 99, 235),
        }
    }

    pub fn info_card_bg(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(22, 28, 38),
            AppTheme::Light => Color32::from_rgb(240, 246, 254),
        }
    }

    pub fn info_card_border(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(40, 60, 90),
            AppTheme::Light => Color32::from_rgb(190, 215, 245),
        }
    }

    pub fn text_primary(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(240, 245, 255),
            AppTheme::Light => Color32::from_rgb(15, 23, 42),
        }
    }

    pub fn apply_to_context(&self, ctx: &egui::Context) {
        self.apply(ctx);
    }

    pub fn surface_subtle(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(18, 22, 28),
            AppTheme::Light => Color32::from_rgb(241, 245, 249),
        }
    }

    pub fn bg_canvas(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(14, 17, 22),
            AppTheme::Light => Color32::from_rgb(248, 250, 252),
        }
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

    pub fn text_secondary(&self) -> Color32 {
        match self {
            AppTheme::Dark => Color32::from_rgb(160, 175, 195),
            AppTheme::Light => Color32::from_rgb(100, 116, 139),
        }
    }
}
