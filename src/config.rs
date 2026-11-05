use crate::theme::AppTheme;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub is_dark_mode: bool,
    pub default_output_dir: PathBuf,
    #[serde(default)]
    pub soapy_search_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let default_dir = dirs::document_dir()
            .map(|p| p.join("Signals_SDR"))
            .unwrap_or_else(|| PathBuf::from("Signals_SDR_Output"));

        #[cfg(target_arch = "wasm32")]
        let default_dir = PathBuf::from("Signals_SDR_Output");

        Self {
            is_dark_mode: true,
            default_output_dir: default_dir,
            soapy_search_path: None,
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
        #[cfg(not(target_arch = "wasm32"))]
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Asegura que la carpeta base y todas sus subcarpetas existan en el sistema de archivos
    pub fn ensure_directories_exist(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = std::fs::create_dir_all(&self.default_output_dir);
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
                let _ = std::fs::create_dir_all(self.default_output_dir.join(slug));
            }
        }
    }

    /// Carga la configuración desde el disco o usa los valores predeterminados
    pub fn load() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let config_path = dirs::config_dir()
                .map(|p| p.join("Signals_SDR").join("config.json"));

            if let Some(path) = config_path {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                        cfg.ensure_directories_exist();
                        cfg.apply_environment();
                        return cfg;
                    }
                }
            }
        }

        let default_cfg = Self::default();
        default_cfg.ensure_directories_exist();
        default_cfg.apply_environment();
        default_cfg
    }

    /// Auto-detecta la ruta de instalación de SoapySDR en ubicaciones conocidas de Windows
    pub fn find_system_soapy_path() -> Option<PathBuf> {
        #[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
        {
            let mut candidates = Vec::new();

            // PothosSDR vía variable de entorno oficial POTHOS_ROOT
            if let Ok(pothos_root) = std::env::var("POTHOS_ROOT") {
                candidates.push(PathBuf::from(pothos_root).join("bin"));
            }

            // Carpetas estándar de ProgramFiles para PothosSDR
            if let Ok(pf) = std::env::var("ProgramFiles") {
                candidates.push(PathBuf::from(&pf).join("PothosSDR").join("bin"));
            }
            if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
                candidates.push(PathBuf::from(&pf86).join("PothosSDR").join("bin"));
            }

            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                candidates.push(PathBuf::from(&userprofile).join("radioconda").join("Library").join("bin"));
                candidates.push(PathBuf::from(&userprofile).join("miniconda3").join("Library").join("bin"));
                candidates.push(PathBuf::from(&userprofile).join("anaconda3").join("Library").join("bin"));
                candidates.push(PathBuf::from(&userprofile).join("PothosSDR").join("bin"));
            }
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(parent) = exe_path.parent() {
                    candidates.push(parent.to_path_buf());
                }
            }

            // Rutas directas en C: y D: para PothosSDR y Radioconda
            candidates.push(PathBuf::from(r"C:\Program Files\PothosSDR\bin"));
            candidates.push(PathBuf::from(r"C:\Program Files (x86)\PothosSDR\bin"));
            candidates.push(PathBuf::from(r"C:\PothosSDR\bin"));
            candidates.push(PathBuf::from(r"D:\Program Files\PothosSDR\bin"));
            candidates.push(PathBuf::from(r"D:\PothosSDR\bin"));
            candidates.push(PathBuf::from(r"C:\Users\USUARIO\radioconda\Library\bin"));
            candidates.push(PathBuf::from(r"C:\radioconda\Library\bin"));
            candidates.push(PathBuf::from(r"C:\miniconda3\Library\bin"));
            candidates.push(PathBuf::from(r"C:\anaconda3\Library\bin"));

            for cand in candidates {
                if cand.exists() && (cand.join("SoapySDR.dll").exists() || cand.join("SoapySDRUtil.exe").exists()) {
                    return Some(cand);
                }
            }
        }
        None
    }

    /// Inyecta la ruta de búsqueda de SoapySDR a la variable de entorno PATH y al cargador de Windows
    pub fn apply_environment(&self) {
        #[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
        {
            let resolved_path = self.soapy_search_path.as_ref()
                .map(PathBuf::from)
                .or_else(Self::find_system_soapy_path);

            if let Some(path) = resolved_path {
                if path.is_dir() {
                    let path_str = path.display().to_string();
                    if let Ok(current_path) = std::env::var("PATH") {
                        // Limpiar cualquier otra ruta de SoapySDR y anteponer la seleccionada
                        let filtered_entries: Vec<&str> = current_path
                            .split(';')
                            .filter(|p| {
                                let l = p.trim().to_lowercase();
                                !l.is_empty() && !l.eq_ignore_ascii_case(&path_str)
                            })
                            .collect();
                        let new_path = format!("{};{}", path_str, filtered_entries.join(";"));
                        std::env::set_var("PATH", new_path);
                    }

                    // Registrar directorio en el cargador dinámico de Windows (kernel32!SetDllDirectoryW)
                    use std::os::windows::ffi::OsStrExt;
                    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                    extern "system" {
                        fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
                    }
                    unsafe {
                        SetDllDirectoryW(wide.as_ptr());
                    }

                    // Configurar SOAPY_SDR_ROOT para que Soapy cargue los módulos específicos de esta distribución
                    if let Some(parent) = path.parent() {
                        let root = if path.ends_with("bin") && parent.ends_with("Library") {
                            parent.parent().unwrap_or(parent)
                        } else {
                            parent
                        };
                        std::env::set_var("SOAPY_SDR_ROOT", root.display().to_string());
                    }
                }
            }
        }
    }

    /// Guarda la configuración actual en el disco
    pub fn save(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(mut path) = dirs::config_dir() {
            path = path.join("Signals_SDR");
            let _ = std::fs::create_dir_all(&path);
            let file_path = path.join("config.json");
            if let Ok(serialized) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(file_path, serialized);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_properties() {
        let cfg = AppConfig::default();
        assert!(cfg.default_output_dir.to_string_lossy().contains("Signals_SDR"));
        assert!(cfg.soapy_search_path.is_none());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_system_soapy_detection_or_fallback() {
        let _ = AppConfig::find_system_soapy_path();
        let cfg = AppConfig::default();
        cfg.apply_environment();
    }
}
