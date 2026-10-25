//! Utilidades de integración web (WebAssembly) para descarga de archivos y peticiones HTTP.

#[cfg(target_arch = "wasm32")]
pub fn trigger_browser_download(filename: &str, data: &[u8], mime_type: &str) -> Result<(), String> {
    use web_sys::wasm_bindgen::JsCast;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let window = web_sys::window().ok_or_else(|| "No se encontró el objeto global window".to_string())?;
    let document = window
        .document()
        .ok_or_else(|| "No se encontró el documento HTML".to_string())?;

    let array = js_sys::Uint8Array::new_with_length(data.len() as u32);
    array.copy_from(data);

    let parts = js_sys::Array::new();
    parts.push(&array.buffer());

    let bag = BlobPropertyBag::new();
    bag.set_type(mime_type);

    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &bag)
        .map_err(|e| format!("Error creando Blob JS: {:?}", e))?;

    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Error creando ObjectURL: {:?}", e))?;

    let element = document
        .create_element("a")
        .map_err(|e| format!("Error creando elemento anchor: {:?}", e))?;

    let anchor: HtmlAnchorElement = element
        .dyn_into()
        .map_err(|e| format!("Error convirtiendo a HtmlAnchorElement: {:?}", e))?;

    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.style().set_property("display", "none").ok();

    if let Some(body) = document.body() {
        body.append_child(&anchor).ok();
        anchor.click();
        body.remove_child(&anchor).ok();
    } else {
        anchor.click();
    }

    Url::revoke_object_url(&url).ok();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn trigger_browser_download(_filename: &str, _data: &[u8], _mime_type: &str) -> Result<(), String> {
    Err("La descarga directa por navegador solo aplica en WebAssembly".to_string())
}

/// Descarga bytes desde una URL de forma asíncrona (compatible con Web y Desktop)
#[cfg(target_arch = "wasm32")]
pub fn fetch_url_bytes<F, E>(url: String, on_success: F, on_error: E)
where
    F: FnOnce(Vec<u8>) + 'static,
    E: FnOnce(String) + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        use web_sys::wasm_bindgen::JsCast;
        use web_sys::{Request, RequestInit, Response};

        let opts = RequestInit::new();
        opts.set_method("GET");

        let request = match Request::new_with_str_and_init(&url, &opts) {
            Ok(r) => r,
            Err(e) => {
                on_error(format!("Error creando Request: {:?}", e));
                return;
            }
        };

        let window = match web_sys::window() {
            Some(w) => w,
            None => {
                on_error("Ventana global no disponible".to_string());
                return;
            }
        };

        let resp_value = match wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await {
            Ok(v) => v,
            Err(e) => {
                on_error(format!("Fallo en petición fetch HTTP: {:?}", e));
                return;
            }
        };

        let resp: Response = match resp_value.dyn_into() {
            Ok(r) => r,
            Err(_) => {
                on_error("Respuesta no es de tipo Response".to_string());
                return;
            }
        };

        if !resp.ok() {
            on_error(format!("HTTP {} {}", resp.status(), resp.status_text()));
            return;
        }

        let array_buffer = match resp.array_buffer() {
            Ok(ab) => ab,
            Err(e) => {
                on_error(format!("Error leyendo array buffer: {:?}", e));
                return;
            }
        };

        let buffer_value = match wasm_bindgen_futures::JsFuture::from(array_buffer).await {
            Ok(bv) => bv,
            Err(e) => {
                on_error(format!("Fallo en lectura de bytes: {:?}", e));
                return;
            }
        };

        let type_array = js_sys::Uint8Array::new(&buffer_value);
        let mut vec = vec![0u8; type_array.length() as usize];
        type_array.copy_to(&mut vec);
        on_success(vec);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_url_bytes<F, E>(url: String, on_success: F, on_error: E)
where
    F: FnOnce(Vec<u8>) + Send + 'static,
    E: FnOnce(String) + Send + 'static,
{
    std::thread::spawn(move || {
        match ureq::get(&url).call() {
            Ok(mut response) => {
                let mut bytes = Vec::new();
                use std::io::Read;
                if let Ok(_) = response.body_mut().as_reader().read_to_end(&mut bytes) {
                    on_success(bytes);
                } else {
                    on_error("Fallo al leer cuerpo de respuesta HTTP".to_string());
                }
            }
            Err(e) => {
                on_error(format!("Error en descarga HTTP: {}", e));
            }
        }
    });
}

/// Crea una URL de objeto Blob para reproducción de audio en el navegador.
/// Detecta automáticamente el tipo MIME (WAV o MP3) según el contenido. Solo disponible en WASM.
#[cfg(target_arch = "wasm32")]
pub fn create_audio_blob_url(audio_bytes: &[u8]) -> Result<String, String> {
    use web_sys::{Blob, BlobPropertyBag, Url};

    let mime = if audio_bytes.starts_with(b"ID3")
        || (audio_bytes.len() >= 2 && audio_bytes[0] == 0xFF && (audio_bytes[1] & 0xE0) == 0xE0)
    {
        "audio/mpeg"
    } else {
        "audio/wav"
    };

    let array = js_sys::Uint8Array::new_with_length(audio_bytes.len() as u32);
    array.copy_from(audio_bytes);

    let parts = js_sys::Array::new();
    parts.push(&array.buffer());

    let bag = BlobPropertyBag::new();
    bag.set_type(mime);

    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &bag)
        .map_err(|e| format!("Error creando Blob de audio: {:?}", e))?;

    Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Error creando ObjectURL de audio: {:?}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_audio_blob_url(_audio_bytes: &[u8]) -> Result<String, String> {
    Err("create_audio_blob_url solo disponible en WebAssembly".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn web_audio_play(url: &str, start_time: f32) {
    let script = format!(
        r#"if (window._signals_sdr_audio) {{ window._signals_sdr_audio.play('{}', {}); }}"#,
        url, start_time
    );
    let _ = js_sys::eval(&script);
}

#[cfg(target_arch = "wasm32")]
pub fn web_audio_pause() {
    let _ = js_sys::eval(r#"if (window._signals_sdr_audio) { window._signals_sdr_audio.pause(); }"#);
}

#[cfg(target_arch = "wasm32")]
pub fn web_audio_stop() {
    let _ = js_sys::eval(r#"if (window._signals_sdr_audio) { window._signals_sdr_audio.stop(); }"#);
}

#[cfg(target_arch = "wasm32")]
pub fn web_audio_seek(time: f32) {
    let script = format!(
        r#"if (window._signals_sdr_audio) {{ window._signals_sdr_audio.seek({}); }}"#,
        time
    );
    let _ = js_sys::eval(&script);
}

#[cfg(target_arch = "wasm32")]
pub fn web_audio_status() -> Option<(bool, f32, f32)> {
    if let Ok(val) = js_sys::eval(r#"window._signals_sdr_audio ? window._signals_sdr_audio.status() : null"#) {
        if let Some(s) = val.as_string() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&s) {
                let playing = parsed.get("playing").and_then(|v| v.as_bool()).unwrap_or(false);
                let current = parsed.get("currentTime").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let duration = parsed.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                return Some((playing, current, duration));
            }
        }
    }
    None
}
