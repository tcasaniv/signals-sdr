//! # Capa de Presentación e Interfaz Gráfica (UI)
//!
//! Este módulo implementa la interfaz de usuario interactiva y responsiva de Signals SDR
//! construida con [`egui`] y [`eframe`], adhiriéndose a las directrices de diseño de
//! **Google Material Design 3** y **Apple Human Interface Guidelines (HIG)**.
//!
//! ## Principios de Diseño
//!
//! - **Diseño Adaptativo y Responsivo:**
//!   - **Escritorio ($\ge 720\text{ px}$):** Barra lateral estrecha (*Navigation Rail*, 72 px),
//!     panel flexible redimensionable de transmisores (220 a 450 px), y tarjetas de submenú
//!     de configuración a ancho completo (`ui.available_width()`) con bordes redondeados y chevrons vectoriales.
//!   - **Móvil ($< 720\text{ px}$):** Barra superior táctil con botón de retorno en flujo *Master-Detail*
//!     y barra de navegación inferior (*Bottom Bar*) con indicadores de píldora activa.
//! - **Iconografía Vectorial:** Ausencia absoluta de emojis en etiquetas y botones; empleo exclusivo
//!   de trazados vectoriales limpios y Google Material Symbols (`egui_icons` / [`crate::icons`]).
//! - **Sistemas Tonal y de Temas:** Paletas cromáticas para Modo Oscuro y Modo Claro con contraste
//!   ergonómico para uso prolongado en estaciones de radio y laboratorios.
//!
//! ## Componentes e Instrumentación de Laboratorio
//!
//! - [`virtual_sink`]: **Instrumento Virtual de Laboratorio**, renderizado como panel flexible integrado
//!   en el área de trabajo derecha (`CentralPanel`) con botón de cierre `[X]` en la cabecera. Incluye 5 pestañas:
//!   - **Espectro RF (FFT):** Analizador espectral de magnitud en dB con retención de picos máximos y mínimos.
//!   - **Cascada (Waterfall):** Representación bidimensional tiempo-frecuencia en falso color con 3 resoluciones.
//!   - **Dominio del Tiempo:** Osciloscopio en cuadratura para inspeccionar las señales $I(t)$, $Q(t)$ y su envolvente $|IQ|$.
//!   - **Constelación I/Q:** Diagrama de plano complejo para visualizar constelaciones digitales (OQPSK, BPSK, etc.).
//!   - **Métricas de Señal:** Telemetría en vivo con cálculo de potencia pico (dBFS), RMS, factor de cresta (PAPR),
//!     offset de continua $I/Q$ y aislamiento de portadora.
//! - [`spectrogram`]: Generador de cascada espectral con zoom y desplazamiento temporal.
//! - [`spectrum`]: Oscilograma interactivo en tiempo real con exportación de capturas PNG.
//! - [`audio_player`]: Reproductor de audio acústico integrado con decodificación inmediata de subportadoras y tonos.
//! - [`channel_effects`]: Vista pedagógica del Paso 3 para previsualizar y simular el canal físico sin truncamientos.
//! - [`transmission`]: Panel de configuración del Paso 4 (Hardware SDR, Servidor RTL-TCP, Streaming UDP y Formatos I/Q).

pub mod audio_player;
pub mod channel_effects;
pub mod components;
pub mod desktop_nav;
pub mod image_modal;
pub mod mobile_nav;
pub mod output_files_modal;
pub mod spectrogram;
pub mod spectrum;
pub mod transmission;
pub mod virtual_sink;
