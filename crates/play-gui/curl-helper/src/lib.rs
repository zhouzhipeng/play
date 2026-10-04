pub mod app;
mod curl_parser;
mod executor;
mod storage;

pub use app::CurlHelperApp;

const WINDOW_TITLE: &str = "Curl Helper";

pub fn run() -> eframe::Result {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).expect("Bad icon");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title(WINDOW_TITLE)
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(CurlHelperApp::new(&cc.egui_ctx)))),
    )
}
