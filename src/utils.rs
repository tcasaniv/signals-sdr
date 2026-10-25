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

/// Decodifica un archivo de audio (WAV, MP3, etc.) directamente desde un buffer de bytes en memoria.
/// Compatible tanto con WebAssembly como con Desktop.
pub fn load_audio_from_memory(
    bytes: &[u8],
    extension_hint: Option<&str>,
) -> Result<fm_modulator::AudioData, Box<dyn std::error::Error>> {
    use std::io::Cursor;
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::errors::Error as SymphoniaError;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let cursor = Box::new(Cursor::new(bytes.to_vec()));
    let mss = MediaSourceStream::new(cursor, Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = extension_hint {
        hint.with_extension(ext);
    }

    let meta_opts = MetadataOptions::default();
    let fmt_opts = FormatOptions::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No se encontró ninguna pista de audio válida en el archivo")?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or("No se pudo determinar la tasa de muestreo del audio")?;

    let dec_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

    let mut mono_samples = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut original_channels = 1;

    loop {
        let packet = match format.next_packet() {
            Ok(pkt) => pkt,
            Err(SymphoniaError::IoError(ref err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::IoError(_)) => break,
            Err(err) => return Err(Box::new(err)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                original_channels = spec.channels.count();

                if sample_buf.is_none() {
                    sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
                }

                if let Some(buf) = sample_buf.as_mut() {
                    buf.copy_interleaved_ref(decoded);
                    let n_ch = spec.channels.count();
                    for frame in buf.samples().chunks_exact(n_ch) {
                        let mono: f32 = frame.iter().sum::<f32>() / n_ch as f32;
                        mono_samples.push(mono);
                    }
                }
            }
            Err(SymphoniaError::DecodeError(_)) => {
                continue;
            }
            Err(SymphoniaError::IoError(_)) => break,
            Err(err) => return Err(Box::new(err)),
        }
    }

    if mono_samples.is_empty() {
        return Err("El archivo de audio no contiene muestras legibles".into());
    }

    Ok(fm_modulator::AudioData {
        samples: mono_samples,
        sample_rate,
        original_channels,
    })
}

/// Remuestrea una señal I/Q compleja mediante interpolación lineal de fase continua.
/// Realiza una única reserva exacta de memoria y es compatible con Web y Desktop.
pub fn resample_iq(samples: &[(f32, f32)], in_rate: u32, out_rate: u32) -> Vec<(f32, f32)> {
    if in_rate == out_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let duration = samples.len() as f64 / in_rate as f64;
    let n_out = (duration * out_rate as f64).round() as usize;
    if n_out == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n_out);
    let ratio = (samples.len() - 1) as f64 / ((n_out - 1).max(1) as f64);
    for j in 0..n_out {
        let src_pos = j as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        if idx + 1 < samples.len() {
            let i = samples[idx].0 * (1.0 - frac) + samples[idx + 1].0 * frac;
            let q = samples[idx].1 * (1.0 - frac) + samples[idx + 1].1 * frac;
            out.push((i, q));
        } else {
            out.push(samples[idx]);
        }
    }
    out
}
