fn main() {
    #[cfg(target_os = "windows")]
    {
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if target_env == "msvc" {
            println!("cargo:rustc-link-arg=/DELAYLOAD:SoapySDR.dll");
            println!("cargo:rustc-link-arg=delayimp.lib");
        }

        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Signals SDR");
        res.set("FileDescription", "Signals SDR - Generador Didáctico de Señales de Radio");
        res.set("LegalCopyright", "Copyright (c) 2026 Signals SDR Contributors");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile windows resource: {}", e);
        }
    }
}
