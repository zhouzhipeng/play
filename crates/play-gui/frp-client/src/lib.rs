use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use eframe::egui;
use tokio::sync::broadcast;

const WINDOW_TITLE: &str = "FRP Client";
const DEFAULT_CLIENT_CONFIG: &str = r#"# Minimal rathole client configuration.
# Adjust remote_addr, token, and local_addr for your environment.

[client]
remote_addr = "127.0.0.1:2333"
default_token = "change_this_token"

[client.services.demo_http]
local_addr = "127.0.0.1:3000"
"#;

enum ClientEvent {
    Stopped(String),
    Failed(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientState {
    Stopped,
    Running,
}

#[derive(Clone, Copy, Default)]
pub struct FrpClientOptions {
    pub auto_start: bool,
}

pub struct FrpClientApp {
    config_path: PathBuf,
    config_text: String,
    state: ClientState,
    last_status: String,
    event_rx: Receiver<ClientEvent>,
    event_tx: Sender<ClientEvent>,
    shutdown_tx: Option<broadcast::Sender<bool>>,
}

impl FrpClientApp {
    pub fn new() -> Self {
        Self::new_with_options(FrpClientOptions::default())
    }

    pub fn new_with_options(options: FrpClientOptions) -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        let config_path = default_config_path();

        let (config_text, last_status) = match ensure_config_file(&config_path)
            .and_then(|_| fs::read_to_string(&config_path).context("read FRP client config failed"))
        {
            Ok(config_text) => (
                config_text,
                format!("Using config {}", config_path.display()),
            ),
            Err(error) => (
                DEFAULT_CLIENT_CONFIG.to_string(),
                format!("Failed to initialize config: {error:#}"),
            ),
        };

        let mut app = Self {
            config_path,
            config_text,
            state: ClientState::Stopped,
            last_status,
            event_rx,
            event_tx,
            shutdown_tx: None,
        };

        if options.auto_start {
            app.start_client();
        }

        app
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.poll_events();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("FRP Client");
            ui.label("Desktop client for editing and running a rathole FRP client config.");
            ui.add_space(8.0);

            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(format!("Config: {}", self.config_path.display()));
                ui.label(match self.state {
                    ClientState::Stopped => "Status: stopped",
                    ClientState::Running => "Status: running",
                });

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        self.save_config();
                    }
                    if ui.button("Reload").clicked() {
                        self.reload_config();
                    }
                    if ui.button("Load Example").clicked() {
                        self.load_example();
                    }
                    if ui.button("Open Config Dir").clicked() {
                        self.open_config_dir();
                    }
                });

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.state == ClientState::Stopped, egui::Button::new("Start"))
                        .clicked()
                    {
                        self.start_client();
                    }
                    if ui
                        .add_enabled(self.state == ClientState::Running, egui::Button::new("Stop"))
                        .clicked()
                    {
                        self.stop_client();
                    }
                });
            });

            ui.add_space(10.0);
            ui.label("Configuration");
            ui.add(
                egui::TextEdit::multiline(&mut self.config_text)
                    .desired_rows(24)
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );

            ui.add_space(10.0);
            ui.separator();
            ui.label(&self.last_status);
        });
    }

    fn poll_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                ClientEvent::Stopped(message) | ClientEvent::Failed(message) => {
                    self.state = ClientState::Stopped;
                    self.shutdown_tx = None;
                    self.last_status = message;
                }
            }
        }
    }

    fn save_config(&mut self) {
        match save_config_file(&self.config_path, &self.config_text) {
            Ok(()) => {
                self.last_status = format!("Saved {}", self.config_path.display());
            }
            Err(error) => {
                self.last_status = format!("Failed to save config: {error:#}");
            }
        }
    }

    fn reload_config(&mut self) {
        match fs::read_to_string(&self.config_path) {
            Ok(content) => {
                self.config_text = content;
                self.last_status = format!("Reloaded {}", self.config_path.display());
            }
            Err(error) => {
                self.last_status = format!("Failed to reload config: {error}");
            }
        }
    }

    fn load_example(&mut self) {
        self.config_text = DEFAULT_CLIENT_CONFIG.to_string();
        self.last_status = "Loaded example FRP client config into the editor".to_string();
    }

    fn start_client(&mut self) {
        if self.state == ClientState::Running {
            self.last_status = "FRP client is already running".to_string();
            return;
        }

        if let Err(error) = save_config_file(&self.config_path, &self.config_text) {
            self.last_status = format!("Failed to save config before start: {error:#}");
            return;
        }

        if let Err(error) = validate_config(&self.config_path) {
            self.last_status = format!("Invalid FRP client config: {error:#}");
            return;
        }

        let (shutdown_tx, shutdown_rx) = broadcast::channel(4);
        let config_path = self.config_path.clone();
        let event_tx = self.event_tx.clone();

        thread::spawn(move || {
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = event_tx.send(ClientEvent::Failed(format!(
                        "Failed to create tokio runtime: {error}"
                    )));
                    return;
                }
            };

            let args = rathole::Cli {
                config_path: Some(config_path.clone()),
                server: false,
                client: true,
                genkey: None,
            };

            let result = runtime.block_on(async move { rathole::run(args, shutdown_rx).await });
            let message = match result {
                Ok(()) => ClientEvent::Stopped("FRP client stopped".to_string()),
                Err(error) => ClientEvent::Failed(format!("FRP client exited: {error:#}")),
            };
            let _ = event_tx.send(message);
        });

        self.shutdown_tx = Some(shutdown_tx);
        self.state = ClientState::Running;
        self.last_status = format!("Started FRP client with {}", self.config_path.display());
    }

    fn stop_client(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(true);
            self.last_status = "Stopping FRP client...".to_string();
        } else {
            self.last_status = "FRP client is not running".to_string();
        }
        self.state = ClientState::Stopped;
    }

    fn open_config_dir(&mut self) {
        let Some(parent) = self.config_path.parent() else {
            self.last_status = "Config directory is not available".to_string();
            return;
        };

        match open_path(parent) {
            Ok(()) => {
                self.last_status = format!("Opened {}", parent.display());
            }
            Err(error) => {
                self.last_status = format!("Failed to open config directory: {error:#}");
            }
        }
    }
}

