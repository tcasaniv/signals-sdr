//! Transmisión y streaming en tiempo real de muestras I/Q sobre red local para GNU Radio y clientes SDR remotos.
//!
//! Soporta:
//! 1. Streaming GNU Radio via TCP (servidor local para bloque TCP Source) o UDP (emisor para bloque UDP Source) en formatos fc32 y sc16.
//! 2. Servidor emulador RTL-TCP (puerto 1234) para clientes como SDR#, GQRX, SDR++, WebSDR o GNU Radio rtl_tcp source.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
}

#[derive(Debug, Clone)]
pub struct NetworkStreamHandle {
    pub stop_signal: Arc<AtomicBool>,
    pub bytes_sent: Arc<AtomicU64>,
    pub clients_connected: Arc<Mutex<usize>>,
    pub start_time: Instant,
    pub status: Arc<Mutex<String>>,
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

    pub fn get_stats(&self) -> NetworkStreamStats {
        if let Some(ref h) = self.handle {
            let is_running = !h.stop_signal.load(Ordering::Relaxed);
            let bytes = h.bytes_sent.load(Ordering::Relaxed);
            let elapsed = h.start_time.elapsed().as_secs_f32().max(0.001);
            let kbps = (bytes as f32 / 1024.0) / elapsed;
            let clients = *h.clients_connected.lock().unwrap_or_else(|e| e.into_inner());
            let status = h.status.lock().unwrap_or_else(|e| e.into_inner()).clone();

            NetworkStreamStats {
                is_running,
                bytes_sent: bytes,
                clients_connected: clients,
                elapsed_secs: elapsed,
                throughput_kbps: kbps,
                status_message: status,
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

        let handle = NetworkStreamHandle {
            stop_signal: stop_signal.clone(),
            bytes_sent: bytes_sent.clone(),
            clients_connected: clients_connected.clone(),
            start_time: Instant::now(),
            status: status.clone(),
        };

        std::thread::spawn(move || {
            use std::io::{Read, Write};

            // Convertir muestras I/Q (float -1..1) a formato RTL-SDR (uint8 0..255 intercalado I/Q)
            let mut rtl_bytes = Vec::with_capacity(samples.len() * 2);
            for &(i, q) in &samples {
                let u8_i = ((i.clamp(-1.0, 1.0) * 127.5) + 127.5) as u8;
                let u8_q = ((q.clamp(-1.0, 1.0) * 127.5) + 127.5) as u8;
                rtl_bytes.push(u8_i);
                rtl_bytes.push(u8_q);
            }

            let chunk_samples = 8192; // 16384 bytes por paquete RTL
            let bytes_per_chunk = chunk_samples * 2;
            let chunk_duration_sec = chunk_samples as f64 / sample_rate as f64;
            let chunk_duration = std::time::Duration::from_secs_f64(chunk_duration_sec);

            let mut active_clients: Vec<std::net::TcpStream> = Vec::new();
            let mut sample_byte_cursor = 0;

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
                            _ => {}
                        }
                    }
                }

                // 3. Enviar muestras continuas con pacing
                if !active_clients.is_empty() {
                    let end_idx = (sample_byte_cursor + bytes_per_chunk).min(rtl_bytes.len());
                    let slice = &rtl_bytes[sample_byte_cursor..end_idx];
                    sample_byte_cursor = end_idx;

                    if sample_byte_cursor >= rtl_bytes.len() {
                        sample_byte_cursor = 0; // Bucle continuo
                    }

                    active_clients.retain_mut(|client| {
                        client.write_all(slice).is_ok()
                    });

                    if let Ok(mut c) = clients_connected.lock() {
                        *c = active_clients.len();
                    }
                    bytes_sent.fetch_add(slice.len() as u64, Ordering::Relaxed);
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
        );
        assert!(res.is_ok());
        assert!(controller.is_running());
        controller.stop();
        assert!(!controller.is_running());
    }
}
