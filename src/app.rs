use crate::config::AppConfig;
use crate::pipelines::{PipelineId, PipelinesState};
use crate::sdr::SdrTransmissionController;
use crate::theme::AppTheme;
use crate::ui::desktop_nav::{
    render_desktop_pipeline_list, render_desktop_rail, render_desktop_settings_list, DesktopNavState,
    NavSection, SettingsTab,
};
use crate::ui::mobile_nav::{
    render_mobile_bottom_bar, render_mobile_header, render_mobile_pipeline_grid, MobileNavState,
};
use egui::{
    CentralPanel, Color32, Frame, Panel, RichText, ScrollArea, Stroke, Ui, Vec2,
};

pub struct SignalsSdrApp {
    pub config: AppConfig,
    pub dark_mode: bool,
    pub selected_pipeline: PipelineId,
    pub previous_pipeline: Option<PipelineId>,
    pub desktop_nav: DesktopNavState,
    pub mobile_nav: MobileNavState,
    pub pipelines: PipelinesState,
    pub sdr_controller: SdrTransmissionController,
}

impl Default for SignalsSdrApp {
    fn default() -> Self {
        let config = AppConfig::load();
        let dark_mode = config.is_dark_mode;
        let mut desktop_nav = DesktopNavState::default();
        if let Some(ref path) = config.soapy_search_path {
            desktop_nav.soapy_path_input = path.clone();
        }
        Self {
            config,
            dark_mode,
            selected_pipeline: PipelineId::NoaaApt,
            previous_pipeline: Some(PipelineId::NoaaApt),
            desktop_nav,
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

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
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

        theme.apply_to_context(ui.ctx());

        // Liberar recursos de pipelines inactivos si cambió el pipeline seleccionado
        if self.previous_pipeline != Some(self.selected_pipeline) {
            self.pipelines.release_inactive(self.selected_pipeline);
            self.previous_pipeline = Some(self.selected_pipeline);
        }

        let screen_width = ui.ctx().viewport_rect().width();
        let is_mobile = screen_width < 840.0;

        if is_mobile {
            self.render_mobile_ui(ui, &theme);
        } else {
            self.render_desktop_ui(ui, &theme);
        }
    }
}

impl SignalsSdrApp {
    fn render_mobile_ui(&mut self, ui: &mut Ui, theme: &AppTheme) {
        // Barra superior móvil con relleno continuo del tema (sin áreas negras)
        Panel::top("mobile_header")
            .frame(
                Frame::new()
                    .fill(theme.surface_container_low())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .inner_margin(8.0),
            )
            .show(ui, |ui| {
                render_mobile_header(
                    ui,
                    theme,
                    &mut self.mobile_nav,
                    self.desktop_nav.active_section,
                    self.selected_pipeline,
                    self.desktop_nav.active_settings_tab,
                );
            });

        // Barra inferior móvil con relleno continuo del tema
        Panel::bottom("mobile_bottom")
            .frame(
                Frame::new()
                    .fill(theme.surface_container_low())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .inner_margin(4.0),
            )
            .show(ui, |ui| {
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
            .show(ui, |ui| {
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match self.desktop_nav.active_section {
                        NavSection::Settings => {
                            if self.mobile_nav.showing_detail {
                                self.render_settings_detail(ui, theme);
                            } else {
                                crate::ui::mobile_nav::render_mobile_settings_grid(
                                    ui,
                                    theme,
                                    &mut self.desktop_nav,
                                    &mut self.mobile_nav,
                                );
                            }
                        }
                        NavSection::Pipelines => {
                            if self.mobile_nav.showing_detail {
                                self.render_active_pipeline(ui, theme);
                            } else {
                                render_mobile_pipeline_grid(
                                    ui,
                                    theme,
                                    &mut self.selected_pipeline,
                                    &mut self.mobile_nav,
                                    &mut self.desktop_nav,
                                );
                            }
                        }
                    }
                });
            });
    }

