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
pub fn pick_file_async<F>(_filter_name: &'static str, extensions: &'static [&'static str], on_loaded: F)
where
    F: FnOnce(String, Vec<u8>) + 'static,
{
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};

    let accept = extensions
        .iter()
        .map(|ext| {
            if ext.starts_with('.') {
                ext.to_string()
            } else {
                format!(".{}", ext)
            }
        })
        .collect::<Vec<_>>()
        .join(",");

    let mut callback_slot = Some(on_loaded);
    let closure = Closure::once(move |name: String, u8_array: js_sys::Uint8Array| {
        if let Some(cb) = callback_slot.take() {
            let bytes = u8_array.to_vec();
            cb(name, bytes);
        }
    });

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    // Inyección de seguridad por si no estuviese en el HTML
    let _ = js_sys::eval(r#"
        if (!window._signals_sdr_pick_file) {
            window._signals_sdr_pick_file = function(accept, callback) {
                const input = document.createElement('input');
                input.type = 'file';
                if (accept) input.accept = accept;
                input.style.display = 'none';
                input.onchange = function() {
                    if (!input.files || input.files.length === 0) return;
                    const file = input.files[0];
                    const reader = new FileReader();
                    reader.onload = function() {
                        const u8 = new Uint8Array(reader.result);
                        callback(file.name, u8);
                    };
                    reader.readAsArrayBuffer(file);
                };
                document.body.appendChild(input);
                input.click();
                setTimeout(() => { if (input.parentNode) input.parentNode.removeChild(input); }, 60000);
            };
        }
    "#);

    if let Ok(func) = js_sys::Reflect::get(&window, &JsValue::from_str("_signals_sdr_pick_file")) {
        if let Some(f) = func.dyn_ref::<js_sys::Function>() {
            let _ = f.call2(&window, &JsValue::from_str(&accept), closure.as_ref().unchecked_ref());
            closure.forget();
        }
    }
}
