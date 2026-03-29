mod app;
mod curl_parser;
mod executor;
mod storage;

use eframe::egui;

fn main() -> eframe::Result {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).expect("Bad icon");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("Curl Helper")
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "Curl Helper",
        options,
        Box::new(|cc| Ok(Box::new(app::CurlHelperApp::new(cc)))),
    )
}