impl Drop for FrpClientApp {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(true);
        }
    }
}

impl eframe::App for FrpClientApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let _ = frame;
        self.show(ctx);
    }
}

pub fn run() -> eframe::Result {
    run_with_options(FrpClientOptions::default())
}

pub fn run_with_options(launch_options: FrpClientOptions) -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png"))
        .expect("invalid icon data");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([860.0, 760.0])
            .with_min_inner_size([760.0, 640.0])
            .with_title(WINDOW_TITLE)
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        WINDOW_TITLE,
        native_options,
        Box::new(move |_cc| Ok(Box::new(FrpClientApp::new_with_options(launch_options)))),
    )
}

fn default_config_path() -> PathBuf {
    ProjectDirs::from("com", "zhouzhipeng", "play")
        .map(|dirs| dirs.data_dir().join("frp").join("client.toml"))
        .unwrap_or_else(|| PathBuf::from("frp-client.toml"))
}

fn ensure_config_file(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create config directory {}", parent.display()))?;
    }

    fs::write(path, DEFAULT_CLIENT_CONFIG)
        .with_context(|| format!("write FRP client config {}", path.display()))?;
    Ok(())
}

fn save_config_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create config directory {}", parent.display()))?;
    }

    fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn validate_config(path: &Path) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("create tokio runtime failed")?;
    runtime
        .block_on(async { rathole::Config::from_file(path).await })
        .with_context(|| format!("validate {}", path.display()))?;
    Ok(())
}

fn open_path(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");

    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer");

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");

    command
        .arg(path)
        .spawn()
        .with_context(|| format!("open {}", path.display()))?;
    Ok(())
}
