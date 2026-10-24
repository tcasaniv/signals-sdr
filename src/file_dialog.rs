//! Abstracción transparente de diálogo de archivos para Escritorio (rfd nativo) y Web (stub para sandbox).

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
