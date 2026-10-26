//! # Subsistema de Hardware SDR y Transmisión por Red
//!
//! Este módulo agrupa los controladores de emisión de radio definida por software (SDR)
//! tanto para transceptores físicos conectados por USB/Ethernet como para canales virtuales
//! y servidores de red didácticos.
//!
//! ## 1. Control de Hardware SDR Físico ([`worker`])
//! Proporciona una interfaz unificada sobre la biblioteca de abstracción de hardware SDR Seify:
//! - **Detección Automática:** Enumeración en caliente de dispositivos conectados (HackRF One,
//!   Nuand bladeRF, Ettus Research USRP, dispositivos SoapySDR o transceptor virtual Dummy).
//! - **Selector Jerárquico de 2 Etapas:** Selección primero del tipo de transceptor y luego de
//!   la instancia concreta mediante número de serie o dirección IP.
//! - **Transmisión Asíncrona en Bucle:** Hilo dedicado con doble búfer que envía muestras I/Q
//!   continuas hacia el frontend de radio sin saturar la memoria ni bloquear el hilo de interfaz gráfica.
//! - **Parámetros en Caliente:** Configuración interactiva de frecuencia portadora (Hz), ganancia de transmisión (dB),
//!   amplificador RF (+14 dB en HackRF), tasa de muestreo y repetición en bucle.
//!
//! ## 2. Transmisión por Red y Emulación RTL-TCP ([`network_tx`])
//! Permite transmitir las señales generadas a través de sockets TCP y UDP sin necesidad de
//! transceptores SDR de transmisión físicos:
//!
//! ### Servidor RTL-TCP Didáctico
//! Implementa el protocolo estándar de red de `rtl_tcp` en `127.0.0.1:1234`, permitiendo conectar
//! cualquier cliente SDR estándar de mercado (SDR#, SDR++, GQRX, WebSDR, GNU Radio):
//! - **Cabecera `RTL0`:** Envío inmediato del bloque de 12 bytes identificador con sintonizador tipo 5 (Rafael Micro R820T)
//!   y 29 pasos de ganancia disponibles.
//! - **Framing TCP Acumulativo:** Procesamiento robusto de comandos de 5 bytes sin riesgo de desalineación por fragmentación de paquetes.
//! - **Comandos de Ganancia Soportados:**
//!   - `0x01` (`SET_FREQUENCY`): Sintonización dinámica de portadora virtual.
//!   - `0x02` (`SET_SAMPLE_RATE`): Configuración de tasa de muestreo y sincronización automática del reloj de pacing.
//!   - `0x03` (`SET_GAIN_MODE`): Modo manual vs Tuner AGC.
//!   - `0x04` (`SET_GAIN`): Ganancia analógica directa en décimas de dB (0 a 496).
//!   - `0x0D` (`SET_TUNER_GAIN_BY_INDEX`): Selección de ganancia indexada en los 29 escalones oficiales del chip R820T (0.0 dB a 49.6 dB).
//! - **Modelo Físico de Atenuación y Dither ADC:** Escala logarítmica realista en dB ($10^{(\text{gain}-38)/20}$),
//!   saturación progresiva por encima del punto óptimo (38 dB) y dither continuo del ADC RTL2832U ($\sigma_{\text{adc}} = 0.014$)
//!   que garantiza un piso de ruido natural constante de ~ -75 dBFS (evitando la línea muerta plana a -200 dBFS).
//! - **Tasas de Muestreo Soportadas:** 15 opciones normalizadas cubriendo el 100% de SDR# y SDR++ (0.25 MSps a 3.2 MSps).
//!
//! ### Streaming GNU Radio UDP
//! Envía paquetes UDP continuos hacia bloques *UDP Source* de GNU Radio Companion con formatos configurables:
//! `Float32`, `Complex32` e `Int16`.

pub mod network_tx;
pub mod worker;

pub use network_tx::{
    GnuRadioFormat, GnuRadioProtocol, NetworkStreamStats, NetworkTxController,
};
pub use worker::{
    detect_connected_radios, detect_detailed_radios, DetectedDeviceInfo, SdrBackend,
    SdrDeviceOption, SdrHardwareType, SdrTransmissionController, TransmissionStatus, TxConfig,
};
