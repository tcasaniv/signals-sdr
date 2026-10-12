use crate::theme::AppTheme;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub is_dark_mode: bool,
    pub default_output_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_dir = dirs::document_dir()
            .map(|p| p.join("Signals_SDR"))
            .unwrap_or_else(|| PathBuf::from("Signals_SDR_Output"));

        Self {
            is_dark_mode: true,
            default_output_dir: default_dir,
        }
    }
}

impl AppConfig {
    pub fn current_theme(&self) -> AppTheme {
        if self.is_dark_mode {
            AppTheme::Dark
        } else {
            AppTheme::Light
        }
    }

    /// Obtiene y crea (si no existe) la subcarpeta organizada correspondiente a un pipeline
    pub fn get_pipeline_dir(&self, pipeline_slug: &str) -> PathBuf {
        let dir = self.default_output_dir.join(pipeline_slug);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Asegura que la carpeta base y todas sus subcarpetas existan en el sistema de archivos
    pub fn ensure_directories_exist(&self) {
        let _ = fs::create_dir_all(&self.default_output_dir);
        let pipelines = [
            "noaa_apt",
            "meteor_lrpt",
            "iss_sstv",
            "generic_sstv",
            "generic_fm",
            "generic_am",
            "generic_iq",
        ];
        for slug in pipelines {
            let _ = fs::create_dir_all(self.default_output_dir.join(slug));
        }
    }

    /// Carga la configuración desde el disco o usa los valores predeterminados
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .map(|p| p.join("Signals_SDR").join("config.json"));

        if let Some(path) = config_path {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                    cfg.ensure_directories_exist();
                    return cfg;
                }
            }
        }

        let default_cfg = Self::default();
        default_cfg.ensure_directories_exist();
        default_cfg
    }

    /// Guarda la configuración actual en el disco
    pub fn save(&self) {
        if let Some(mut path) = dirs::config_dir() {
            path = path.join("Signals_SDR");
            let _ = fs::create_dir_all(&path);
            let file_path = path.join("config.json");
            if let Ok(serialized) = serde_json::to_string_pretty(self) {
                let _ = fs::write(file_path, serialized);
            }
        }
    }
}
