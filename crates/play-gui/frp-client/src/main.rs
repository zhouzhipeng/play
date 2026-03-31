fn main() -> eframe::Result {
    let auto_start = std::env::args().skip(1).any(|arg| arg == "--auto-start");
    frp_client::run_with_options(frp_client::FrpClientOptions { auto_start })
}
