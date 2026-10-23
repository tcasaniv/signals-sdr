use num_complex::Complex32;
use seify::{DynDevice, TxStreamer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SdrBackend {
    SeifyNative,
    SoapySdr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SdrHardwareType {
    Dummy,
    HackRfSoapy,
    HackRfSeify,
    BladeRfSoapy,
    UsrpSoapy,
    PlutoSdrSoapy,
    LimeSdrSoapy,
    SoapyAuto,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdrDeviceOption {
    pub display_name: String,
    pub backend: SdrBackend,
    pub hw_type: SdrHardwareType,
    pub default_args: String,
    pub description: String,
}

impl SdrDeviceOption {
    pub fn all_supported() -> Vec<SdrDeviceOption> {
        vec![
            // --- Seify Nativo (Por Defecto - Sin Dependencias Externas) ---
            SdrDeviceOption {
                display_name: "HackRF One (Transmisión Real)".to_string(),
                backend: SdrBackend::SeifyNative,
                hw_type: SdrHardwareType::HackRfSeify,
                default_args: "driver=hackrf".to_string(),
                description: "Transceptor semidúplex con driver Rust puro nusb. No requiere software externo (mínimo 2 MSps).".to_string(),
            },
            SdrDeviceOption {
                display_name: "bladeRF (Transmisión Real)".to_string(),
                backend: SdrBackend::SeifyNative,
                hw_type: SdrHardwareType::BladeRfSoapy,
                default_args: "driver=bladerf".to_string(),
                description: "Transceptor full dúplex USB 3.0 con driver nativo Rust.".to_string(),
            },
            SdrDeviceOption {
                display_name: "Simulador Dummy (Sin hardware)".to_string(),
                backend: SdrBackend::SeifyNative,
                hw_type: SdrHardwareType::Dummy,
                default_args: "driver=dummy".to_string(),
                description: "Simula transmisión en memoria para pruebas de laboratorio sin emitir RF.".to_string(),
            },
            SdrDeviceOption {
                display_name: "Driver Personalizado Seify".to_string(),
                backend: SdrBackend::SeifyNative,
                hw_type: SdrHardwareType::Custom,
                default_args: "driver=hackrf".to_string(),
                description: "Ingresa argumentos directos para el backend Seify Nativo (ej: driver=hackrf).".to_string(),
            },

            // --- SoapySDR (Avanzado - Módulos C++ / Radioconda) ---
            SdrDeviceOption {
                display_name: "Detección Automática de Radio (SoapySDR)".to_string(),
                backend: SdrBackend::SoapySdr,
                hw_type: SdrHardwareType::SoapyAuto,
                default_args: "driver=soapy".to_string(),
                description: "Busca transceptores de radio reales conectados descartando tarjetas de audio.".to_string(),
            },
            SdrDeviceOption {
                display_name: "HackRF One (SoapySDR)".to_string(),
                backend: SdrBackend::SoapySdr,
                hw_type: SdrHardwareType::HackRfSoapy,
                default_args: "driver=soapy,soapy_driver=hackrf".to_string(),
                description: "Transceptor semidúplex 1 MHz - 6 GHz vía SoapySDR (estable en Windows, mínimo 1 MSps).".to_string(),
            },
            SdrDeviceOption {
                display_name: "bladeRF (SoapySDR)".to_string(),
                backend: SdrBackend::SoapySdr,
                hw_type: SdrHardwareType::BladeRfSoapy,
                default_args: "driver=soapy,soapy_driver=bladerf".to_string(),
                description: "Transceptor full dúplex USB 3.0 vía SoapySDR.".to_string(),
            },
            SdrDeviceOption {
                display_name: "USRP / UHD (Ettus Research / Red)".to_string(),
                backend: SdrBackend::SoapySdr,
                hw_type: SdrHardwareType::UsrpSoapy,
                default_args: "driver=soapy,soapy_driver=uhd".to_string(),
                description: "Dispositivos USRP B200, B210, N210, X310 vía SoapyUHD (admite IP de red).".to_string(),
            },
            SdrDeviceOption {
                display_name: "PlutoSDR / ADALM-Pluto (SoapySDR)".to_string(),
                backend: SdrBackend::SoapySdr,
                hw_type: SdrHardwareType::PlutoSdrSoapy,
                default_args: "driver=soapy,soapy_driver=plutosdr".to_string(),
                description: "Transceptor de Analog Devices ADALM-PLUTO vía USB o URI IP de red.".to_string(),
            },
            SdrDeviceOption {
                display_name: "LimeSDR (SoapySDR)".to_string(),
                backend: SdrBackend::SoapySdr,
                hw_type: SdrHardwareType::LimeSdrSoapy,
                default_args: "driver=soapy,soapy_driver=lime".to_string(),
                description: "Transceptor MIMO LMS7002M (LimeSDR USB / Mini).".to_string(),
            },
            SdrDeviceOption {
                display_name: "Driver Personalizado Soapy".to_string(),
                backend: SdrBackend::SoapySdr,
                hw_type: SdrHardwareType::Custom,
                default_args: "driver=soapy,soapy_driver=hackrf".to_string(),
                description: "Ingresa argumentos directos para SoapySDR (ej: driver=soapy,soapy_driver=hackrf).".to_string(),
            },
        ]
    }

    pub fn for_backend(backend: SdrBackend) -> Vec<SdrDeviceOption> {
        Self::all_supported()
            .into_iter()
            .filter(|d| d.backend == backend)
            .collect()
    }
}

/// Información completa de un transceptor SDR detectado físicamente
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedDeviceInfo {
    pub label: String,
    pub backend: SdrBackend,
    pub hw_type: SdrHardwareType,
    pub driver: String,
    pub soapy_driver: Option<String>,
    pub serial: Option<String>,
    pub full_args: String,
}

/// Detecta los transceptores de radio reales conectados por USB o red excluyendo tarjetas de audio
pub fn detect_detailed_radios() -> Vec<DetectedDeviceInfo> {
    let mut radios = Vec::new();
    if let Ok(devs) = seify::enumerate() {
        for d in devs {
            let driver_str = d.get::<String>("driver").unwrap_or_default();
            let soapy_driver = d.get::<String>("soapy_driver").ok();
            let soapy_driver_str = soapy_driver.as_deref().unwrap_or("");

            // Descartar audio y dispositivos virtuales dummy
            if driver_str == "audio" || soapy_driver_str == "audio" || driver_str == "dummy" {
                continue;
            }

            let serial = d.get::<String>("serial").ok().or_else(|| d.get::<String>("part_id").ok());
            let label = d
                .get::<String>("label")
                .or_else(|_| d.get::<String>("device"))
                .or_else(|_| d.get::<String>("product"))
                .unwrap_or_else(|_| {
                    if !soapy_driver_str.is_empty() {
                        format!("Soapy: {}", soapy_driver_str)
                    } else {
                        driver_str.clone()
                    }
                });

            let is_soapy = driver_str == "soapy" || !soapy_driver_str.is_empty();
            let backend = if is_soapy {
                SdrBackend::SoapySdr
            } else {
                SdrBackend::SeifyNative
            };

            let hw_type = if driver_str.contains("hackrf") || soapy_driver_str.contains("hackrf") {
                if is_soapy { SdrHardwareType::HackRfSoapy } else { SdrHardwareType::HackRfSeify }
            } else if driver_str.contains("bladerf") || soapy_driver_str.contains("bladerf") {
                SdrHardwareType::BladeRfSoapy
            } else if driver_str.contains("uhd") || soapy_driver_str.contains("uhd") {
                SdrHardwareType::UsrpSoapy
            } else if driver_str.contains("pluto") || soapy_driver_str.contains("pluto") {
                SdrHardwareType::PlutoSdrSoapy
            } else if driver_str.contains("lime") || soapy_driver_str.contains("lime") {
                SdrHardwareType::LimeSdrSoapy
            } else {
                SdrHardwareType::Custom
            };

            let full_args = if is_soapy {
                if let Some(ref s) = serial {
                    format!("driver=soapy,soapy_driver={},serial={}", soapy_driver_str, s)
                } else {
                    format!("driver=soapy,soapy_driver={}", soapy_driver_str)
                }
            } else {
                if let Some(ref s) = serial {
                    format!("driver={},serial={}", driver_str, s)
                } else {
                    format!("driver={}", driver_str)
                }
            };

            radios.push(DetectedDeviceInfo {
                label,
                backend,
                hw_type,
                driver: driver_str,
                soapy_driver,
                serial,
                full_args,
            });
        }
    }
    radios
}

/// Mantiene compatibilidad con detect_connected_radios devolviendo las etiquetas
pub fn detect_connected_radios() -> Vec<String> {
    detect_detailed_radios().into_iter().map(|r| r.label).collect()
}

#[derive(Debug, Clone)]
pub struct TxConfig {
    pub backend: SdrBackend,
    pub hw_type: SdrHardwareType,
    pub device_args: String,
    pub target_serial: String,
    pub frequency_hz: f64,
    pub sample_rate_hz: u32,
    pub repeat: bool,

    // HackRF específicos
    pub hackrf_vga_gain: f64, // 0.0 ..= 47.0 dB
    pub hackrf_rf_amp: bool,   // +14 dB PA

    // USRP específicos
    pub usrp_ip: String,       // ej: "192.168.10.2" o vacio
    pub usrp_subdev: String,   // ej: "A:0"
    pub usrp_gain: f64,        // 0.0 ..= 31.5 dB
    pub usrp_antenna: String,  // "TX/RX", "TRX"

    // PlutoSDR específicos
    pub pluto_uri: String,     // ej: "ip:192.168.2.1" o "usb:"
    pub pluto_attenuation: f64,// -89.75 ..= 0.0 dB
    pub pluto_antenna: String, // "A"

    // bladeRF específicos
    pub bladerf_tx_gain: f64,  // 0.0 ..= 66.0 dB

    // LimeSDR específicos
    pub lime_gain: f64,        // 0.0 ..= 60.0 dB
    pub lime_antenna: String,  // "BAND1", "BAND2"

    // Argumentos directos manuales
    pub custom_args: String,
}

impl Default for TxConfig {
    fn default() -> Self {
        Self {
            backend: SdrBackend::SeifyNative,
            hw_type: SdrHardwareType::HackRfSeify,
            device_args: "driver=hackrf".to_string(),
            target_serial: String::new(),
            frequency_hz: 915_000_000.0,
            sample_rate_hz: 2_000_000,
            repeat: false,

            hackrf_vga_gain: 16.0,
            hackrf_rf_amp: false,

            usrp_ip: String::new(),
            usrp_subdev: "A:0".to_string(),
            usrp_gain: 20.0,
            usrp_antenna: "TX/RX".to_string(),

            pluto_uri: String::new(),
            pluto_attenuation: -10.0,
            pluto_antenna: "A".to_string(),

            bladerf_tx_gain: 30.0,

            lime_gain: 30.0,
            lime_antenna: "BAND1".to_string(),

            custom_args: String::new(),
        }
    }
}

impl TxConfig {
    /// Resuelve los argumentos de conexión reales para Seify según el SDR seleccionado
    pub fn resolve_device_args(&self) -> Result<String, String> {
        let mut base_args = match self.hw_type {
            SdrHardwareType::Dummy => Ok("driver=dummy".to_string()),
            SdrHardwareType::HackRfSeify => Ok("driver=hackrf".to_string()),
            SdrHardwareType::HackRfSoapy => Ok("driver=soapy,soapy_driver=hackrf".to_string()),
            SdrHardwareType::BladeRfSoapy => {
                if self.backend == SdrBackend::SeifyNative {
                    Ok("driver=bladerf".to_string())
                } else {
                    Ok("driver=soapy,soapy_driver=bladerf".to_string())
                }
            }
            SdrHardwareType::UsrpSoapy => {
                let ip = self.usrp_ip.trim();
                if !ip.is_empty() {
                    Ok(format!("driver=soapy,soapy_driver=uhd,addr={}", ip))
                } else {
                    Ok("driver=soapy,soapy_driver=uhd".to_string())
                }
            }
            SdrHardwareType::PlutoSdrSoapy => {
                let uri = self.pluto_uri.trim();
                if !uri.is_empty() {
                    Ok(format!("driver=soapy,soapy_driver=plutosdr,uri={}", uri))
                } else {
                    Ok("driver=soapy,soapy_driver=plutosdr".to_string())
                }
            }
            SdrHardwareType::LimeSdrSoapy => Ok("driver=soapy,soapy_driver=lime".to_string()),
            SdrHardwareType::Custom => {
                let args = self.custom_args.trim();
                if args.is_empty() {
                    Err("Los argumentos personalizados no pueden estar vacíos. Ejemplo: driver=hackrf o driver=soapy,soapy_driver=hackrf".to_string())
                } else {
                    Ok(args.to_string())
                }
            }
            SdrHardwareType::SoapyAuto => {
                match seify::enumerate() {
                    Ok(devs) => {
                        for d in devs {
                            let driver_str = d.get::<String>("driver").unwrap_or_default();
                            let soapy_driver = d.get::<String>("soapy_driver").unwrap_or_default();
                            if driver_str == "audio" || soapy_driver == "audio" || driver_str == "dummy" {
                                continue;
                            }
                            if !soapy_driver.is_empty() {
                                return Ok(format!("driver=soapy,soapy_driver={}", soapy_driver));
                            } else if !driver_str.is_empty() {
                                return Ok(format!("driver={}", driver_str));
                            }
                        }
                        Err("SoapySDR no detectó ningún transceptor de radio conectado (se ignoraron las tarjetas de sonido). Verifica el cable USB o selecciona un driver específico.".to_string())
                    }
                    Err(e) => Err(format!("Error al consultar dispositivos SoapySDR: {}", e)),
                }
            }
        }?;

        // Añadir serial específico si está configurado y no figura en la cadena base
        let serial = self.target_serial.trim();
        if !serial.is_empty() && !base_args.contains("serial=") && base_args != "driver=dummy" {
            base_args.push_str(&format!(",serial={}", serial));
        }

        Ok(base_args)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransmissionStatus {
    Idle,
    Transmitting {
        samples_sent: usize,
        total_samples: usize,
        elapsed_sec: f32,
    },
    Finished {
        total_samples: usize,
        elapsed_sec: f32,
    },
    Error(String),
}

pub struct SdrTransmissionController {
    stop_signal: Arc<AtomicBool>,
    status: Arc<Mutex<TransmissionStatus>>,
    handle: Option<JoinHandle<()>>,
}

impl SdrTransmissionController {
    pub fn new() -> Self {
        Self {
            stop_signal: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(TransmissionStatus::Idle)),
            handle: None,
        }
    }

    pub fn probe_devices() -> Vec<SdrDeviceOption> {
        SdrDeviceOption::all_supported()
    }

    pub fn status(&self) -> TransmissionStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn is_transmitting(&self) -> bool {
        matches!(self.status(), TransmissionStatus::Transmitting { .. })
    }

    pub fn clear_error(&mut self) {
        let mut st = self.status.lock().unwrap();
        if matches!(*st, TransmissionStatus::Error(_)) {
            *st = TransmissionStatus::Idle;
        }
    }

    pub fn start(&mut self, config: TxConfig, iq_samples: Vec<(f32, f32)>) {
        self.stop();

        let stop = Arc::new(AtomicBool::new(false));
        self.stop_signal = Arc::clone(&stop);
        let status = Arc::clone(&self.status);
        *status.lock().unwrap() = TransmissionStatus::Transmitting {
            samples_sent: 0,
            total_samples: iq_samples.len(),
            elapsed_sec: 0.0,
        };

        let handle = thread::spawn(move || {
            let t0 = Instant::now();
            let total = iq_samples.len();

            // 1. Manejo exclusivo del Simulador Dummy
            if config.hw_type == SdrHardwareType::Dummy {
                log::info!("Iniciando simulación en memoria (Modo Dummy)");
                let block_size = 8192;
                let mut offset = 0;
                let sample_duration_sec = 1.0 / (config.sample_rate_hz.max(1) as f32);

                while !stop.load(Ordering::Relaxed) && offset < total {
                    let end = (offset + block_size).min(total);
                    let count = end - offset;
                    let sleep_ms = ((count as f32 * sample_duration_sec) * 1000.0) as u64;
                    thread::sleep(std::time::Duration::from_millis(sleep_ms.min(50)));

                    offset = end;
                    let elapsed = t0.elapsed().as_secs_f32();
                    *status.lock().unwrap() = TransmissionStatus::Transmitting {
                        samples_sent: offset,
                        total_samples: total,
                        elapsed_sec: elapsed,
                    };
                }

                let elapsed = t0.elapsed().as_secs_f32();
                *status.lock().unwrap() = TransmissionStatus::Finished {
                    total_samples: total,
                    elapsed_sec: elapsed,
                };
                return;
            }

            // 2. Hardware Real: Resolver argumentos
            let device_args = match config.resolve_device_args() {
                Ok(args) => args,
                Err(err_msg) => {
                    log::error!("Fallo de resolución de argumentos SDR: {}", err_msg);
                    *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                    return;
                }
            };

            // 3. Validación previa de sample rate para hardware con restricciones físicas
            if config.hw_type == SdrHardwareType::HackRfSeify
                || config.hw_type == SdrHardwareType::HackRfSoapy
                || device_args.contains("hackrf")
            {
                let min_rate = if config.hw_type == SdrHardwareType::HackRfSeify {
                    2_000_000
                } else {
                    1_000_000
                };
                if config.sample_rate_hz < min_rate || config.sample_rate_hz > 20_000_000 {
                    let err_msg = format!(
                        "Tasa de muestreo de {} Hz incompatible con HackRF One. El hardware requiere entre {} Hz ({:.1} MSps) y 20,000,000 Hz (20 MSps). Ajusta la tasa de muestreo del pipeline a un valor compatible.",
                        config.sample_rate_hz, min_rate, min_rate as f32 / 1e6
                    );
                    log::error!("{}", err_msg);
                    *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                    return;
                }
            }

            // Convertir muestras a Complex32
            let complex_samples: Vec<Complex32> = iq_samples
                .iter()
                .map(|&(i, q)| Complex32::new(i, q))
                .collect();

            // 4. Intentar abrir el dispositivo con Seify
            log::info!("Conectando a SDR con argumentos: '{}'", device_args);
            let dev = match DynDevice::from_args(&device_args) {
                Ok(d) => d,
                Err(e) => {
                    let err_str = e.to_string();
                    let friendly_msg = if err_str.contains("DeviceNotFound") {
                        format!(
                            "No se encontró el dispositivo SDR ('{}'). Verifica que esté conectado firmemente al puerto USB y no esté suspendido o desconectado.",
                            device_args
                        )
                    } else if err_str.contains("DriverFeatureNotEnabled") {
                        format!(
                            "El driver requerido para '{}' no está habilitado en este binario: {}",
                            device_args, e
                        )
                    } else {
                        format!(
                            "No se pudo inicializar el dispositivo SDR ('{}'): {}. Verifica que ninguna otra aplicación (SDR#, GQRX, CubicSDR, GNU Radio) esté utilizando el dispositivo actualmente.",
                            device_args, e
                        )
                    };
                    log::error!("{}", friendly_msg);
                    *status.lock().unwrap() = TransmissionStatus::Error(friendly_msg);
                    return;
                }
            };

            // 5. Obtener canal de transmisión TX(0)
            let tx0 = match dev.tx(0) {
                Ok(ch) => ch,
                Err(e) => {
                    let err_msg = format!(
                        "El dispositivo SDR ('{}') no soporta transmisión (TX) o el canal 0 no está disponible: {}",
                        device_args, e
                    );
                    log::error!("{}", err_msg);
                    *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                    return;
                }
            };

            // 6. Configurar Frecuencia Central RF
            log::info!("Configurando frecuencia a {:.3} MHz...", config.frequency_hz / 1e6);
            if let Err(e) = tx0.frequency().set(config.frequency_hz) {
                let err_msg = format!(
                    "Frecuencia central {:.3} MHz rechazada por el dispositivo SDR: {}",
                    config.frequency_hz / 1e6, e
                );
                log::error!("{}", err_msg);
                *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                return;
            }

            // 7. Configurar Tasa de Muestreo
            log::info!("Configurando tasa de muestreo a {} Hz...", config.sample_rate_hz);
            if let Err(e) = tx0.sample_rate().set(config.sample_rate_hz as f64) {
                let err_msg = format!(
                    "Tasa de muestreo de {} Hz rechazada por el dispositivo SDR: {}",
                    config.sample_rate_hz, e
                );
                log::error!("{}", err_msg);
                *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                return;
            }

            // 8. Configurar Ganancias y Opciones específicas de hardware
            match config.hw_type {
                SdrHardwareType::HackRfSeify | SdrHardwareType::HackRfSoapy => {
                    let _ = tx0.gain().set(config.hackrf_vga_gain);
                    let _ = tx0.gain().element("VGA").set(config.hackrf_vga_gain);
                    let amp_val = if config.hackrf_rf_amp { 14.0 } else { 0.0 };
                    let _ = tx0.gain().element("AMP").set(amp_val);
                    log::info!("HackRF: VGA Gain = {:.1} dB, PA Amp = {} dB", config.hackrf_vga_gain, amp_val);
                }
                SdrHardwareType::UsrpSoapy => {
                    let _ = tx0.gain().set(config.usrp_gain);
                    if !config.usrp_antenna.trim().is_empty() {
                        let _ = tx0.antenna().select(&config.usrp_antenna);
                    }
                    log::info!("USRP: Gain = {:.1} dB, Antenna = {}", config.usrp_gain, config.usrp_antenna);
                }
                SdrHardwareType::PlutoSdrSoapy => {
                    let _ = tx0.gain().set(config.pluto_attenuation);
                    if !config.pluto_antenna.trim().is_empty() {
                        let _ = tx0.antenna().select(&config.pluto_antenna);
                    }
                    log::info!("PlutoSDR: Attenuation = {:.1} dB", config.pluto_attenuation);
                }
                SdrHardwareType::BladeRfSoapy => {
                    let _ = tx0.gain().set(config.bladerf_tx_gain);
                    log::info!("bladeRF: TX Gain = {:.1} dB", config.bladerf_tx_gain);
                }
                SdrHardwareType::LimeSdrSoapy => {
                    let _ = tx0.gain().set(config.lime_gain);
                    if !config.lime_antenna.trim().is_empty() {
                        let _ = tx0.antenna().select(&config.lime_antenna);
                    }
                    log::info!("LimeSDR: Gain = {:.1} dB, Antenna = {}", config.lime_gain, config.lime_antenna);
                }
                _ => {
                    let _ = tx0.gain().set(config.hackrf_vga_gain);
                }
            }

            // 9. Crear Streamer de Transmisión
            log::info!("Creando streamer TX...");
            let mut streamer = match tx0.streamer() {
                Ok(s) => s,
                Err(e) => {
                    let err_msg = format!("Error al crear el streamer TX de muestras: {}", e);
                    log::error!("{}", err_msg);
                    *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                    return;
                }
            };

            log::info!("Activando streamer TX en hardware...");
            if let Err(e) = streamer.activate() {
                let err_msg = format!("Error al activar la transmisión en el hardware SDR: {}", e);
                log::error!("{}", err_msg);
                *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                return;
            }

            // 10. Streaming de muestras I/Q reales al hardware
            let block_size = 16384;
            let mut offset = 0;

            log::info!("Transmitiendo {} muestras I/Q por RF...", total);
            while !stop.load(Ordering::Relaxed) {
                if offset >= total {
                    if config.repeat {
                        offset = 0;
                    } else {
                        break;
                    }
                }

                let end = (offset + block_size).min(total);
                let chunk = &complex_samples[offset..end];
                if let Err(e) = streamer.write(&[chunk], None, false, 1_000_000) {
                    let err_msg = format!("Fallo durante la escritura de muestras en el SDR: {}", e);
                    log::error!("{}", err_msg);
                    let _ = streamer.deactivate();
                    *status.lock().unwrap() = TransmissionStatus::Error(err_msg);
                    return;
                }

                offset = end;
                let elapsed = t0.elapsed().as_secs_f32();
                *status.lock().unwrap() = TransmissionStatus::Transmitting {
                    samples_sent: offset,
                    total_samples: total,
                    elapsed_sec: elapsed,
                };
            }

            let _ = streamer.deactivate();
            let elapsed = t0.elapsed().as_secs_f32();
            log::info!("Transmisión de hardware finalizada con éxito: {} muestras en {:.2} s", total, elapsed);
            *status.lock().unwrap() = TransmissionStatus::Finished {
                total_samples: total,
                elapsed_sec: elapsed,
            };
        });

        self.handle = Some(handle);
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        *self.status.lock().unwrap() = TransmissionStatus::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_supported_devices_presence() {
        let seify_devs = SdrDeviceOption::for_backend(SdrBackend::SeifyNative);
        assert!(seify_devs.iter().any(|d| d.hw_type == SdrHardwareType::HackRfSeify));
        assert!(seify_devs.iter().any(|d| d.hw_type == SdrHardwareType::Dummy));

        let soapy_devs = SdrDeviceOption::for_backend(SdrBackend::SoapySdr);
        assert!(soapy_devs.iter().any(|d| d.hw_type == SdrHardwareType::HackRfSoapy));
        assert!(soapy_devs.iter().any(|d| d.hw_type == SdrHardwareType::SoapyAuto));
        assert!(soapy_devs.iter().any(|d| d.hw_type == SdrHardwareType::UsrpSoapy));
        assert!(soapy_devs.iter().any(|d| d.hw_type == SdrHardwareType::PlutoSdrSoapy));
    }

    #[test]
    fn test_resolve_device_args_correctness() {
        let mut cfg = TxConfig::default();

        cfg.backend = SdrBackend::SeifyNative;
        cfg.hw_type = SdrHardwareType::HackRfSeify;
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=hackrf");

        // Con serial especificado para entornos Multi-SDR
        cfg.target_serial = "123456789".to_string();
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=hackrf,serial=123456789");
        cfg.target_serial.clear();

        cfg.hw_type = SdrHardwareType::Dummy;
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=dummy");

        cfg.backend = SdrBackend::SoapySdr;
        cfg.hw_type = SdrHardwareType::HackRfSoapy;
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=hackrf");

        cfg.hw_type = SdrHardwareType::BladeRfSoapy;
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=bladerf");

        cfg.hw_type = SdrHardwareType::LimeSdrSoapy;
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=lime");

        cfg.hw_type = SdrHardwareType::UsrpSoapy;
        cfg.usrp_ip = String::new();
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=uhd");

        cfg.usrp_ip = "192.168.10.2".to_string();
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=uhd,addr=192.168.10.2");

        cfg.hw_type = SdrHardwareType::PlutoSdrSoapy;
        cfg.pluto_uri = String::new();
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=plutosdr");

        cfg.pluto_uri = "ip:192.168.2.1".to_string();
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=plutosdr,uri=ip:192.168.2.1");

        cfg.hw_type = SdrHardwareType::Custom;
        cfg.custom_args = String::new();
        assert!(cfg.resolve_device_args().is_err());

        cfg.custom_args = "driver=soapy,soapy_driver=custom_radio".to_string();
        assert_eq!(cfg.resolve_device_args().unwrap(), "driver=soapy,soapy_driver=custom_radio");
    }

    #[test]
    fn test_dummy_transmission_lifecycle() {
        let mut controller = SdrTransmissionController::new();
        assert_eq!(controller.status(), TransmissionStatus::Idle);
        assert!(!controller.is_transmitting());

        let mut cfg = TxConfig::default();
        cfg.hw_type = SdrHardwareType::Dummy;
        cfg.sample_rate_hz = 1_000_000;

        let samples = vec![(0.5, 0.0); 1000];
        controller.start(cfg, samples);
        std::thread::sleep(std::time::Duration::from_millis(50));
        controller.stop();
        assert_eq!(controller.status(), TransmissionStatus::Idle);
    }
}
