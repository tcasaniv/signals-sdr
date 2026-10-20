#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use signals_sdr::SignalsSdrApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 780.0])
        .with_min_inner_size([380.0, 500.0])
        .with_title("Signals SDR - Generador Didáctico de Señales de Radio");

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon_256.png")) {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Signals SDR",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            egui_icons::initialize(&cc.egui_ctx);
            Ok(Box::new(SignalsSdrApp::default()))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Para plataforma WebAssembly en localhost o servidor web
    use eframe::wasm_bindgen::JsCast as _;
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("Sin ventana global")
            .document()
            .expect("Sin documento");

        let canvas = document
            .get_element_by_id("signals_sdr_canvas")
            .expect("No se encontró el elemento canvas signals_sdr_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Elemento no es un Canvas");

        let runner = eframe::WebRunner::new();
        runner
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    egui_extras::install_image_loaders(&cc.egui_ctx);
                    egui_icons::initialize(&cc.egui_ctx);
                    Ok(Box::new(SignalsSdrApp::default()))
                }),
            )
            .await
            .expect("Error al iniciar eframe en WebAssembly");
    });
}