    fn render_desktop_ui(&mut self, ui: &mut Ui, theme: &AppTheme) {
        // 1. Navigation Rail estrecho (ancho fijo 72px estilo Material 3)
        Panel::left("desktop_nav_rail")
            .resizable(false)
            .exact_size(72.0)
            .frame(
                Frame::new()
                    .fill(theme.surface_container_low())
                    .stroke(Stroke::new(1.0_f32, theme.card_border()))
                    .inner_margin(6.0),
            )
            .show(ui, |ui| {
                render_desktop_rail(ui, theme, &mut self.desktop_nav, &mut self.dark_mode);
            });

        // 2. Panel flexible secundario (solo si side_panel_expanded es true)
        if self.desktop_nav.side_panel_expanded {
            Panel::left("desktop_catalog")
                .resizable(true)
                .default_size(240.0)
                .min_size(180.0)
                .max_size(360.0)
                .frame(
                    Frame::new()
                        .fill(theme.surface_container_low())
                        .stroke(Stroke::new(1.0_f32, theme.card_border()))
                        .inner_margin(10.0),
                )
                .show(ui, |ui| {
                    match self.desktop_nav.active_section {
                        NavSection::Pipelines => {
                            render_desktop_pipeline_list(
                                ui,
                                theme,
                                &mut self.desktop_nav,
                                &mut self.selected_pipeline,
                            );
                        }
                        NavSection::Settings => {
                            render_desktop_settings_list(
                                ui,
                                theme,
                                &mut self.desktop_nav,
                            );
                        }
                    }
                });
        }

        // 3. Panel central principal (ocupa toda la altura y el ancho restante del viewport)
        CentralPanel::default()
            .frame(Frame::new().fill(theme.bg_canvas()).inner_margin(16.0))
            .show(ui, |ui| {
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match self.desktop_nav.active_section {
                        NavSection::Settings => self.render_settings_detail(ui, theme),
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

    fn render_settings_detail(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            match self.desktop_nav.active_settings_tab {
                SettingsTab::General => self.render_settings_general(ui, theme),
                SettingsTab::Appearance => self.render_settings_appearance(ui, theme),
                SettingsTab::SoapySdr => self.render_settings_soapy(ui, theme),
                SettingsTab::License => self.render_settings_license(ui, theme),
                SettingsTab::About => self.render_settings_about(ui, theme),
            }
        });
    }

    fn render_settings_appearance(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Apariencia y Tema Visual").strong().size(18.0).color(theme.text_primary()));
            ui.label(
                RichText::new("Personalice el modo de color y contraste visual de la interfaz.")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(14.0);

            // Tarjeta de modo de color
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Modo de Color").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Seleccione el tema visual para el entorno de trabajo:")
                                .size(12.0)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            let dark_label = format!("{} Modo Oscuro", crate::icons::ICON_DARK_MODE.codepoint);
                            if ui.selectable_label(self.dark_mode, dark_label).clicked() {
                                self.dark_mode = true;
                                self.config.is_dark_mode = true;
                                self.config.save();
                            }
                            let light_label = format!("{} Modo Claro", crate::icons::ICON_LIGHT_MODE.codepoint);
                            if ui.selectable_label(!self.dark_mode, light_label).clicked() {
                                self.dark_mode = false;
                                self.config.is_dark_mode = false;
                                self.config.save();
                            }
                        });
                    });
                });
        });
    }

    fn render_settings_general(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Rutas y Salidas").strong().size(18.0).color(theme.text_primary()));
            ui.label(
                RichText::new("Configure el almacenamiento de señales y exportaciones en disco.")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(14.0);

            // Tarjeta de almacenamiento
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
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
                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            if ui.button(format!("{} Cambiar Carpeta...", crate::icons::ICON_FOLDER_OPEN.codepoint)).clicked() {
                                if let Some(new_dir) = crate::file_dialog::FileDialog::new().pick_folder() {
                                    self.config.default_output_dir = new_dir;
                                    self.config.ensure_directories_exist();
                                    self.config.save();
                                }
                            }

                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if ui.button(format!("{} Abrir Carpeta Base", crate::icons::ICON_FOLDER.codepoint)).clicked() {
                                    let _ = crate::utils::open_folder(&self.config.default_output_dir);
                                }
                            }
                        });

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
                    ui.set_width(ui.available_width());
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
                        ui.add(egui::Label::new(RichText::new("- Verifique las regulaciones de telecomunicaciones de su país.").size(12.0)).wrap());
                        ui.add(egui::Label::new(RichText::new("- Para pruebas de laboratorio sin licencia, utilice siempre cargas ficticias (Dummy Load) o atenuadores coaxiales directos.").size(12.0)).wrap());
                        ui.add(egui::Label::new(RichText::new("- No transmita en frecuencias satelitales oficiales sobre el aire sin la debida coordinación.").size(12.0)).wrap());
                    });
                });
        });
    }

    fn render_settings_soapy(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Controlador SoapySDR (Radioconda / PothosSDR)").strong().size(18.0).color(theme.text_primary()));
            ui.label(
                RichText::new("Configuración de rutas dinámicas de SoapySDR para la detección de dispositivos SDR en Windows.")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(14.0);

            let is_available = crate::sdr::is_soapy_available();
            let status_bg = if is_available {
                if *theme == AppTheme::Dark { Color32::from_rgb(10, 32, 18) } else { Color32::from_rgb(235, 250, 240) }
            } else {
                if *theme == AppTheme::Dark { Color32::from_rgb(32, 24, 10) } else { Color32::from_rgb(254, 249, 235) }
            };
            let status_stroke = if is_available {
                Color32::from_rgb(34, 197, 94)
            } else {
                Color32::from_rgb(234, 179, 8)
            };

            Frame::new()
                .fill(status_bg)
                .stroke(Stroke::new(1.0, status_stroke))
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let icon = if is_available { crate::icons::ICON_CHECK_CIRCLE.codepoint } else { crate::icons::ICON_INFO.codepoint };
                        ui.label(RichText::new(icon).size(18.0).color(status_stroke));
                        ui.vertical(|ui| {
                            if is_available {
                                ui.label(RichText::new("SoapySDR activo y vinculado correctamente").strong().color(status_stroke));
                                ui.label(RichText::new("Las librerías dinámicas están disponibles en memoria. Seify puede detectar y transmitir con PlutoSDR, LimeSDR, bladeRF y USRP.").size(12.0).color(theme.text_secondary()));
                            } else {
                                ui.label(RichText::new("SoapySDR no detectado en el sistema actualmente").strong().color(status_stroke));
                                ui.label(RichText::new("No se encontró SoapySDR.dll en el PATH ni en las carpetas estándar. Si vas a usar transceptores SoapySDR (PlutoSDR, LimeSDR, bladeRF), instala PothosSDR o Radioconda con 'scripts/instalar-controladores-sdr.bat' o indica la ruta abajo. Para HackRF One nativo por USB, simulador Dummy y RTL-TCP por red, ¡no necesitas instalar nada!").size(12.0).color(theme.text_secondary()));
                            }
                        });
                    });
                });

            ui.add_space(12.0);

            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Ruta de Bibliotecas SoapySDR (Windows)").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    "En Windows, los controladores SDR de SoapySDR se instalan habitualmente a través de PothosSDR \
                                     (ej. 'C:\\Program Files\\PothosSDR\\bin') o Radioconda (ej. 'C:\\Users\\<usuario>\\radioconda\\Library\\bin'). \
                                     Estas carpetas contienen SoapySDR.dll y los módulos para PlutoSDR, LimeSDR, HackRF, Airspy, RTL-SDR y bladeRF. \
                                     Si no están en el PATH global, Signals SDR puede detectarlas y cargarlas automáticamente en tiempo de ejecución."
                                )
                                .size(12.0)
                                .color(theme.text_secondary())
                            )
                            .wrap()
                        );
                        ui.add_space(10.0);

                        // Botón principal destacado para detectar Radioconda y PothosSDR
                        ui.horizontal_wrapped(|ui| {
                            let scan_btn = egui::Button::new(
                                RichText::new(format!("{} Detectar Radioconda y PothosSDR en el Sistema", crate::icons::ICON_SEARCH.codepoint))
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE)
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(scan_btn).clicked() {
                                let active_ref = self.config.soapy_search_path.as_deref();
                                self.desktop_nav.detected_soapy_installations = scan_all_soapy_installations(active_ref);
                                if self.desktop_nav.detected_soapy_installations.is_empty() {
                                    self.desktop_nav.soapy_test_status = Some((
                                        "No se encontraron instalaciones automáticas de Radioconda ni PothosSDR en las rutas estándar. Puedes indicar la carpeta manualmente abajo.".to_string(),
                                        false,
                                    ));
                                } else {
                                    self.desktop_nav.soapy_test_status = Some((
                                        format!("¡Se encontraron {} instalación(es) de SoapySDR en su equipo! Seleccione cuál desea utilizar.", self.desktop_nav.detected_soapy_installations.len()),
                                        true,
                                    ));
                                }
                            }
                        });

                        ui.add_space(10.0);
                        ui.label(RichText::new("Instalaciones Detectadas en el Sistema").strong().size(13.5).color(theme.accent_blue()));
                        ui.add_space(4.0);

                        // Escaneo automático inicial
                        if !self.desktop_nav.soapy_scan_done {
                            self.desktop_nav.soapy_scan_done = true;
                            let active_ref = self.config.soapy_search_path.as_deref();
                            self.desktop_nav.detected_soapy_installations = scan_all_soapy_installations(active_ref);
                        }

                        let mut selected_to_use: Option<(String, String)> = None;

                        if self.desktop_nav.detected_soapy_installations.is_empty() {
                            ui.label(
                                RichText::new("No se han escaneado instalaciones aún o no se encontraron en las rutas habituales. Haz clic en 'Detectar Radioconda y PothosSDR en el Sistema' o ingresa la ruta manual abajo.")
                                    .size(11.5)
                                    .color(theme.text_secondary())
                            );
                        } else {
                            ui.label(
                                RichText::new("Selecciona cuál de las instalaciones detectadas deseas activar para transmitir con SoapySDR:")
                                    .size(11.5)
                                    .color(theme.text_secondary())
                            );
                            ui.add_space(4.0);

                            for inst in &self.desktop_nav.detected_soapy_installations {
                                let is_active = inst.is_active || self.config.soapy_search_path.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(&inst.path));
                                Frame::new()
                                    .fill(if is_active { theme.surface_container_high() } else { theme.surface_subtle() })
                                    .stroke(Stroke::new(1.0, if is_active { theme.accent_blue() } else { theme.card_border() }))
                                    .corner_radius(6.0)
                                    .inner_margin(10.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let icon = if inst.name.to_lowercase().contains("pothos") {
                                                crate::icons::ICON_RADIO.codepoint
                                            } else {
                                                crate::icons::ICON_TUNE.codepoint
                                            };
                                            ui.label(RichText::new(icon).size(18.0).color(if is_active { theme.accent_blue() } else { theme.text_secondary() }));

                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new(&inst.name).strong().size(13.0));
                                                    if is_active {
                                                        ui.label(RichText::new("[ ✓ ACTIVO EN ESTA SESIÓN ]").strong().size(10.5).color(Color32::from_rgb(34, 197, 94)));
                                                    }
                                                });
                                                ui.label(RichText::new(&inst.path).size(11.0).monospace().color(theme.text_secondary()));
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if !is_active {
                                                    let use_btn = egui::Button::new(
                                                        RichText::new("Seleccionar y Usar").size(11.5).strong().color(Color32::WHITE)
                                                    )
                                                    .fill(theme.accent_blue())
                                                    .corner_radius(6.0);

                                                    if ui.add(use_btn).clicked() {
                                                        selected_to_use = Some((inst.name.clone(), inst.path.clone()));
                                                    }
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(6.0);
                            }
                        }

                        if let Some((name, path)) = selected_to_use {
                            self.desktop_nav.soapy_path_input = path.clone();
                            self.config.soapy_search_path = Some(path.clone());
                            self.config.apply_environment();
                            self.config.save();
                            for item in &mut self.desktop_nav.detected_soapy_installations {
                                item.is_active = item.path.eq_ignore_ascii_case(&path);
                            }
                            self.desktop_nav.soapy_test_status = Some((
                                format!("¡Controlador cambiado a {} y aplicado al entorno de ejecución con éxito!: {}", name, path),
                                true,
                            ));
                        }

                        ui.add_space(10.0);
                        ui.label(RichText::new("Ruta manual personalizada:").strong().size(12.5));
                        ui.label(RichText::new("Si tienes una versión personalizada de SoapySDR en otra carpeta o disco:").size(11.5).color(theme.text_secondary()));
                        ui.add_space(4.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.desktop_nav.soapy_path_input)
                                    .hint_text("Ej: C:\\Program Files\\PothosSDR\\bin o C:\\Users\\USUARIO\\radioconda\\Library\\bin")
                                    .desired_width((ui.available_width() - 320.0).clamp(180.0, 420.0)),
                            );

                            let browse_btn = egui::Button::new(
                                RichText::new(format!("{} Examinar Carpeta...", crate::icons::ICON_FOLDER_OPEN.codepoint)).size(12.0)
                            )
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0);

                            if ui.add(browse_btn).clicked() {
                                if let Some(folder) = crate::file_dialog::FileDialog::new().pick_folder() {
                                    let mut resolved = folder;
                                    if !resolved.join("SoapySDR.dll").exists() && !resolved.join("SoapySDRUtil.exe").exists() {
                                        if resolved.join("Library").join("bin").join("SoapySDR.dll").exists() {
                                            resolved = resolved.join("Library").join("bin");
                                        } else if resolved.join("bin").join("SoapySDR.dll").exists() {
                                            resolved = resolved.join("bin");
                                        }
                                    }
                                    self.desktop_nav.soapy_path_input = resolved.display().to_string();
                                }
                            }

                            let save_btn = egui::Button::new(
                                RichText::new(format!("{} Guardar y Aplicar", crate::icons::ICON_SAVE.codepoint))
                                    .strong()
                                    .size(12.0)
                                    .color(Color32::WHITE)
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0);

                            if ui.add(save_btn).clicked() {
                                let mut path_to_save = self.desktop_nav.soapy_path_input.trim().to_string();
                                if !path_to_save.is_empty() {
                                    let p = std::path::PathBuf::from(&path_to_save);
                                    if !p.join("SoapySDR.dll").exists() && !p.join("SoapySDRUtil.exe").exists() {
                                        if p.join("Library").join("bin").join("SoapySDR.dll").exists() {
                                            path_to_save = p.join("Library").join("bin").display().to_string();
                                            self.desktop_nav.soapy_path_input = path_to_save.clone();
                                        } else if p.join("bin").join("SoapySDR.dll").exists() {
                                            path_to_save = p.join("bin").display().to_string();
                                            self.desktop_nav.soapy_path_input = path_to_save.clone();
                                        }
                                    }
                                    self.config.soapy_search_path = Some(path_to_save);
                                } else {
                                    self.config.soapy_search_path = None;
                                }
                                self.config.apply_environment();
                                self.config.save();
                                self.desktop_nav.soapy_test_status = Some(("Ruta guardada y aplicada al entorno de ejecución.".to_string(), true));
                            }

                            let test_btn = egui::Button::new(
                                RichText::new(format!("{} Probar Detección", crate::icons::ICON_TUNE.codepoint)).size(12.0)
                            )
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0);

                            if ui.add(test_btn).clicked() {
                                let (msg, is_ok) = test_soapy_detection(&self.desktop_nav.soapy_path_input);
                                self.desktop_nav.soapy_test_status = Some((msg, is_ok));
                            }
                        });

                        if let Some((ref msg, is_ok)) = self.desktop_nav.soapy_test_status {
                            ui.add_space(10.0);
                            let color = if is_ok {
                                Color32::from_rgb(34, 197, 94)
                            } else {
                                Color32::from_rgb(239, 68, 68)
                            };
                            let box_bg = if is_ok {
                                if *theme == AppTheme::Dark { Color32::from_rgb(10, 30, 20) } else { Color32::from_rgb(235, 250, 240) }
                            } else {
                                if *theme == AppTheme::Dark { Color32::from_rgb(35, 15, 15) } else { Color32::from_rgb(254, 242, 242) }
                            };
                            Frame::new()
                                .fill(box_bg)
                                .stroke(Stroke::new(1.0_f32, color))
                                .corner_radius(6.0)
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(RichText::new(msg).size(12.0).color(color)).wrap());
                                });
                        }
                    });
                });
        });
    }

    fn render_settings_license(&mut self, ui: &mut Ui, theme: &AppTheme) {
        const LICENSE_TEXT: &str = include_str!("../LICENSE");

        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Licencia de Software").strong().size(18.0).color(theme.text_primary()));
            ui.label(
                RichText::new("Signals SDR es software libre distribuido bajo los términos de la GNU General Public License v3 (GPLv3).")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(14.0);

            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} GNU General Public License v3", crate::icons::ICON_DESCRIPTION.codepoint))
                                    .strong()
                                    .size(15.0)
                                    .color(theme.accent_blue()),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let copy_btn = egui::Button::new(
                                    RichText::new(format!("{} Copiar Licencia Completa", crate::icons::ICON_CONTENT_COPY.codepoint)).size(12.0)
                                )
                                .fill(theme.surface_subtle())
                                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                                .corner_radius(6.0);

                                if ui.add(copy_btn).clicked() {
                                    ui.ctx().copy_text(LICENSE_TEXT.to_string());
                                    self.desktop_nav.license_copied_feedback = true;
                                }
                            });
                        });

                        if self.desktop_nav.license_copied_feedback {
                            ui.add_space(4.0);
                            ui.label(RichText::new("¡Texto completo de la licencia GPLv3 copiado al portapapeles!").size(12.0).color(Color32::from_rgb(34, 197, 94)));
                        }

                        ui.add_space(10.0);

                        let bg_box = if *theme == AppTheme::Dark { Color32::from_rgb(15, 18, 24) } else { Color32::from_rgb(243, 244, 246) };
                        Frame::new()
                            .fill(bg_box)
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ScrollArea::vertical()
                                    .max_height(480.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(LICENSE_TEXT)
                                                    .monospace()
                                                    .size(11.5)
                                                    .color(theme.text_primary())
                                            )
                                            .wrap()
                                        );
                                    });
                            });
                    });
                });
        });
    }

    fn render_settings_about(&mut self, ui: &mut Ui, theme: &AppTheme) {
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Acerca de Signals SDR").strong().size(18.0).color(theme.text_primary()));
            ui.label(
                RichText::new("Información del proyecto, repositorio de código abierto y contribuciones.")
                    .size(13.0)
                    .color(theme.text_secondary()),
            );
            ui.add_space(14.0);

            // Tarjeta de identidad
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new(egui::include_image!("../assets/icon_256.png"))
                                .fit_to_exact_size(egui::vec2(64.0, 64.0))
                                .corner_radius(10.0),
                        );
                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Signals SDR").strong().size(19.0).color(theme.text_primary()));
                                ui.label(
                                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                        .size(11.5)
                                        .color(theme.accent_blue())
                                        .monospace(),
                                );
                            });
                            ui.add_space(3.0);
                            ui.label(
                                RichText::new("Suite integral de DSP, telemetría satelital y transmisión SDR para radioaficionados y docencia.")
                                    .size(12.5)
                                    .color(theme.text_secondary()),
                            );
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Licencia: GPLv3").size(11.5).color(theme.text_secondary()));
                                ui.label(RichText::new("•").size(10.0).color(theme.text_secondary()));
                                ui.label(RichText::new("Escrito en Rust").size(11.5).color(theme.text_secondary()));
                            });
                        });
                    });
                });

            ui.add_space(14.0);

            // Tarjeta de enlaces GitHub
            Frame::new()
                .fill(theme.card_bg())
                .stroke(Stroke::new(1.0_f32, theme.card_border()))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Repositorio y Código Fuente").strong().size(14.0).color(theme.accent_blue()));
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("El código fuente completo de Signals SDR está disponible públicamente en GitHub:")
                                .size(12.0)
                                .color(theme.text_secondary()),
                        );
                        ui.add_space(10.0);

                        ui.horizontal_wrapped(|ui| {
                            let repo_btn = egui::Button::new(
                                RichText::new(format!("{} Abrir Repositorio en GitHub", crate::icons::ICON_OPEN_IN_NEW.codepoint))
                                    .strong()
                                    .size(12.5)
                                    .color(Color32::WHITE),
                            )
                            .fill(theme.accent_blue())
                            .corner_radius(6.0)
                            .min_size(Vec2::new(220.0, 34.0));

                            if ui.add(repo_btn).clicked() {
                                crate::utils::open_path_or_url("https://github.com/tcasaniv/signals-sdr");
                            }

                            let issues_btn = egui::Button::new(
                                RichText::new(format!("{} Reportar Problema en GitHub Issues", crate::icons::ICON_FEEDBACK.codepoint))
                                    .size(12.5),
                            )
                            .fill(theme.surface_subtle())
                            .stroke(Stroke::new(1.0_f32, theme.card_border()))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(220.0, 34.0));

                            if ui.add(issues_btn).clicked() {
                                crate::utils::open_path_or_url("https://github.com/tcasaniv/signals-sdr/issues");
                            }
                        });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("URL:").size(11.5).color(theme.text_secondary()));
                            if ui.link("https://github.com/tcasaniv/signals-sdr").clicked() {
                                crate::utils::open_path_or_url("https://github.com/tcasaniv/signals-sdr");
                            }
                        });
                    });
                });
        });
    }
}

