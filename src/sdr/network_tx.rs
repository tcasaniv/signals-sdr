//! Transmisión y streaming en tiempo real de muestras I/Q sobre red local para GNU Radio y clientes SDR remotos.
//!
//! Soporta:
//! 1. Streaming GNU Radio via TCP (servidor local para bloque TCP Source) o UDP (emisor para bloque UDP Source) en formatos fc32 y sc16.
//! 2. Servidor emulador RTL-TCP (puerto 1234) para clientes como SDR#, GQRX, SDR++, WebSDR o GNU Radio rtl_tcp source.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Generador de números pseudoaleatorios ultrarrápido y ligero para ruido AWGN
#[derive(Debug, Clone)]
struct FastRng(u64);

impl FastRng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x853c49e6748fea9b } else { seed })
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    fn next_gaussian(&mut self) -> (f32, f32) {
        let u1 = self.next_f32().max(1e-7);
        let u2 = self.next_f32();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        (r * theta.cos(), r * theta.sin())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GnuRadioProtocol {
    #[default]
    TcpServer,
    UdpSender,
}

impl GnuRadioProtocol {
    pub fn name(&self) -> &'static str {
        match self {
            Self::TcpServer => "TCP Servidor (GNU Radio se conecta como Cliente)",
            Self::UdpSender => "UDP Emisor (Envía directamente a puerto GNU Radio)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GnuRadioFormat {
    #[default]
    Fc32, // Float32 Complex (IEEE 754 Little Endian, 8 bytes por muestra)
    Sc16, // Int16 Complex (Signed 16-bit Q15, 4 bytes por muestra)
}

impl GnuRadioFormat {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fc32 => "Complex Float32 (fc32 - Estándar GNU Radio)",
            Self::Sc16 => "Complex Int16 (sc16 - 16-bit Signed)",
        }
    }

    pub fn bytes_per_sample(&self) -> usize {
        match self {
            Self::Fc32 => 8,
            Self::Sc16 => 4,
        }
    }
}

/// Estadísticas en vivo de la transmisión de red
#[derive(Debug, Clone, Default)]
pub struct NetworkStreamStats {
    pub is_running: bool,
    pub bytes_sent: u64,
    pub clients_connected: usize,
    pub elapsed_secs: f32,
    pub throughput_kbps: f32,
    pub status_message: String,
    pub is_playing_signal: bool,
    pub client_gain_db: f32,
}

#[derive(Debug, Clone)]
pub struct NetworkStreamHandle {
    pub stop_signal: Arc<AtomicBool>,
    pub bytes_sent: Arc<AtomicU64>,
    pub clients_connected: Arc<Mutex<usize>>,
    pub start_time: Instant,
    pub status: Arc<Mutex<String>>,
    pub loop_signal: Arc<AtomicBool>,
    pub retrigger_requested: Arc<AtomicBool>,
    pub simulate_hardware: Arc<AtomicBool>,
    pub tx_attenuation_bits: Arc<AtomicU32>,
    pub client_gain_tenths: Arc<AtomicU32>,
    pub is_playing_signal: Arc<AtomicBool>,
}

/// Controlador unificado de streaming en red
#[derive(Debug, Clone, Default)]
pub struct NetworkTxController {
    handle: Option<NetworkStreamHandle>,
}

