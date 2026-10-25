use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IqExportFormat {
    /// Archivo WAV de 2 canales I/Q (16-bit PCM) compatible con SDR#, SDR++, Audacity y Gqrx
    WavIq16,
    /// Archivo RAW binario con int16 escalado en formato Q11 ([-2048, 2047]), estándar para bladeRF
    RawSc16Q11,
    /// Archivo RAW binario con int16 a escala completa ([-32768, 32767])
    RawSc16Full,
    /// Archivo RAW binario con signed int8 entrelazado ([-128, 127]), formato nativo para hackrf_transfer
    RawInt8,
    /// Archivo RAW binario con float32 entrelazados [I0, Q0, ...] para GNU Radio o SoapySDR
    RawFloat32,
    /// Archivo WAV de audio mono (16-bit PCM) para señales con subportadora audible
    WavAudioMono,
}

impl IqExportFormat {
    pub fn name(&self) -> &'static str {
        match self {
            IqExportFormat::WavIq16 => "WAV I/Q Estéreo (16-bit PCM - SDR#, SDR++)",
            IqExportFormat::RawSc16Q11 => "RAW SC16_Q11 (int16 Little-Endian para bladeRF)",
            IqExportFormat::RawSc16Full => "RAW SC16 Estándar (int16 Little-Endian Full Scale)",
            IqExportFormat::RawInt8 => "RAW Int8 (signed 8-bit para hackrf_transfer)",
            IqExportFormat::RawFloat32 => "RAW Float32 (float32 Little-Endian)",
            IqExportFormat::WavAudioMono => "WAV Audio Mono (Subportadora audible)",
        }
    }

    pub fn label(&self) -> &'static str {
        self.name()
    }

    pub fn default_extension(&self) -> &'static str {
        match self {
            IqExportFormat::WavIq16 => "wav",
            IqExportFormat::RawSc16Q11 => "bin",
            IqExportFormat::RawSc16Full => "bin",
            IqExportFormat::RawInt8 => "raw",
            IqExportFormat::RawFloat32 => "iq",
            IqExportFormat::WavAudioMono => "wav",
        }
    }
}