fn scan_all_soapy_installations(current_active: Option<&str>) -> Vec<crate::ui::desktop_nav::SoapyInstallation> {
    let mut results = Vec::new();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut seen = std::collections::HashSet::new();
        let mut candidates: Vec<(String, &'static str)> = Vec::new();

        if let Ok(pothos_root) = std::env::var("POTHOS_ROOT") {
            candidates.push((format!("{}\\bin", pothos_root), "PothosSDR (Variable POTHOS_ROOT)"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            candidates.push((format!("{}\\PothosSDR\\bin", pf), "PothosSDR (Oficial 64-bit)"));
        }
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            candidates.push((format!("{}\\PothosSDR\\bin", pf86), "PothosSDR (Oficial 32-bit)"));
        }

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            candidates.push((format!("{}\\radioconda\\Library\\bin", userprofile), "Radioconda (Usuario - Recomendado)"));
            candidates.push((format!("{}\\miniconda3\\Library\\bin", userprofile), "Miniconda (Usuario)"));
            candidates.push((format!("{}\\anaconda3\\Library\\bin", userprofile), "Anaconda (Usuario)"));
            candidates.push((format!("{}\\PothosSDR\\bin", userprofile), "PothosSDR (Usuario)"));
        }

        candidates.push((r"C:\Program Files\PothosSDR\bin".to_string(), "PothosSDR (C:\\Program Files)"));
        candidates.push((r"C:\Program Files (x86)\PothosSDR\bin".to_string(), "PothosSDR (32-bit)"));
        candidates.push((r"C:\PothosSDR\bin".to_string(), "PothosSDR (Raíz C:)"));
        candidates.push((r"D:\Program Files\PothosSDR\bin".to_string(), "PothosSDR (D:\\Program Files)"));
        candidates.push((r"D:\PothosSDR\bin".to_string(), "PothosSDR (Raíz D:)"));
        candidates.push((r"C:\radioconda\Library\bin".to_string(), "Radioconda (Raíz C:)"));
        candidates.push((r"D:\radioconda\Library\bin".to_string(), "Radioconda (Raíz D:)"));
        candidates.push((r"C:\miniconda3\Library\bin".to_string(), "Conda (Raíz C:)"));
        candidates.push((r"C:\anaconda3\Library\bin".to_string(), "Anaconda (Raíz C:)"));

        // Inspeccionar el PATH global
        if let Ok(path_var) = std::env::var("PATH") {
            for entry in path_var.split(';') {
                let trimmed = entry.trim();
                if !trimmed.is_empty() {
                    let lower = trimmed.to_lowercase();
                    if lower.contains("pothos") {
                        candidates.push((trimmed.to_string(), "PothosSDR (Detectado en PATH)"));
                    } else if lower.contains("radioconda") {
                        candidates.push((trimmed.to_string(), "Radioconda (Detectado en PATH)"));
                    } else if lower.contains("conda") {
                        candidates.push((trimmed.to_string(), "Conda / Soapy (Detectado en PATH)"));
                    }
                }
            }
        }

        for (cand, flavor) in candidates {
            let mut p = std::path::PathBuf::from(&cand);
            if !p.join("SoapySDR.dll").exists() && !p.join("SoapySDRUtil.exe").exists() {
                if p.join("Library").join("bin").join("SoapySDR.dll").exists() {
                    p = p.join("Library").join("bin");
                } else if p.join("bin").join("SoapySDR.dll").exists() {
                    p = p.join("bin");
                }
            }

            if p.is_dir() && (p.join("SoapySDR.dll").exists() || p.join("SoapySDRUtil.exe").exists() || p.join("libSoapySDR.so").exists()) {
                let p_str = p.display().to_string();
                let norm = p_str.to_lowercase().replace('/', "\\");
                if seen.insert(norm) {
                    let is_active = if let Some(active) = current_active {
                        active.trim().eq_ignore_ascii_case(&p_str)
                    } else {
                        results.is_empty()
                    };

                    results.push(crate::ui::desktop_nav::SoapyInstallation {
                        name: flavor.to_string(),
                        path: p_str,
                        is_active,
                    });
                }
            }
        }
    }

    results
}

