//! Servidor Web LAN integrado para Signals SDR
//!
//! Permite exponer la aplicación en la red local (`http://0.0.0.0:8080`)
//! para que cualquier dispositivo en la misma red Wi-Fi (celulares Android,
//! iPhones, iPads, tablets o laptops) pueda conectarse e interactuar
//! con los transceptores SDR conectados físicamente a la máquina anfitriona.

use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use crate::sdr::SdrTransmissionController;

fn get_host_controller() -> &'static Mutex<SdrTransmissionController> {
    static CONTROLLER: OnceLock<Mutex<SdrTransmissionController>> = OnceLock::new();
    CONTROLLER.get_or_init(|| Mutex::new(SdrTransmissionController::new()))
}

/// Configuración de ejecución del servidor web
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_address: IpAddr,
    pub port: u16,
    pub web_dir: PathBuf,
    pub open_browser: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            port: 8080,
            web_dir: PathBuf::from("dist"),
            open_browser: false,
        }
    }
}

/// Analiza los argumentos de línea de comandos para determinar si se ejecuta en modo servidor
pub enum CliMode {
    Desktop,
    Serve(ServerConfig),
    Help,
    Version,
}

pub fn parse_cli_args() -> CliMode {
    let args: Vec<String> = std::env::args().collect();

    let mut serve = false;
    let mut port = 8080;
    let mut bind_address = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let mut web_dir = PathBuf::from("dist");
    let mut open_browser = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return CliMode::Help,
            "-v" | "--version" => return CliMode::Version,
            "-s" | "--serve" => {
                serve = true;
            }
            "-p" | "--port" => {
                if i + 1 < args.len() {
                    if let Ok(p) = args[i + 1].parse::<u16>() {
                        port = p;
                        i += 1;
                    }
                }
            }
            "-b" | "--bind" => {
                if i + 1 < args.len() {
                    if let Ok(ip) = args[i + 1].parse::<IpAddr>() {
                        bind_address = ip;
                        i += 1;
                    }
                }
            }
            "-d" | "--dir" => {
                if i + 1 < args.len() {
                    web_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "-o" | "--open" => {
                open_browser = true;
            }
            "--desktop" => {
                return CliMode::Desktop;
            }
            _ => {}
        }
        i += 1;
    }

    if serve {
        CliMode::Serve(ServerConfig {
            bind_address,
            port,
            web_dir,
            open_browser,
        })
    } else {
        CliMode::Desktop
    }
}

pub fn print_help() {
    println!(
        "\
Signals SDR v{} - Generador Didáctico de Señales de Radio y Satélites

USO:
    signals-sdr [OPCIONES]

OPCIONES:
    -s, --serve               Inicia el servidor web LAN para acceso remoto
    -p, --port <PUERTO>       Puerto para el servidor web (por defecto: 8080)
    -b, --bind <DIRECCIÓN>    Dirección IP de escucha (por defecto: 0.0.0.0)
    -d, --dir <RUTA>          Directorio de archivos web estáticos (por defecto: dist o web/dist)
    -o, --open                Abre automáticamente el navegador en la URL del servidor
    -h, --help                Muestra esta ayuda
    -v, --version             Muestra la versión de la aplicación

MODALIDADES DE EJECUCIÓN:
    1. Modo Escritorio (Predeterminado):
       Ejecuta la interfaz nativa en Windows, Linux o macOS.
       Ejemplo: `signals-sdr`

    2. Modo Servidor Web LAN:
       Expone la aplicación en la red local para control remoto inalámbrico
       desde teléfonos (Android/iOS), tablets (iPad) o laptops vía Wi-Fi.
       Ejemplo: `signals-sdr --serve --port 8080`

Para más detalles, consulte: https://github.com/apt-tx/signals-sdr",
        env!("CARGO_PKG_VERSION")
    );
}

pub fn print_version() {
    println!("Signals SDR v{}", env!("CARGO_PKG_VERSION"));
}

/// Obtiene la dirección IP local de la máquina en la red LAN
fn get_local_lan_ip() -> Option<IpAddr> {
    // Intenta conectar un socket UDP ficticio a una dirección pública para determinar la interfaz activa
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip())
}