/// Codifica muestras I/Q en memoria (útil para descarga directa en WebAssembly o guardado en disco)
pub fn export_iq_to_memory(
    samples: &[(f32, f32)],
    sample_rate: u32,
    format: IqExportFormat,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Cursor;

    match format {
        IqExportFormat::WavIq16 => {
            let spec = WavSpec {
                channels: 2,
                sample_rate,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };
            let mut buf = Vec::new();
            {
                let cursor = Cursor::new(&mut buf);
                let mut writer = WavWriter::new(cursor, spec)?;
                for &(i, q) in samples {
                    let i_s = (i * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    let q_s = (q * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    writer.write_sample(i_s)?;
                    writer.write_sample(q_s)?;
                }
                writer.finalize()?;
            }
            Ok(buf)
        }
        IqExportFormat::RawSc16Q11 => {
            let mut buf = Vec::with_capacity(samples.len() * 4);
            for &(i, q) in samples {
                let i_val = (i * 2047.0).clamp(-2048.0, 2047.0) as i16;
                let q_val = (q * 2047.0).clamp(-2048.0, 2047.0) as i16;
                buf.extend_from_slice(&i_val.to_le_bytes());
                buf.extend_from_slice(&q_val.to_le_bytes());
            }
            Ok(buf)
        }
        IqExportFormat::RawSc16Full => {
            let mut buf = Vec::with_capacity(samples.len() * 4);
            for &(i, q) in samples {
                let i_val = (i * 32767.0).clamp(-32768.0, 32767.0) as i16;
                let q_val = (q * 32767.0).clamp(-32768.0, 32767.0) as i16;
                buf.extend_from_slice(&i_val.to_le_bytes());
                buf.extend_from_slice(&q_val.to_le_bytes());
            }
            Ok(buf)
        }
        IqExportFormat::RawInt8 => {
            let mut buf = Vec::with_capacity(samples.len() * 2);
            for &(i, q) in samples {
                let i_val = (i * 127.0).clamp(-128.0, 127.0) as i8;
                let q_val = (q * 127.0).clamp(-128.0, 127.0) as i8;
                buf.push(i_val as u8);
                buf.push(q_val as u8);
            }
            Ok(buf)
        }
        IqExportFormat::RawFloat32 => {
            let mut buf = Vec::with_capacity(samples.len() * 8);
            for &(i, q) in samples {
                buf.extend_from_slice(&i.to_le_bytes());
                buf.extend_from_slice(&q.to_le_bytes());
            }
            Ok(buf)
        }
        IqExportFormat::WavAudioMono => {
            let spec = WavSpec {
                channels: 1,
                sample_rate,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };
            let mut buf = Vec::new();
            {
                let cursor = Cursor::new(&mut buf);
                let mut writer = WavWriter::new(cursor, spec)?;
                for &(i, _) in samples {
                    let i_s = (i * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    writer.write_sample(i_s)?;
                }
                writer.finalize()?;
            }
            Ok(buf)
        }
    }
}

/// Exporta muestras I/Q al formato y ruta seleccionados
pub fn export_iq_file(
    path: &Path,
    samples: &[(f32, f32)],
    sample_rate: u32,
    format: IqExportFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = export_iq_to_memory(samples, sample_rate, format)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Lee muestras I/Q directamente desde un buffer en memoria (compatible con Web y Desktop)
pub fn read_iq_from_memory(
    bytes: &[u8],
    format: IqExportFormat,
) -> Result<(Vec<(f32, f32)>, u32), Box<dyn std::error::Error>> {
    use std::io::Cursor;

    match format {
        IqExportFormat::WavIq16 => {
            let cursor = Cursor::new(bytes);
            let mut reader = hound::WavReader::new(cursor)?;
            let sample_rate = reader.spec().sample_rate;
            let mut samples = Vec::new();
            let mut i_val = 0.0f32;
            let mut is_i = true;

            for sample in reader.samples::<i16>() {
                let s = (sample? as f32) / 32768.0;
                if is_i {
                    i_val = s;
                    is_i = false;
                } else {
                    samples.push((i_val, s));
                    is_i = true;
                }
            }
            Ok((samples, sample_rate))
        }
        IqExportFormat::RawSc16Q11 => {
            let mut samples = Vec::with_capacity(bytes.len() / 4);
            for chunk in bytes.chunks_exact(4) {
                let i = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 2048.0;
                let q = i16::from_le_bytes([chunk[2], chunk[3]]) as f32 / 2048.0;
                samples.push((i, q));
            }
            Ok((samples, 288_000))
        }
        IqExportFormat::RawSc16Full => {
            let mut samples = Vec::with_capacity(bytes.len() / 4);
            for chunk in bytes.chunks_exact(4) {
                let i = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0;
                let q = i16::from_le_bytes([chunk[2], chunk[3]]) as f32 / 32768.0;
                samples.push((i, q));
            }
            Ok((samples, 288_000))
        }
        IqExportFormat::RawInt8 => {
            let mut samples = Vec::with_capacity(bytes.len() / 2);
            for chunk in bytes.chunks_exact(2) {
                let i = (chunk[0] as i8 as f32) / 128.0;
                let q = (chunk[1] as i8 as f32) / 128.0;
                samples.push((i, q));
            }
            Ok((samples, 2_000_000))
        }
        IqExportFormat::RawFloat32 => {
            let mut samples = Vec::with_capacity(bytes.len() / 8);
            for chunk in bytes.chunks_exact(8) {
                let i = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let q = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                samples.push((i, q));
            }
            Ok((samples, 1_000_000))
        }
        IqExportFormat::WavAudioMono => {
            let cursor = Cursor::new(bytes);
            let mut reader = hound::WavReader::new(cursor)?;
            let sample_rate = reader.spec().sample_rate;
            let mut samples = Vec::new();
            for sample in reader.samples::<i16>() {
                let s = (sample? as f32) / 32768.0;
                samples.push((s, 0.0));
            }
            Ok((samples, sample_rate))
        }
    }
}

/// Lee un archivo I/Q cargado (para el pipeline Generic IQ)
pub fn read_iq_file(
    path: &Path,
    format: IqExportFormat,
) -> Result<(Vec<(f32, f32)>, u32), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    read_iq_from_memory(&buffer, format)
}
