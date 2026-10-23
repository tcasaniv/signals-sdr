# Guía de Contribución a Signals SDR

¡Gracias por tu interés en contribuir a **Signals SDR**! Este proyecto es de código abierto bajo la licencia [GNU General Public License v3.0 (GPL-3.0-or-later)](LICENSE), enfocado en la educación, experimentación y docencia en radio definida por software (SDR), telecomunicaciones satelitales y procesamiento digital de señales (DSP).

Para mantener la calidad, seguridad, reproducibilidad y coherencia visual del proyecto, todos los colaboradores deben seguir las siguientes directrices.

---

## 1. Política de Responsabilidad Humana y Uso de Inteligencia Artificial

> [!IMPORTANT]
> **El contribuidor humano que firma, abre y envía un Pull Request (PR) es el único y exclusivo responsable legal, técnico y ético de cada línea de código, documentación o recurso introducido en el repositorio.**

En Signals SDR reconocemos el enorme valor de las herramientas modernas de Inteligencia Artificial (asistentes de código, modelos de lenguaje LLM y agentes de programación como Claude, ChatGPT, Gemini, GitHub Copilot, Cursor, Antigravity, etc.). Su uso está plenamente permitido y fomentado para acelerar el desarrollo.

Sin embargo, aplican las siguientes reglas obligatorias:

1. **El Humano en el Bucle (*Human-in-the-Loop*):**
   - Una IA no tiene identidad jurídica ni puede asumir responsabilidad por el software que genera.
   - En ningún caso se aceptará como justificación ante errores, regresiones, alucinaciones o vulnerabilidades la frase *"la IA generó este código"*.
   - El autor del PR debe haber leído, comprendido en su totalidad, validado y probado el código antes de someterlo a revisión.

2. **Verificación Estricta contra Alucinaciones:**
   - La IA puede sugerir APIs inexistentes, banderas de compilación obsoletas o modelos matemáticos de modulación incorrectos. Es deber del contribuidor comprobar contra la documentación oficial de Rust, `egui` y los estándares satelitales (NOAA, EUMETSAT, CCSDS, ITU).

3. **Integridad de Licencias y Propiedad Intelectual:**
   - El contribuidor humano certifica que el código no infringe derechos de autor de terceros ni incorpora fragmentos de código con licencias incompatibles con GPLv3.

4. **Cumplimiento de Normativas de Radiofrecuencia (RF):**
   - Signals SDR interactúa con hardware de transmisión física de radio. Cualquier módulo que configure frecuencias, modulaciones o etapas de amplificación de potencia debe ser rigurosamente supervisado por el contribuidor para evitar emisiones en bandas prohibidas o de socorro (ej. 121.5 MHz, 406 MHz).

---

## 2. Cómo Empezar

### Requisitos de Desarrollo
- **Rust Toolchain:** Versión estable 1.80 o superior (`rustup default stable`).
- **Target WebAssembly (opcional pero recomendado):** `rustup target add wasm32-unknown-unknown`.
- **Cargo-clippy y Rustfmt:** `rustup component add clippy rustfmt`.
- **Git** configurado con tu nombre y correo electrónico real.

### Flujo de Trabajo (Fork & Branch)
1. Haz un **Fork** del repositorio en GitHub.
2. Clona tu fork localmente:
   ```bash
   git clone https://github.com/tcasaniv/signals-sdr.git
   cd signals-sdr
   ```
3. Crea una rama de trabajo con un nombre descriptivo:
   ```bash
   git checkout -b feature/nuevo-protocolo-aprs
   # o para correcciones:
   git checkout -b fix/cascada-zoom-touch
   ```

---

## 3. Convenciones de Commits y Mensajes

