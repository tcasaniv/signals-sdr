use crate::config::AppConfig;
use crate::pipelines::{PipelineId, PipelinesState};
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::desktop_nav::{
    render_desktop_pipeline_list, render_desktop_rail, DesktopNavState, NavSection,
};
use crate::ui::mobile_nav::{
    render_mobile_bottom_bar, render_mobile_header, render_mobile_pipeline_grid, MobileNavState,
};
use egui::{
    CentralPanel, Context, Frame, RichText, ScrollArea, SidePanel, Stroke, TopBottomPanel, Ui,
};

pub struct SignalsSdrApp {
    pub config: AppConfig,
    pub dark_mode: bool,
    pub selected_pipeline: PipelineId,
    pub desktop_nav: DesktopNavState,
    pub mobile_nav: MobileNavState,
    pub pipelines: PipelinesState,
    pub sdr_controller: SdrTransmissionController,
}

impl Default for SignalsSdrApp {
    fn default() -> Self {
        let config = AppConfig::load();
        let dark_mode = config.is_dark_mode;
        Self {
            config,
            dark_mode,
            selected_pipeline: PipelineId::NoaaApt,
            desktop_nav: DesktopNavState::default(),
            mobile_nav: MobileNavState::default(),
            pipelines: PipelinesState::default(),
            sdr_controller: SdrTransmissionController::new(),
        }
    }
}

impl eframe::App for SignalsSdrApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let theme = if self.dark_mode {
            AppTheme::Dark
        } else {
            AppTheme::Light
        };
        let c = theme.bg_canvas();
        [
            c.r() as f32 / 255.0,
            c.g() as f32 / 255.0,
            c.b() as f32 / 255.0,
            1.0,
        ]
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let theme = if self.dark_mode {
            AppTheme::Dark
        } else {
            AppTheme::Light
        };

        // Actualizar configuración si cambia el tema
        if self.config.is_dark_mode != self.dark_mode {
            self.config.is_dark_mode = self.dark_mode;
            self.config.save();
        }

        theme.apply_to_context(ctx);

        let screen_width = ctx.viewport_rect().width();
        let is_mobile = screen_width < 720.0;

        if is_mobile {
            self.render_mobile_ui(ctx, &theme);
        } else {
            self.render_desktop_ui(ctx, &theme);
        }
    }
}

impl SignalsSdrApp {
    fn render_mobile_ui(&mut self, ctx: &Context, theme: &AppTheme) {
        // Barra superior móvil con relleno continuo del tema (sin áreas negras)
        TopBottomPanel::top("mobile_header")
            .frame(
                Frame::new()
                    .fill(theme.surface_subtle())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                render_mobile_header(
                    ui,
                    theme,
                    &mut self.mobile_nav,
                    self.desktop_nav.active_section,
                    self.selected_pipeline,
                );
            });

        // Barra inferior móvil con relleno continuo del tema
        TopBottomPanel::bottom("mobile_bottom")
            .frame(
                Frame::new()
                    .fill(theme.surface_subtle())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .inner_margin(6.0),
            )
            .show(ctx, |ui| {
                render_mobile_bottom_bar(
                    ui,
                    theme,
                    &mut self.desktop_nav.active_section,
                    &mut self.mobile_nav,
                );
            });