/// Inicia el servidor web LAN bloqueante
pub fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::new(config.bind_address, config.port);
    let listener = TcpListener::bind(addr)?;

    let lan_ip = get_local_lan_ip().unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

    println!("================================================================================");
    println!(" Signals SDR v{} - Servidor Web LAN Iniciado", env!("CARGO_PKG_VERSION"));
    println!("================================================================================");
    println!(" Acceso Local:      http://localhost:{}", config.port);
    println!(" Acceso en Red LAN: http://{}:{} (desde tu celular, tablet o laptop)", lan_ip, config.port);
    println!(" Directorio Web:    {}", config.web_dir.display());
    println!(" Control SDR:       Hardware físico (HackRF, bladeRF, USRP) habilitado");
    println!("================================================================================");
    println!(" Presiona Ctrl+C para detener el servidor.\n");

    if config.open_browser {
        let url = format!("http://localhost:{}", config.port);
        open_browser_url(&url);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Resolver el directorio estático real (dist, web/dist, o fallback embebido)
    let resolved_dir = resolve_static_dir(&config.web_dir);

    for stream in listener.incoming() {
        if !r.load(Ordering::Relaxed) {
            break;
        }

        match stream {
            Ok(stream) => {
                let dir = resolved_dir.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_connection(stream, &dir) {
                        eprintln!("[Server] Error al procesar petición: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("[Server] Error en conexión entrante: {}", e);
            }
        }
    }

    Ok(())
}

fn resolve_static_dir(dir: &Path) -> PathBuf {
    if dir.exists() && dir.is_dir() {
        return dir.to_path_buf();
    }
    let alt1 = PathBuf::from("web/dist");
    if alt1.exists() && alt1.is_dir() {
        return alt1;
    }
    let alt2 = PathBuf::from("web");
    if alt2.exists() && alt2.is_dir() {
        return alt2;
    }
    dir.to_path_buf()
}

fn handle_connection(mut stream: TcpStream, static_dir: &Path) -> std::io::Result<()> {
    let mut buffer = [0u8; 8192];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request_str.lines();
    let request_line = match lines.next() {
        Some(line) => line,
        None => return Ok(()),
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let mut raw_path = parts[1];
    if let Some(idx) = raw_path.find('?') {
        raw_path = &raw_path[..idx];
    }
    if let Some(idx) = raw_path.find('#') {
        raw_path = &raw_path[..idx];
    }

    // Soporte CORS preflight
    if method == "OPTIONS" {
        let response = "HTTP/1.1 204 No Content\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                        Access-Control-Allow-Headers: Content-Type\r\n\
                        Content-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }

    // Rutas de API REST para control de transmisión de hardware por LAN
    if raw_path == "/api/status" && method == "GET" {
        let st = get_host_controller().lock().unwrap().status();
        let json = match st {
            crate::sdr::TransmissionStatus::Idle => {
                serde_json::json!({ "status": "idle" }).to_string()
            }
            crate::sdr::TransmissionStatus::Transmitting { samples_sent, total_samples, elapsed_sec } => {
                serde_json::json!({
                    "status": "transmitting",
                    "samples_sent": samples_sent,
                    "total_samples": total_samples,
                    "elapsed_sec": elapsed_sec,
                }).to_string()
            }
            crate::sdr::TransmissionStatus::Finished { total_samples, elapsed_sec } => {
                serde_json::json!({
                    "status": "finished",
                    "total_samples": total_samples,
                    "elapsed_sec": elapsed_sec,
                }).to_string()
            }
            crate::sdr::TransmissionStatus::Error(msg) => {
                serde_json::json!({
                    "status": "error",
                    "message": msg,
                }).to_string()
            }
        };
        return serve_json(&mut stream, 200, &json);
    }

    if raw_path == "/api/devices" && method == "GET" {
        let devs = crate::sdr::detect_detailed_radios();
        let json = serde_json::to_string(&devs).unwrap_or_else(|_| "[]".to_string());
        return serve_json(&mut stream, 200, &json);
    }

    if raw_path == "/api/transmit" && method == "POST" {
        let body = if let Some(idx) = request_str.find("\r\n\r\n") {
            &request_str[idx + 4..]
        } else {
            ""
        };
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            let mut cfg = crate::sdr::TxConfig::default();
            if let Some(f) = v["frequency_hz"].as_f64() {
                cfg.frequency_hz = f;
            }
            if let Some(sr) = v["sample_rate_hz"].as_u64() {
                cfg.sample_rate_hz = sr as u32;
            }
            if let Some(hw) = v["hw_type"].as_str() {
                match hw {
                    "HackRfSeify" => cfg.hw_type = crate::sdr::SdrHardwareType::HackRfSeify,
                    "HackRfSoapy" => cfg.hw_type = crate::sdr::SdrHardwareType::HackRfSoapy,
                    "BladeRfSoapy" => cfg.hw_type = crate::sdr::SdrHardwareType::BladeRfSoapy,
                    "UsrpSoapy" => cfg.hw_type = crate::sdr::SdrHardwareType::UsrpSoapy,
                    "PlutoSdrSoapy" => cfg.hw_type = crate::sdr::SdrHardwareType::PlutoSdrSoapy,
                    "LimeSdrSoapy" => cfg.hw_type = crate::sdr::SdrHardwareType::LimeSdrSoapy,
                    "Dummy" => cfg.hw_type = crate::sdr::SdrHardwareType::Dummy,
                    _ => cfg.hw_type = crate::sdr::SdrHardwareType::HackRfSeify,
                }
            }
            if let Some(args) = v["device_args"].as_str() {
                if !args.is_empty() {
                    cfg.device_args = args.to_string();
                }
            }
            cfg.repeat = v["repeat"].as_bool().unwrap_or(false);

            let count = v["samples_count"].as_u64().unwrap_or(2_000_000) as usize;
            let mut samples = Vec::with_capacity(count.min(2_000_000));
            for i in 0..count.min(2_000_000) {
                let t = i as f32 / cfg.sample_rate_hz as f32;
                let phase = 2.0 * std::f32::consts::PI * 1000.0 * t;
                samples.push((0.7 * phase.cos(), 0.7 * phase.sin()));
            }

            get_host_controller().lock().unwrap().start(cfg, samples);
            return serve_json(&mut stream, 200, r#"{"status":"started"}"#);
        } else {
            return serve_json(&mut stream, 400, r#"{"error":"Payload JSON no válido"}"#);
        }
    }

    if raw_path == "/api/stop" && method == "POST" {
        get_host_controller().lock().unwrap().stop();
        return serve_json(&mut stream, 200, r#"{"status":"stopped"}"#);
    }

    if method != "GET" {
        let response = "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }

    let clean_path = raw_path.trim_start_matches('/');
    let target_path = if clean_path.is_empty() {
        static_dir.join("index.html")
    } else {
        static_dir.join(clean_path)
    };

    // Servir archivo estático si existe
    if target_path.exists() && target_path.is_file() {
        serve_file(&mut stream, &target_path)?;
    } else {
        // SPA Fallback: intentar servir index.html
        let index_path = static_dir.join("index.html");
        if index_path.exists() && index_path.is_file() {
            serve_file(&mut stream, &index_path)?;
        } else {
            // Servir página informativa embebida M3
            serve_embedded_dashboard(&mut stream)?;
        }
    }

    Ok(())
}

fn serve_file(stream: &mut TcpStream, path: &Path) -> std::io::Result<()> {
    let content = fs::read(path)?;
    let mime = get_mime_type(path);

    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Cache-Control: no-cache\r\n\
         Connection: close\r\n\r\n",
        mime,
        content.len()
    );

    stream.write_all(header.as_bytes())?;
    stream.write_all(&content)?;
    Ok(())
}

fn serve_json(stream: &mut TcpStream, status_code: u16, json: &str) -> std::io::Result<()> {
    let status_line = match status_code {
        200 => "200 OK",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        _ => "500 Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Cache-Control: no-cache\r\n\
         Connection: close\r\n\r\n",
        status_line,
        json.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(json.as_bytes())?;
    Ok(())
}

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "json" => "application/json",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

fn serve_embedded_dashboard(stream: &mut TcpStream) -> std::io::Result<()> {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Signals SDR - Servidor Web LAN</title>
    <style>
        :root {{
            --bg-color: #0e141b;
            --surface: #17212b;
            --surface-high: #202d3b;
            --primary: #52a5ff;
            --text-primary: #e6edf3;
            --text-secondary: #9aa8b6;
            --border: #2a394a;
            --badge-bg: #1e3a5f;
            --badge-fg: #8ac0ff;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background-color: var(--bg-color);
            color: var(--text-primary);
            margin: 0;
            padding: 24px;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 90vh;
        }}
        .card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 16px;
            padding: 32px;
            max-width: 680px;
            width: 100%;
            box-shadow: 0 12px 36px rgba(0,0,0,0.4);
        }}
        .header {{
            display: flex;
            align-items: center;
            gap: 16px;
            margin-bottom: 24px;
            border-bottom: 1px solid var(--border);
            padding-bottom: 16px;
        }}
        .header h1 {{
            margin: 0;
            font-size: 24px;
            color: var(--text-primary);
        }}
        .badge {{
            background: var(--badge-bg);
            color: var(--badge-fg);
            padding: 4px 10px;
            border-radius: 12px;
            font-size: 12px;
            font-family: monospace;
            font-weight: bold;
        }}
        p {{
            color: var(--text-secondary);
            line-height: 1.6;
            font-size: 15px;
            margin: 12px 0;
        }}
        .info-box {{
            background: var(--surface-high);
            border-left: 4px solid var(--primary);
            padding: 16px;
            border-radius: 8px;
            margin: 20px 0;
        }}
        .info-box h3 {{
            margin: 0 0 8px 0;
            font-size: 15px;
            color: var(--primary);
        }}
        code {{
            background: #090d12;
            padding: 3px 8px;
            border-radius: 6px;
            font-family: "JetBrains Mono", Consolas, monospace;
            color: #79c0ff;
            font-size: 13px;
        }}
        .btn-group {{
            display: flex;
            gap: 12px;
            margin-top: 24px;
            flex-wrap: wrap;
        }}
        .btn {{
            background: var(--primary);
            color: #0b131a;
            padding: 10px 20px;
            border-radius: 8px;
            text-decoration: none;
            font-weight: 600;
            font-size: 14px;
            transition: opacity 0.2s;
        }}
        .btn:hover {{
            opacity: 0.9;
        }}
        .btn-secondary {{
            background: var(--surface-high);
            color: var(--text-primary);
            border: 1px solid var(--border);
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="header">
            <div>
                <h1>Signals SDR <span class="badge">v{}</span></h1>
                <p style="margin: 4px 0 0 0;">Servidor Web LAN Activo & Operativo</p>
            </div>
        </div>

        <div class="info-box">
            <h3>Conexión Remota Establecida</h3>
            <p style="margin:0;">Este dispositivo está conectado exitosamente a la instancia anfitriona de <strong>Signals SDR</strong> a través de la red local (Wi-Fi).</p>
        </div>

        <p>El servidor web está escuchando conexiones en este puerto. Para cargar la interfaz gráfica completa en WebAssembly (WASM):</p>
        <ol style="color: var(--text-secondary); line-height: 1.8; font-size: 14px;">
            <li>Compile el cliente web con Trunk: <code>trunk build --release</code></li>
            <li>Coloque los archivos generados en la carpeta <code>dist/</code> o <code>web/dist/</code>.</li>
            <li>Recargue esta página para interactuar con la suite completa y controlar los transceptores SDR de forma inalámbrica.</li>
        </ol>

        <div class="btn-group">
            <a href="https://github.com/apt-tx/signals-sdr" target="_blank" class="btn">Repositorio GitHub</a>
            <a href="https://github.com/apt-tx/signals-sdr/blob/main/docs/ARCHITECTURE.md" target="_blank" class="btn btn-secondary">Documentación de Arquitectura</a>
        </div>
    </div>
</body>
</html>"#,
        env!("CARGO_PKG_VERSION")
    );

    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        html.len()
    );

    stream.write_all(header.as_bytes())?;
    stream.write_all(html.as_bytes())?;
    Ok(())
}

fn open_browser_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
