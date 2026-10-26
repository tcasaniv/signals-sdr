//! # Formatos de Exportación e Importación de Señales I/Q
//!
//! Este módulo contiene los codificadores y decodificadores de archivos de señal I/Q
//! en banda base, permitiendo guardar las transmisiones sintetizadas en disco para su
//! posterior análisis o transmitirlas mediante utilidades de línea de comandos estándar.
//!
//! ## Formatos Soportados ([`IqExportFormat`])
//!
//! - **WAV I/Q (PCM 16 bits estéreo):**
//!   Formato de audio estándar donde el canal izquierdo almacena la componente en fase ($I$)
//!   y el canal derecho almacena la componente en cuadratura ($Q$). Es directamente compatible
//!   con SDR#, SDR++, GQRX, HDSDR y WebSDR.
//!
//! - **RAW SC16_Q11:**
//!   Muestras enteras con signo de 16 bits en punto fijo con 11 bits fraccionarios,
//!   formato nativo requerido por la herramienta `bladeRF-cli` para transmisión con transceptores Nuand bladeRF.
//!
//! - **RAW SC16 Completo:**
//!   Muestras de 16 bits enteras con signo escaladas a rango completo ($\pm 32767$).
//!
//! - **RAW con signo int8:**
//!   Muestras entrelazadas $I/Q$ de 8 bits con signo ($\pm 127$), formato nativo utilizado
//!   por la utilidad oficial `hackrf_transfer` de Great Scott Gadgets para el HackRF One.
//!
//! - **RAW float32:**
//!   Muestras complejas en coma flotante de 32 bits en formato IEEE 754 little-endian,
//!   formato estándar de intercambio en GNU Radio Companion, inspectrum y especificación SigMF.
//!
//! - **WAV Audio Mono:**
//!   Audio analógico de un solo canal para escuchar el sonido resultante en reproductores
//!   multimedia convencionales.

pub mod formats;
pub use formats::{export_iq_file, export_iq_to_memory, read_iq_file, read_iq_from_memory, IqExportFormat};
