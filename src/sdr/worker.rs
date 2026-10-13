use num_complex::Complex32;
use seify::{DynDevice, TxStreamer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct SdrDeviceOption {
    pub display_name: String,
    pub args: String,
}

impl SdrDeviceOption {
    pub fn all_supported() -> Vec<SdrDeviceOption> {
        vec![
            SdrDeviceOption {
                display_name: "Simulador Dummy (Sin hardware)".to_string(),
                args: "driver=dummy".to_string(),
            },
            SdrDeviceOption {
                display_name: "HackRF One (Transmisión Real)".to_string(),
                args: "driver=hackrf".to_string(),
            },
            SdrDeviceOption {
                display_name: "bladeRF (Transmisión Real)".to_string(),
                args: "driver=bladerf".to_string(),
            },
            SdrDeviceOption {
                display_name: "USRP / UHD (Transmisión Real UHD)".to_string(),
                args: "driver=usrp".to_string(),
            },
            SdrDeviceOption {
                display_name: "PlutoSDR / ADALM-Pluto (Transmisión Real)".to_string(),
                args: "driver=plutosdr".to_string(),
            },
            SdrDeviceOption {
                display_name: "LimeSDR (Transmisión Real)".to_string(),
                args: "driver=limesdr".to_string(),
            },
            SdrDeviceOption {
                display_name: "RTL-SDR (Sólo Recepción / Test)".to_string(),
                args: "driver=rtlsdr".to_string(),
            },
            SdrDeviceOption {
                display_name: "SoapySDR (Detección Automática)".to_string(),
                args: "".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone)]
pub struct TxConfig {
    pub device_args: String,
    pub frequency_hz: f64,
    pub sample_rate_hz: u32,
    pub gain_db: f64,
    pub rf_amp: bool,
    pub repeat: bool,
}

impl Default for TxConfig {
    fn default() -> Self {
        Self {
            device_args: "driver=dummy".to_string(),
            frequency_hz: 137_100_000.0,
            sample_rate_hz: 288_000,
            gain_db: 30.0,
            rf_amp: false,
            repeat: false,
        }
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

    /// Retorna todos los dispositivos SDR compatibles (HackRF, bladeRF, USRP, PlutoSDR, LimeSDR, Dummy, etc.)
    pub fn probe_devices() -> Vec<SdrDeviceOption> {
        SdrDeviceOption::all_supported()
    }

    pub fn status(&self) -> TransmissionStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn is_transmitting(&self) -> bool {
        matches!(self.status(), TransmissionStatus::Transmitting { .. })
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

            // Convertir muestras a Complex32
            let complex_samples: Vec<Complex32> = iq_samples
                .iter()
                .map(|&(i, q)| Complex32::new(i, q))
                .collect();

            // Intentar abrir el dispositivo con Seify
            let dev_res = DynDevice::from_args(&config.device_args);

            match dev_res {
                Ok(dev) => {
                    if let Ok(tx0) = dev.tx(0) {
                        let _ = tx0.frequency().set(config.frequency_hz);
                        let _ = tx0.sample_rate().set(config.sample_rate_hz as f64);
                        let _ = tx0.gain().set(config.gain_db);

                        if let Ok(mut streamer) = tx0.streamer() {
                            let _ = streamer.activate();
                            let block_size = 16384;
                            let mut offset = 0;

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
                                let _ = streamer.write(&[chunk], None, false, 500_000);

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
                            *status.lock().unwrap() = TransmissionStatus::Finished {
                                total_samples: total,
                                elapsed_sec: elapsed,
                            };
                            return;
                        }
                    }
                }
                Err(e) => {
                    // Fallback simulador si el driver de hardware real no está presente
                    log::warn!("No se pudo inicializar dispositivo real: {}, simulando transmisión", e);
                }
            }

            // Simulación en tiempo real si el driver es dummy o falló
            let block_size = 8192;
            let mut offset = 0;
            let sample_duration_sec = 1.0 / (config.sample_rate_hz as f32);

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