fn test_soapy_detection(path_input: &str) -> (String, bool) {
    let trimmed = path_input.trim();
    if trimmed.is_empty() {
        return ("Ingresa una ruta primero.".to_string(), false);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut p = std::path::PathBuf::from(trimmed);
        if !p.exists() || !p.is_dir() {
            return (format!("La ruta especificada no existe o no es una carpeta: {}", trimmed), false);
        }

        if !p.join("SoapySDR.dll").exists() && !p.join("SoapySDRUtil.exe").exists() {
            if p.join("Library").join("bin").join("SoapySDR.dll").exists() {
                p = p.join("Library").join("bin");
            } else if p.join("bin").join("SoapySDR.dll").exists() {
                p = p.join("bin");
            }
        }

        let has_dll = p.join("SoapySDR.dll").exists();
        let has_util = p.join("SoapySDRUtil.exe").exists() || p.join("SoapySDRUtil").exists();
        let has_so = p.join("libSoapySDR.so").exists();

        if has_dll || has_util || has_so {
            let resolved_str = p.display().to_string();
            if let Ok(current_path) = std::env::var("PATH") {
                if !current_path.split(';').any(|item| item.trim().eq_ignore_ascii_case(&resolved_str)) {
                    let new_path = format!("{};{}", resolved_str, current_path);
                    std::env::set_var("PATH", new_path);
                }
            }
            (format!("¡Éxito! SoapySDR detectado en: {} (Binarios encontrados y vinculados correctamente)", resolved_str), true)
        } else {
            (format!("Aviso: No se encontraron 'SoapySDR.dll' ni 'SoapySDRUtil.exe' en la carpeta indicada ({}). Verifica que sea la carpeta bin (ej. 'PothosSDR\\bin' o 'radioconda\\Library\\bin').", trimmed), false)
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        ("La detección de SoapySDR local no aplica en entorno WebAssembly.".to_string(), true)
    }
}
