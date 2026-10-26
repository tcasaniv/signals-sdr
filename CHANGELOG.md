# Changelog

Todos los cambios notables en este proyecto serán documentados en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/).

---

## [0.4.2] - 2026-10-25

### Corregido (Fixes)
- **Piso de Ruido RTL-TCP a Bajas Ganancias**: Se incorporó el modelo de ruido de cuantización y térmico intrínseco del ADC RTL2832U de 8 bits ($\sigma_{\text{adc}} = 0.014$, equivalente a $\pm 1.5$ a $2$ LSBs continuos). Se elimina la línea muerta plana a -200 dBFS / -135 dBFS en SDR# y SDR++ cuando la ganancia se ajusta a 0.0 dB, manteniendo un piso de césped de ruido realista constante de ~ -75 dBFS a -70 dBFS.
- **Reloj de Transmisión RTL-TCP y Pacing**: La duración de los paquetes y el reloj de ritmo se sincronizan dinámicamente con la tasa de muestreo solicitada por el cliente SDR remoto mediante el comando `0x02` (`SET_SAMPLE_RATE`).
- **Panel Flexible de Ajustes en Escritorio**: Las tarjetas de los submenús de configuración en el panel secundario izquierdo ahora ocupan el 100% del ancho disponible (`ui.available_width()`), incorporando bordes redondeados M3 (10 px), padding simétrico y flecha/chevron derecho (`icons::ICON_CHEVRON_RIGHT`), manteniendo paridad visual con la experiencia móvil.

### Añadido (Features)
- **Tasas de Muestreo Estándar RTL-SDR**: Menú selector en el panel RTL-TCP con las tasas de muestreo estándar de SDR# y SDR++ (`0.25 MSps`, `0.90 MSps`, `1.024 MSps`, `1.4 MSps`, `1.8 MSps`, `1.92 MSps`, `2.048 MSps`, `2.4 MSps`, `2.56 MSps`, `3.2 MSps`) y botón de remuestreo automático con un solo clic.
- **Instrumento Virtual como Panel Flexible Integrado**: Se reemplazó la ventana modal flotante (`egui::Window`) por un panel flexible que se dibuja directamente en el área de trabajo derecha (`CentralPanel`). Ocupa el 100% del ancho y alto disponible sin expandirse fuera de la ventana ni tapar los menús laterales, incorporando una barra superior con selector de pestañas y botón de cierre `[X]` (`icons::ICON_CLOSE`) para regresar instantáneamente a la configuración del transmisor.

---

## [0.4.1] - 2026-10-25

### Añadido (Features)
- **Servidor RTL-TCP Didáctico**: Emulación del dongle RTL-SDR sobre puerto TCP 1234 con cabecera `RTL0` y soporte para SDR#, GQRX, SDR++, WebSDR y GNU Radio.
- **Soporte de Ganancia por Índice `0x0D` (`SET_TUNER_GAIN_BY_INDEX`)**: Implementación de los 29 pasos de ganancia del sintonizador R820T/R820T2 (0.0 dB a 49.6 dB) con framing TCP acumulativo libre de desalineación por fragmentación.
- **Simulación Logarítmica de Ganancia y Saturación**: Atenuación en decibelios reales ($10^{(\text{gain} - 38)/20}$), punto óptimo a 38 dB (0 dBFS nominal) y elevación exponencial del ruido térmico al exceder 38 dB.
- **Control de Bucle y Re-transmisión**: Toggle de repetición continua con emisión de piso de ruido AWGN continuo al terminar la señal y botón `Re-transmitir Señal` en caliente sin desconectar al cliente SDR.
- **Atenuación Docente Dinámica**: Deslizador de potencia TX de 0.0 dB a -60.0 dB con sincronización atómica en tiempo real.
- **Instrumento Virtual**:
  - Trazas independientes de Max Peak Hold (amarillo) y Min Peak Hold (magenta) con botón de reset.
  - Escalas dinámicas de los ejes X e Y con sliders de rango en dB y marcas de graduación adaptativas.
  - Opciones de FFT ampliadas hasta 16384 puntos (512 a 16384).
  - Resolución de cascada (waterfall) configurable desde 64 columnas hasta resolución nativa de la FFT.
  - Constelación I/Q calibrada con círculos concéntricos de referencia ($r=0.5, 1.0$) y marcas en $\pm 1.0$ y $\pm 0.5$ con zoom.
- **Navegación Móvil y Ajustes**: Pestaña independiente de "Apariencia y Tema" (Modo Claro/Oscuro), menú móvil de Ajustes como lista vertical de tarjetas y navegación a pantalla completa con botón flecha atrás.
- **Apertura de Enlaces en Navegador Nativo**: Lanzamiento confiable en Windows con `rundll32 url.dll,FileProtocolHandler`.
- **Visor de Imágenes Responsivo**: Encabezado en dos filas para anchos de pantalla móvil (< 550 px).

---

## [0.4.0] - 2026-10-24

### Añadido (Features)
- **Suite Integral de Transmisores y Moduladores**: NOAA APT, Meteor LRPT, ISS SSTV, Generic SSTV, Generic FM, Generic AM y Generic I/Q.
- **Streaming de Red GNU Radio**: Soporte TCP Server y UDP Sender en formatos Complex Float32 (`fc32`) y Complex Int16 (`sc16`).
- **Remuestreo Polifásico y DSP**: Remuestreador de muestras I/Q con interpolación cúbica Hermite.
- **Exportación Multi-formato**: Formatos WAV IQ 16-bit, SC16 Q11 (BladeRF), SC16 Full, Int8 (HackRF) y Float32.
