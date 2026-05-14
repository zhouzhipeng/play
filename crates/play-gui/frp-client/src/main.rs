#![cfg_attr(all(target_os = "windows", feature = "gui"), windows_subsystem = "windows")]

#[cfg(feature = "gui")]
fn main() -> eframe::Result {
    let auto_start = std::env::args().skip(1).any(|arg| arg == "--auto-start");
    frp_client::run_with_options(frp_client::FrpClientOptions { auto_start })
}

#[cfg(not(feature = "gui"))]
fn main() {
    use std::path::PathBuf;

    let args: Vec<String> = std::env::args().collect();
    let mut config_path: Option<PathBuf> = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" | "--config" => match iter.next() {
                Some(v) => config_path = Some(PathBuf::from(v)),
                None => {
                    eprintln!("error: --config requires a path argument");
                    std::process::exit(2);
                }
            },
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-V" | "--version" => {
                println!("frp-client {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            // Accepted for parity with the GUI build; harmless in headless mode.
            "--auto-start" => {}
            other => {
                eprintln!("error: unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }

    let path = config_path.unwrap_or_else(frp_client::default_config_path);
    eprintln!("frp-client: using config {}", path.display());

    match frp_client::run_headless(&path) {
        Ok(()) => {
            eprintln!("frp-client: stopped");
        }
        Err(err) => {
            eprintln!("frp-client: {err:#}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "gui"))]
fn print_help() {
    println!(
        "frp-client {} - headless rathole FRP client\n\
\n\
USAGE:\n    frp-client [OPTIONS]\n\
\n\
OPTIONS:\n\
    -c, --config <PATH>    Path to the rathole client TOML config\n\
                           (default: /etc/frp-client.toml on Unix)\n\
    -h, --help             Print this help\n\
    -V, --version          Print version\n\
\n\
The client runs until it receives SIGINT or SIGTERM (Ctrl+C).",
        env!("CARGO_PKG_VERSION")
    );
}
