/// Utilidades multiplataforma para apertura de archivos, carpetas y URLs
use std::path::Path;

/// Abre un archivo, imagen o enlace web en la aplicación predeterminada del sistema
pub fn open_path_or_url(target: &str) {
    if target.is_empty() {
        return;
    }

    #[cfg(target_arch = "wasm32")]
    {
        // En WebAssembly, solo abrir en nueva pestaña si es un enlace web HTTP o HTTPS legítimo.
        // Las rutas de archivo locales o relativas nunca deben navegarse en el navegador porque
        // el servidor LAN responde con la SPA (index.html), recargando la interfaz.
        if target.starts_with("http://") || target.starts_with("https://") {
            if let Some(window) = web_sys::window() {
                let _ = window.open_with_url_and_target(target, "_blank");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", target])
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(target)
                .spawn();
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(target)
                .spawn();
        }
    }
}

/// Abre una carpeta en el explorador de archivos nativo del sistema
pub fn open_folder(dir: &Path) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dir;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer")
                .arg(dir.as_os_str())
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg(dir.as_os_str())
                .spawn();
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(dir.as_os_str())
                .spawn();
        }
    }
}