Seguimos la convención de [Conventional Commits](https://www.conventionalcommits.org/):

```
<tipo>(<ámbito opcional>): <descripción concisa en imperativo>

[cuerpo explicativo opcional]

[referencias a issues opcionales]
```

### Tipos permitidos:
- `feat`: Nueva característica o funcionalidad (ej. `feat(pipelines): añadir modulador APRS AX.25`).
- `fix`: Corrección de un fallo o error (ej. `fix(ui): restaurar visibilidad de catálogo al buscar`).
- `docs`: Modificaciones exclusivamente en documentación (ej. `docs: actualizar ROADMAP y guía web`).
- `ui`: Ajustes estéticos, de diseño o temas visuales (ej. `ui: alinear píldoras M3 en navegación móvil`).
- `refactor`: Refactorización interna de código sin cambios funcionales ni de API.
- `perf`: Mejoras de rendimiento en algoritmos DSP o renderizado.
- `test`: Añadir o corregir pruebas unitarias o de integración.
- `chore`: Mantenimiento de dependencias, scripts de compilación o configuración de CI.

---

## 4. Estándares de Código y Calidad

Antes de enviar un Pull Request, debes ejecutar y aprobar localmente los siguientes comandos:

### 1. Formato de Código
```bash
cargo fmt --all -- --check
```
*Si hay discrepancias de estilo, corrígelas automáticamente con `cargo fmt --all`.*

### 2. Análisis Estático (Clippy)
```bash
cargo clippy --all-targets -- -D warnings
```
*El código no debe contener advertencias de clippy ni advertencias de compilador desatendidas.*

### 3. Suite Completa de Pruebas
```bash
cargo test
```
*Todas las pruebas unitarias y de integración deben pasar con 0 fallos.*

---

## 5. Lineamientos de Diseño de UI/UX

Signals SDR combina **Material Design 3 (Google)** con las **Human Interface Guidelines (Apple)** y el espíritu de Fluent UI:

1. **Prohibido el uso de Emojis en la Interfaz Gráfica:**
   - La aplicación proyecta una estética profesional de instrumentación de laboratorio.
   - Todo icono debe provenir de los símbolos vectoriales de **Google Material Symbols** integrados mediante `egui_icons` o trazados limpios con `ui.painter()`.
2. **Diseño Adaptativo y Responsivo Obligatorio:**
   - Cualquier pantalla, subpestaña o widget nuevo **debe** probarse en ambos modos:
     - **Modo Escritorio (ancho $\ge$ 720 px):** Aprovechando el Navigation Rail lateral, el catálogo flexible y paneles multitarea.
     - **Modo Móvil (ancho $<$ 720 px):** Aprovechando la barra superior con botón de retorno, la barra inferior de navegación y el flujo Master-Detail.
3. **Flujo Didáctico de 4 Fases por Pipeline:**
   Todo pipeline de transmisión debe organizarse en las 4 fases didácticas del proyecto:
   1. *Elegir fuente* (Carga de imagen, audio o datos I/Q).
   2. *Codificar / Modular* (Fases intermedias explicadas paso a paso con inspección visual).
   3. *Efectos de canal* (Simulación de AWGN, desplazamiento Doppler y atenuación).
   4. *Transmisión* (Hardware SDR vs. exportación a archivo I/Q).
4. **Rendimiento a 60 FPS:**
   - No bloquees el hilo principal de renderizado de `egui`. Los procesamientos de DSP pesados, la codificación de tramas grandes o el acceso a disco/hardware deben ejecutarse en hilos secundarios con canales de comunicación (`mpsc` o `crossbeam`).

---

## 6. Proceso de Envío del Pull Request (PR)

1. Sube tu rama a tu fork:
   ```bash
   git push origin feature/mi-funcionalidad
   ```
2. Abre el Pull Request en GitHub contra la rama `main` de `apt-tx/signals-sdr`.
3. Completa la plantilla de PR describiendo:
   - **Qué problema resuelve o qué función agrega.**
   - **Pruebas manuales y automáticas realizadas.**
   - **Capturas de pantalla o grabaciones (si altera la interfaz de usuario).**
   - **Declaración explícita de que has revisado y validado el código propuesto.**
4. Participa activamente en la revisión de código respondiendo a comentarios o sugerencias de los mantenedores.

---

¡Gracias por ayudar a democratizar la enseñanza y experimentación de las comunicaciones por radio y satelitales en todo el mundo!
