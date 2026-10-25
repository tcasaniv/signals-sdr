//! Abstracción transparente de diálogo de archivos para Escritorio (rfd nativo) y Web (rfd async / sandbox).

#[cfg(not(target_arch = "wasm32"))]
pub use rfd::FileDialog;

#[cfg(target_arch = "wasm32")]
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
#[derive(Default, Debug, Clone)]
pub struct FileDialog;

#[cfg(target_arch = "wasm32")]
impl FileDialog {
    pub fn new() -> Self {
        Self
    }
    pub fn add_filter<S: AsRef<str>>(self, _name: &str, _extensions: &[S]) -> Self {
        self
    }
    pub fn set_file_name(self, _file_name: &str) -> Self {
        self
    }
    pub fn pick_file(self) -> Option<PathBuf> {
        None
    }
    pub fn pick_folder(self) -> Option<PathBuf> {
        None
    }
    pub fn save_file(self) -> Option<PathBuf> {
        None
    }
}

/// Selecciona un archivo de forma asíncrona tanto en Desktop como en WebAssembly (rfd).
/// En Web abre el selector nativo del navegador; en Desktop abre el explorador nativo.
#[cfg(not(target_arch = "wasm32"))]
pub fn pick_file_async<F>(filter_name: &'static str, extensions: &'static [&'static str], on_loaded: F)
where
    F: FnOnce(String, Vec<u8>) + Send + 'static,
{
    std::thread::spawn(move || {
        let mut dialog = rfd::FileDialog::new();
        dialog = dialog.add_filter(filter_name, extensions);
        if let Some(path) = dialog.pick_file() {
            let filename = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            if let Ok(bytes) = std::fs::read(&path) {
                on_loaded(filename, bytes);
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn pick_file_async<F>(filter_name: &'static str, extensions: &'static [&'static str], on_loaded: F)
where
    F: FnOnce(String, Vec<u8>) + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        let mut dialog = rfd::AsyncFileDialog::new();
        dialog = dialog.add_filter(filter_name, extensions);
        if let Some(handle) = dialog.pick_file().await {
            let filename = handle.file_name();
            let bytes = handle.read().await;
            on_loaded(filename, bytes);
        }
    });
}
