use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use eframe::egui;

include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"));

const WINDOW_TITLE: &str = "Play GUI";

struct PlayGuiApp {
    filter: String,
    last_status: Option<String>,
}

impl PlayGuiApp {
    fn new() -> Self {
        Self {
            filter: String::new(),
            last_status: None,
        }
    }

    fn matches_filter(&self, package_name: &str, display_name: &str, relative_dir: &str, description: &str) -> bool {
        let query = self.filter.trim().to_ascii_lowercase();
        if query.is_empty() {
            return true;
        }

        [package_name, display_name, relative_dir, description]
            .into_iter()
            .any(|value| value.to_ascii_lowercase().contains(&query))
    }

    fn launch_tool(&mut self, package_name: &str, display_name: &str) {
        match spawn_tool_process(package_name) {
            Ok(()) => {
                self.last_status = Some(format!("Launching {display_name}"));
            }
            Err(error) => {
                self.last_status = Some(format!("Failed to launch {display_name}: {error}"));
            }
        }
    }
}

impl eframe::App for PlayGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Play Toolbox");
            ui.label("Pure egui toolbox entrance for local desktop tools.");
            ui.add_space(10.0);

            render_toolbar(ui, self);
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut visible_count = 0usize;

                for (package_name, display_name, relative_dir, description) in DISCOVERED_TOOLS {
                    if !self.matches_filter(package_name, display_name, relative_dir, description) {
                        continue;
                    }

                    visible_count += 1;
                    render_tool_section(
                        ui,
                        self,
                        package_name,
                        display_name,
                        relative_dir,
                        description,
                    );
                    ui.add_space(10.0);
                }

                if visible_count == 0 {
                    ui.label("No tools matched the current filter.");
                }
            });

            if let Some(status) = &self.last_status {
                ui.add_space(12.0);
                ui.separator();
                ui.label(status);
            }
        });
    }
}

fn render_toolbar(ui: &mut egui::Ui, app: &mut PlayGuiApp) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Filter");
            ui.add(
                egui::TextEdit::singleline(&mut app.filter)
                    .desired_width(f32::INFINITY)
                    .hint_text("Search tools by name or package"),
            );
        });
    });
}

fn render_tool_section(
    ui: &mut egui::Ui,
    app: &mut PlayGuiApp,
    package_name: &str,
    display_name: &str,
    relative_dir: &str,
    description: &str,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading(display_name);
        ui.label(description);
        ui.add_space(6.0);
        ui.monospace(format!("package: {package_name}"));
        ui.monospace(format!("path: crates/play-gui/{relative_dir}"));
        ui.add_space(8.0);

        if ui.button(format!("Open {display_name}")).clicked() {
            app.launch_tool(package_name, display_name);
        }
    });
}

fn spawn_tool_process(package_name: &str) -> Result<()> {
    if let Some(binary) = find_sibling_binary(package_name) {
        Command::new(binary)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn binary for {package_name}"))?;
        return Ok(());
    }

    let mut command = Command::new("cargo");
    command.arg("run").arg("-p").arg(package_name);
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }

    command
        .current_dir(workspace_root())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn cargo run for {package_name}"))?;

    Ok(())
}

fn find_sibling_binary(package_name: &str) -> Option<PathBuf> {
    let executable_name = if cfg!(target_os = "windows") {
        format!("{package_name}.exe")
    } else {
        package_name.to_string()
    };

    let current_exe = env::current_exe().ok()?;
    let bin_dir = current_exe.parent()?;
    let candidate = bin_dir.join(executable_name);
    candidate.exists().then_some(candidate)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("workspace root missing")
}

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("invalid icon data");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 560.0])
            .with_min_inner_size([680.0, 480.0])
            .with_title(WINDOW_TITLE)
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|_cc| Ok(Box::new(PlayGuiApp::new()))),
    )
}