        // Contenido central móvil
        CentralPanel::default()
            .frame(Frame::new().fill(theme.bg_canvas()).inner_margin(10.0))
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    match self.desktop_nav.active_section {
                        NavSection::Settings => self.render_settings(ui, theme),
                        NavSection::Pipelines => {
                            if self.mobile_nav.showing_detail {
                                self.render_active_pipeline(ui, theme);
                            } else {
                                render_mobile_pipeline_grid(
                                    ui,
                                    theme,
                                    &mut self.selected_pipeline,
                                    &mut self.mobile_nav,
                                );
                            }
                        }
                    }
                });
            });
    }

    fn render_desktop_ui(&mut self, ctx: &Context, theme: &AppTheme) {
        // 1. Icon Rail estrecho (ancho fijo 64px, ocupa 100% de la altura de la ventana)
        SidePanel::left("desktop_nav_rail")
            .resizable(false)
            .exact_width(64.0)
            .frame(
                Frame::new()
                    .fill(theme.surface_subtle())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                render_desktop_rail(ui, theme, &mut self.desktop_nav);
            });

        // 2. Panel intermedio de catálogo/pipelines (ancho 260px) solo si la sección activa es Pipelines
        if self.desktop_nav.active_section == NavSection::Pipelines {
            SidePanel::left("desktop_catalog")
                .resizable(false)
                .exact_width(260.0)
                .frame(
                    Frame::new()
                        .fill(theme.card_bg())
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .inner_margin(12.0),
                )
                .show(ctx, |ui| {
                    render_desktop_pipeline_list(
                        ui,
                        theme,
                        &mut self.desktop_nav,
                        &mut self.selected_pipeline,
                    );
                });
        }

        // 3. Panel central principal (ocupa toda la altura y el ancho restante del viewport)
        CentralPanel::default()
            .frame(Frame::new().fill(theme.bg_canvas()).inner_margin(16.0))
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    match self.desktop_nav.active_section {
                        NavSection::Settings => self.render_settings(ui, theme),
                        NavSection::Pipelines => self.render_active_pipeline(ui, theme),
                    }
                });
            });
    }

    fn render_active_pipeline(&mut self, ui: &mut Ui, theme: &AppTheme) {
        match self.selected_pipeline {
            PipelineId::NoaaApt => {
                self.pipelines.apt.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
            PipelineId::MeteorLrpt => {
                self.pipelines.lrpt.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
            PipelineId::IssSstv => {
                self.pipelines.iss_sstv.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
            PipelineId::GenericSstv => {
                self.pipelines.generic_sstv.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
            PipelineId::GenericFm => {
                self.pipelines.generic_fm.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
            PipelineId::GenericAm => {
                self.pipelines.generic_am.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
            PipelineId::GenericIq => {
                self.pipelines.generic_iq.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
            PipelineId::NoaaHrpt => {
                self.pipelines.hrpt.render(ui, theme, &self.config, &mut self.sdr_controller);
            }
        }
    }

    fn render_settings(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.label(RichText::new("Ajustes y Configuración Global").strong().size(18.0).color(theme.text_primary()));
            ui.label(
                RichText::new("Configure el tema de la aplicación y las rutas predeterminadas para almacenamiento de archivos procesados y señales I/Q.")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(14.0);

            // Tarjeta de apariencia y tema visual
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Apariencia y Tema Visual").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Seleccione el modo visual para la aplicación:")
                                .size(12.0)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui.selectable_label(self.dark_mode, "Modo Oscuro").clicked() {
                                self.dark_mode = true;
                            }
                            if ui.selectable_label(!self.dark_mode, "Modo Claro").clicked() {
                                self.dark_mode = false;
                            }
                        });
                    });
                });

            ui.add_space(14.0);

            // Tarjeta de almacenamiento
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Carpeta Base de Exportación y Señales").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(
                                "Los archivos preprocesados, audios intermedios y archivos I/Q se organizarán automáticamente en subcarpetas dentro de esta ruta:",
                            )
                            .size(12.0)
                            .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.add(
                            egui::Label::new(
                                RichText::new(self.config.default_output_dir.display().to_string())
                                    .monospace()
                                    .size(11.5)
                                    .color(theme.text_primary()),
                            )
                            .wrap(),
                        );
                        ui.add_space(4.0);

                        if ui.button("Cambiar Carpeta...").clicked() {
                            if let Some(new_dir) = rfd::FileDialog::new().pick_folder() {
                                self.config.default_output_dir = new_dir;
                                self.config.ensure_directories_exist();
                                self.config.save();
                            }
                        }

                        ui.add_space(10.0);
                        ui.label(RichText::new("Estructura de subcarpetas creadas automáticamente:").strong().size(12.0));
                        let subdirs = [
                            ("noaa_apt/", "Imágenes preprocesadas a 909px, compuestos APT y audios AM 2400 Hz"),
                            ("meteor_lrpt/", "Imágenes RGB/IR, tramas CCSDS 72k y archivos I/Q OQPSK"),
                            ("iss_sstv/", "Imágenes de la ISS y señales moduladas en Robot 36 / PD 120"),
                            ("generic_sstv/", "Imágenes y audios SSTV en modos Martin, Scottie, Robot, PD, BW"),
                            ("generic_fm/", "Señales FM banda ancha y banda estrecha"),
                            ("generic_am/", "Señales AM DSB-FC, DSB-SC y SSB"),
                            ("generic_iq/", "Archivos I/Q convertidos o acondicionados"),
                        ];

                        for (sub, desc) in subdirs {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(format!("* {}", sub)).monospace().strong().color(theme.accent_blue()));
                                ui.add(egui::Label::new(RichText::new(desc).size(11.5).color(theme.text_secondary())).wrap());
                            });
                            ui.add_space(2.0);
                        }
                    });
                });

            ui.add_space(14.0);

            // Tarjeta de aspectos legales y didácticos de transmisión SDR
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Aviso Legal y Uso Responsable de Radiofrecuencia").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    "Signals SDR ha sido diseñado exclusivamente para fines formativos, de investigación académica \
                                     y experimentación de radioaficionados autorizados. \
                                     Al transmitir con dispositivos SDR (HackRF, bladeRF, USRP):",
                                )
                                .size(12.0),
                            )
                            .wrap(),
                        );
                        ui.add_space(4.0);
                        ui.add(egui::Label::new(RichText::new("• Verifique las regulaciones de telecomunicaciones de su país.").size(12.0)).wrap());
                        ui.add(egui::Label::new(RichText::new("• Para pruebas de laboratorio sin licencia, utilice siempre cargas ficticias (Dummy Load) o atenuadores coaxiales directos.").size(12.0)).wrap());
                        ui.add(egui::Label::new(RichText::new("• No transmita en frecuencias satelitales oficiales sobre el aire sin la debida coordinación.").size(12.0)).wrap());
                    });
                });
        });
    }
}