impl NetworkTxController {
    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn is_running(&self) -> bool {
        if let Some(ref h) = self.handle {
            !h.stop_signal.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    pub fn stop(&mut self) {
        if let Some(ref h) = self.handle {
            h.stop_signal.store(true, Ordering::Relaxed);
        }
        self.handle = None;
    }

    pub fn retrigger(&self) {
        if let Some(ref h) = self.handle {
            h.retrigger_requested.store(true, Ordering::Relaxed);
        }
    }

    pub fn set_loop(&self, enabled: bool) {
        if let Some(ref h) = self.handle {
            h.loop_signal.store(enabled, Ordering::Relaxed);
        }
    }

    pub fn set_simulate_hardware(&self, sim: bool) {
        if let Some(ref h) = self.handle {
            h.simulate_hardware.store(sim, Ordering::Relaxed);
        }
    }

    pub fn set_tx_attenuation(&self, att_db: f32) {
        if let Some(ref h) = self.handle {
            h.tx_attenuation_bits.store(att_db.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn get_stats(&self) -> NetworkStreamStats {
        if let Some(ref h) = self.handle {
            let is_running = !h.stop_signal.load(Ordering::Relaxed);
            let bytes = h.bytes_sent.load(Ordering::Relaxed);
            let elapsed = h.start_time.elapsed().as_secs_f32().max(0.001);
            let kbps = (bytes as f32 / 1024.0) / elapsed;
            let clients = *h.clients_connected.lock().unwrap_or_else(|e| e.into_inner());
            let status = h.status.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let is_playing_signal = h.is_playing_signal.load(Ordering::Relaxed);
            let client_gain_db = (h.client_gain_tenths.load(Ordering::Relaxed) as f32) / 10.0;

            NetworkStreamStats {
                is_running,
                bytes_sent: bytes,
                clients_connected: clients,
                elapsed_secs: elapsed,
                throughput_kbps: kbps,
                status_message: status,
                is_playing_signal,
                client_gain_db,
            }
        } else {
            NetworkStreamStats::default()
        }
    }

    /// Inicia el streaming para GNU Radio (TCP Server o UDP Sender)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_gnuradio_stream(
        &mut self,
        samples: Vec<(f32, f32)>,
        sample_rate: u32,
        protocol: GnuRadioProtocol,
        format: GnuRadioFormat,
        host: String,
        port: u16,
        continuous_loop: bool,
    ) -> Result<(), String> {
        self.stop();

        if samples.is_empty() {
            return Err("No hay muestras I/Q para transmitir.".to_string());
        }
        if sample_rate == 0 {
            return Err("Tasa de muestreo inválida (0 Hz).".to_string());
        }

        let stop_signal = Arc::new(AtomicBool::new(false));
        let bytes_sent = Arc::new(AtomicU64::new(0));
        let clients_connected = Arc::new(Mutex::new(0));
        let status = Arc::new(Mutex::new("Iniciando transmisión de red...".to_string()));

        let handle = NetworkStreamHandle {
            stop_signal: stop_signal.clone(),
            bytes_sent: bytes_sent.clone(),
            clients_connected: clients_connected.clone(),
            start_time: Instant::now(),
            status: status.clone(),
            loop_signal: Arc::new(AtomicBool::new(continuous_loop)),
            retrigger_requested: Arc::new(AtomicBool::new(false)),
            simulate_hardware: Arc::new(AtomicBool::new(false)),
            tx_attenuation_bits: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
            client_gain_tenths: Arc::new(AtomicU32::new(300)),
            is_playing_signal: Arc::new(AtomicBool::new(true)),
        };

        match protocol {
            GnuRadioProtocol::TcpServer => {
                let bind_addr = format!("{}:{}", host, port);
                let listener = std::net::TcpListener::bind(&bind_addr)
                    .map_err(|e| format!("Error enlazando servidor TCP en {}: {}", bind_addr, e))?;
                listener.set_nonblocking(true)
                    .map_err(|e| format!("Error configurando nonblocking: {}", e))?;

                *status.lock().unwrap() = format!("Servidor TCP esperando conexión de GNU Radio en {}", bind_addr);

                std::thread::spawn(move || {
                    let chunk_size = 4096;
                    let bytes_per_chunk = chunk_size * format.bytes_per_sample();
                    let chunk_duration_sec = chunk_size as f64 / sample_rate as f64;
                    let chunk_duration = std::time::Duration::from_secs_f64(chunk_duration_sec);

                    let mut active_clients: Vec<std::net::TcpStream> = Vec::new();
                    let mut sample_cursor = 0;

                    while !stop_signal.load(Ordering::Relaxed) {
                        let loop_start = Instant::now();

                        // 1. Aceptar nuevos clientes sin bloquear
                        match listener.accept() {
                            Ok((stream, addr)) => {
                                let _ = stream.set_nodelay(true);
                                let _ = stream.set_nonblocking(false);
                                active_clients.push(stream);
                                if let Ok(mut c) = clients_connected.lock() {
                                    *c = active_clients.len();
                                }
                                if let Ok(mut s) = status.lock() {
                                    *s = format!("Cliente conectado desde {}. Transmitiendo...", addr);
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(_) => {}
                        }

                        // 2. Si hay clientes, generar y enviar bloque de muestras
                        if !active_clients.is_empty() {
                            let end_idx = (sample_cursor + chunk_size).min(samples.len());
                            let slice = &samples[sample_cursor..end_idx];
                            sample_cursor = end_idx;

                            if sample_cursor >= samples.len() {
                                if continuous_loop {
                                    sample_cursor = 0;
                                } else {
                                    if let Ok(mut s) = status.lock() {
                                        *s = "Transmisión de muestras completada.".to_string();
                                    }
                                    break;
                                }
                            }

                            // Codificar bytes según formato
                            let mut buffer = Vec::with_capacity(bytes_per_chunk);
                            match format {
                                GnuRadioFormat::Fc32 => {
                                    for &(i, q) in slice {
                                        buffer.extend_from_slice(&i.to_le_bytes());
                                        buffer.extend_from_slice(&q.to_le_bytes());
                                    }
                                }
                                GnuRadioFormat::Sc16 => {
                                    for &(i, q) in slice {
                                        let i16_i = (i.clamp(-1.0, 1.0) * 32767.0) as i16;
                                        let i16_q = (q.clamp(-1.0, 1.0) * 32767.0) as i16;
                                        buffer.extend_from_slice(&i16_i.to_le_bytes());
                                        buffer.extend_from_slice(&i16_q.to_le_bytes());
                                    }
                                }
                            }

                            // Enviar a todos los clientes activos
                            use std::io::Write;
                            active_clients.retain_mut(|client| {
                                match client.write_all(&buffer) {
                                    Ok(_) => true,
                                    Err(_) => false,
                                }
                            });

                            if let Ok(mut c) = clients_connected.lock() {
                                *c = active_clients.len();
                            }
                            bytes_sent.fetch_add(buffer.len() as u64, Ordering::Relaxed);
                        }

                        // 3. Control estricto de ritmo (Pacing) basado en la tasa de muestreo
                        let elapsed = loop_start.elapsed();
                        if elapsed < chunk_duration {
                            std::thread::sleep(chunk_duration - elapsed);
                        }
                    }

                    stop_signal.store(true, Ordering::Relaxed);
                    if let Ok(mut c) = clients_connected.lock() {
                        *c = 0;
                    }
                });
            }
            GnuRadioProtocol::UdpSender => {
                let target_addr = format!("{}:{}", host, port);
                let socket = std::net::UdpSocket::bind("0.0.0.0:0")
                    .map_err(|e| format!("Error abriendo socket UDP local: {}", e))?;

                *status.lock().unwrap() = format!("Transmitiendo UDP hacia GNU Radio en {}", target_addr);
                *clients_connected.lock().unwrap() = 1;

                std::thread::spawn(move || {
                    let chunk_size = 512; // Muestras por paquete UDP (~4KB para fc32, evita fragmentación IP)
                    let bytes_per_chunk = chunk_size * format.bytes_per_sample();
                    let chunk_duration_sec = chunk_size as f64 / sample_rate as f64;
                    let chunk_duration = std::time::Duration::from_secs_f64(chunk_duration_sec);

                    let mut sample_cursor = 0;

                    while !stop_signal.load(Ordering::Relaxed) {
                        let loop_start = Instant::now();

                        let end_idx = (sample_cursor + chunk_size).min(samples.len());
                        let slice = &samples[sample_cursor..end_idx];
                        sample_cursor = end_idx;

                        if sample_cursor >= samples.len() {
                            if continuous_loop {
                                sample_cursor = 0;
                            } else {
                                if let Ok(mut s) = status.lock() {
                                    *s = "Transmisión UDP completada.".to_string();
                                }
                                break;
                            }
                        }

                        let mut buffer = Vec::with_capacity(bytes_per_chunk);
                        match format {
                            GnuRadioFormat::Fc32 => {
                                for &(i, q) in slice {
                                    buffer.extend_from_slice(&i.to_le_bytes());
                                    buffer.extend_from_slice(&q.to_le_bytes());
                                }
                            }
                            GnuRadioFormat::Sc16 => {
                                for &(i, q) in slice {
                                    let i16_i = (i.clamp(-1.0, 1.0) * 32767.0) as i16;
                                    let i16_q = (q.clamp(-1.0, 1.0) * 32767.0) as i16;
                                    buffer.extend_from_slice(&i16_i.to_le_bytes());
                                    buffer.extend_from_slice(&i16_q.to_le_bytes());
                                }
                            }
                        }

                        if let Ok(n) = socket.send_to(&buffer, &target_addr) {
                            bytes_sent.fetch_add(n as u64, Ordering::Relaxed);
                        }

                        let elapsed = loop_start.elapsed();
                        if elapsed < chunk_duration {
                            std::thread::sleep(chunk_duration - elapsed);
                        }
                    }

                    stop_signal.store(true, Ordering::Relaxed);
                    if let Ok(mut c) = clients_connected.lock() {
                        *c = 0;
                    }
                });
            }
        }

        self.handle = Some(handle);
        Ok(())
    }

    /// Inicia el servidor emulador RTL-TCP (puerto estándar 1234)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_rtl_tcp_server(
        &mut self,
        samples: Vec<(f32, f32)>,
        sample_rate: u32,
        host: String,
        port: u16,
        continuous_loop: bool,
        simulate_hardware: bool,
        tx_attenuation_db: f32,
    ) -> Result<(), String> {
        self.stop();

        if samples.is_empty() {
            return Err("No hay muestras I/Q para transmitir.".to_string());
        }
        if sample_rate == 0 {
            return Err("Tasa de muestreo inválida (0 Hz).".to_string());
        }

        let stop_signal = Arc::new(AtomicBool::new(false));
        let bytes_sent = Arc::new(AtomicU64::new(0));
        let clients_connected = Arc::new(Mutex::new(0));
        let status = Arc::new(Mutex::new("Iniciando servidor RTL-TCP...".to_string()));

        let bind_addr = format!("{}:{}", host, port);
        let listener = std::net::TcpListener::bind(&bind_addr)
            .map_err(|e| format!("Error enlazando servidor RTL-TCP en {}: {}", bind_addr, e))?;
        listener.set_nonblocking(true)
            .map_err(|e| format!("Error configurando nonblocking: {}", e))?;

        *status.lock().unwrap() = format!("Servidor RTL-TCP listo en {}. Esperando cliente (SDR#, GQRX, SDR++, etc.)...", bind_addr);

        let loop_signal = Arc::new(AtomicBool::new(continuous_loop));
        let retrigger_requested = Arc::new(AtomicBool::new(false));
        let simulate_hardware_atom = Arc::new(AtomicBool::new(simulate_hardware));
        let tx_attenuation_bits = Arc::new(AtomicU32::new(tx_attenuation_db.to_bits()));
        let client_gain_tenths = Arc::new(AtomicU32::new(297)); // 29.7 dB por defecto
        let is_playing_signal = Arc::new(AtomicBool::new(true));

        let handle = NetworkStreamHandle {
            stop_signal: stop_signal.clone(),
            bytes_sent: bytes_sent.clone(),
            clients_connected: clients_connected.clone(),
            start_time: Instant::now(),
            status: status.clone(),
            loop_signal: loop_signal.clone(),
            retrigger_requested: retrigger_requested.clone(),
            simulate_hardware: simulate_hardware_atom.clone(),
            tx_attenuation_bits: tx_attenuation_bits.clone(),
            client_gain_tenths: client_gain_tenths.clone(),
            is_playing_signal: is_playing_signal.clone(),
        };

        std::thread::spawn(move || {
            use std::io::{Read, Write};

            let chunk_samples = 8192; // 16384 bytes por paquete RTL
            let bytes_per_chunk = chunk_samples * 2;
            let chunk_duration_sec = chunk_samples as f64 / sample_rate as f64;
            let chunk_duration = std::time::Duration::from_secs_f64(chunk_duration_sec);

            let mut active_clients: Vec<std::net::TcpStream> = Vec::new();
            let mut sample_cursor: usize = 0;
            let mut rng = FastRng::new(123456789);

            // Encabezado dongle RTL0 estándar:
            // "RTL0" (4 bytes) + Tuner Type R820T (5, uint32_be) + Tuner Gains Count (29, uint32_be)
            let mut rtl0_header = [0u8; 12];
            rtl0_header[0..4].copy_from_slice(b"RTL0");
            rtl0_header[4..8].copy_from_slice(&5u32.to_be_bytes());
            rtl0_header[8..12].copy_from_slice(&29u32.to_be_bytes());

            while !stop_signal.load(Ordering::Relaxed) {
                let loop_start = Instant::now();

                // 1. Aceptar clientes
                match listener.accept() {
                    Ok((mut stream, addr)) => {
                        let _ = stream.set_nodelay(true);
                        // Enviar encabezado dongle RTL0 requerido por los clientes SDR
                        if stream.write_all(&rtl0_header).is_ok() {
                            let _ = stream.set_nonblocking(true);
                            active_clients.push(stream);
                            if let Ok(mut c) = clients_connected.lock() {
                                *c = active_clients.len();
                            }
                            if let Ok(mut s) = status.lock() {
                                *s = format!("Cliente SDR conectado desde {}. Transmitiendo señal...", addr);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => {}
                }

                // 2. Procesar comandos entrantes de los clientes (5 bytes: 1 byte cmd + 4 bytes param BE)
                let mut cmd_buf = [0u8; 5];
                for client in &mut active_clients {
                    while let Ok(5) = client.read(&mut cmd_buf) {
                        let cmd_id = cmd_buf[0];
                        let param = u32::from_be_bytes([cmd_buf[1], cmd_buf[2], cmd_buf[3], cmd_buf[4]]);
                        match cmd_id {
                            0x01 => {
                                if let Ok(mut s) = status.lock() {
                                    *s = format!("Cliente sintonizó frecuencia central: {:.3} MHz", param as f64 / 1_000_000.0);
                                }
                            }
                            0x02 => {
                                if let Ok(mut s) = status.lock() {
                                    *s = format!("Cliente configuró sample rate: {} Sps", param);
                                }
                            }
                            0x03 => {
                                if param == 0 {
                                    client_gain_tenths.store(297, Ordering::Relaxed);
                                }
                                if let Ok(mut s) = status.lock() {
                                    *s = format!("Cliente modo ganancia: {}", if param == 0 { "AGC (Automático)" } else { "Manual" });
                                }
                            }
                            0x04 => {
                                client_gain_tenths.store(param, Ordering::Relaxed);
                                if let Ok(mut s) = status.lock() {
                                    *s = format!("Cliente configuró ganancia RF: {:.1} dB", param as f32 / 10.0);
                                }
                            }
                            0x08 => {
                                if param != 0 {
                                    client_gain_tenths.store(297, Ordering::Relaxed);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // 3. Chequeo de retrigger manual
                if retrigger_requested.swap(false, Ordering::Relaxed) {
                    sample_cursor = 0;
                    is_playing_signal.store(true, Ordering::Relaxed);
                    if let Ok(mut s) = status.lock() {
                        *s = "Re-transmitiendo señal I/Q desde el inicio...".to_string();
                    }
                }

                // 4. Enviar muestras continuas con pacing
                if !active_clients.is_empty() {
                    let is_sim = simulate_hardware_atom.load(Ordering::Relaxed);
                    let loop_active = loop_signal.load(Ordering::Relaxed);
                    let att_db = f32::from_bits(tx_attenuation_bits.load(Ordering::Relaxed));
                    let tx_scale = 10.0_f32.powf(att_db / 20.0);

                    let gain_tenths = client_gain_tenths.load(Ordering::Relaxed) as f32;
                    let gain_db = (gain_tenths / 10.0).clamp(0.0, 49.6);

                    let (rx_gain_factor, base_noise_std) = if is_sim {
                        let gain_norm = (gain_db / 38.0).clamp(0.0, 1.0);
                        let rx_factor = 0.01 + 0.99 * gain_norm;
                        let noise_std = if gain_db > 38.0 {
                            let excess = (gain_db - 38.0) / (49.6 - 38.0);
                            0.015 + 0.22 * excess.powf(1.5)
                        } else {
                            0.015 * (gain_norm.max(0.08))
                        };
                        (rx_factor, noise_std)
                    } else {
                        (1.0, 0.0)
                    };

                    let mut chunk_buf = Vec::with_capacity(bytes_per_chunk);

                    for _ in 0..chunk_samples {
                        let (raw_i, raw_q) = if sample_cursor < samples.len() {
                            let s = samples[sample_cursor];
                            sample_cursor += 1;
                            (s.0, s.1)
                        } else if loop_active {
                            sample_cursor = 0;
                            let s = samples[sample_cursor];
                            sample_cursor += 1;
                            (s.0, s.1)
                        } else {
                            if is_playing_signal.swap(false, Ordering::Relaxed) {
                                if let Ok(mut s) = status.lock() {
                                    *s = "Señal I/Q finalizada. Transmitiendo piso de ruido base...".to_string();
                                }
                            }
                            (0.0, 0.0)
                        };

                        let is_playing = is_playing_signal.load(Ordering::Relaxed);
                        let (gauss_i, gauss_q) = if is_sim || !is_playing {
                            rng.next_gaussian()
                        } else {
                            (0.0, 0.0)
                        };

                        let out_i = if is_playing {
                            (raw_i * tx_scale * rx_gain_factor) + (gauss_i * base_noise_std)
                        } else {
                            gauss_i * (base_noise_std.max(0.025))
                        };

                        let out_q = if is_playing {
                            (raw_q * tx_scale * rx_gain_factor) + (gauss_q * base_noise_std)
                        } else {
                            gauss_q * (base_noise_std.max(0.025))
                        };

                        let u8_i = ((out_i.clamp(-1.0, 1.0) * 127.5) + 127.5) as u8;
                        let u8_q = ((out_q.clamp(-1.0, 1.0) * 127.5) + 127.5) as u8;
                        chunk_buf.push(u8_i);
                        chunk_buf.push(u8_q);
                    }

                    active_clients.retain_mut(|client| {
                        client.write_all(&chunk_buf).is_ok()
                    });

                    if let Ok(mut c) = clients_connected.lock() {
                        *c = active_clients.len();
                    }
                    bytes_sent.fetch_add(chunk_buf.len() as u64, Ordering::Relaxed);
                }

                let elapsed = loop_start.elapsed();
                if elapsed < chunk_duration {
                    std::thread::sleep(chunk_duration - elapsed);
                }
            }

            stop_signal.store(true, Ordering::Relaxed);
            if let Ok(mut c) = clients_connected.lock() {
                *c = 0;
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start_gnuradio_stream(
        &mut self,
        _samples: Vec<(f32, f32)>,
        _sample_rate: u32,
        _protocol: GnuRadioProtocol,
        _format: GnuRadioFormat,
        _host: String,
        _port: u16,
        _continuous_loop: bool,
    ) -> Result<(), String> {
        Err("La transmisión de red TCP/UDP solo está disponible en la versión de escritorio.".to_string())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start_rtl_tcp_server(
        &mut self,
        _samples: Vec<(f32, f32)>,
        _sample_rate: u32,
        _host: String,
        _port: u16,
        _continuous_loop: bool,
        _simulate_hardware: bool,
        _tx_attenuation_db: f32,
    ) -> Result<(), String> {
        Err("El servidor RTL-TCP solo está disponible en la versión de escritorio.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gnuradio_formats() {
        assert_eq!(GnuRadioFormat::Fc32.bytes_per_sample(), 8);
        assert_eq!(GnuRadioFormat::Sc16.bytes_per_sample(), 4);
    }

    #[test]
    fn test_network_controller_default() {
        let mut controller = NetworkTxController::new();
        assert!(!controller.is_running());
        let stats = controller.get_stats();
        assert_eq!(stats.bytes_sent, 0);
        assert!(!stats.is_running);
        controller.stop();
        assert!(!controller.is_running());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_gnuradio_udp_stream_lifecycle() {
        let mut controller = NetworkTxController::new();
        let samples = vec![(0.5f32, -0.5f32); 1024];
        let res = controller.start_gnuradio_stream(
            samples,
            48_000,
            GnuRadioProtocol::UdpSender,
            GnuRadioFormat::Fc32,
            "127.0.0.1".to_string(),
            15999, // Puerto alto para test
            false,
        );
        assert!(res.is_ok());
        assert!(controller.is_running());
        controller.stop();
        assert!(!controller.is_running());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_rtl_tcp_server_lifecycle() {
        let mut controller = NetworkTxController::new();
        let samples = vec![(0.1f32, 0.2f32); 512];
        let res = controller.start_rtl_tcp_server(
            samples,
            250_000,
            "127.0.0.1".to_string(),
            15998, // Puerto alto para test
            true,
            true,
            0.0,
        );
        assert!(res.is_ok());
        assert!(controller.is_running());
        controller.retrigger();
        controller.set_loop(false);
        controller.set_simulate_hardware(false);
        controller.set_tx_attenuation(-20.0);
        let stats = controller.get_stats();
        assert!(stats.is_running);
        controller.stop();
        assert!(!controller.is_running());
    }
}
