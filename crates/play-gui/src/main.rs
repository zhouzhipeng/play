mod settings;
mod startup;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, ViewportBuilder, ViewportCommand, ViewportId};
use settings::PlayGuiSettings;

const WINDOW_TITLE: &str = "Play GUI";

const TOOL_DEFINITIONS: &[ToolDefinition] = &[
    ToolDefinition {
        kind: ToolKind::CurlHelper,
        package_name: "curl-helper",
        display_name: "Curl Helper",
        description: "Manage, organize, and run curl commands in a desktop client",
    },
    ToolDefinition {
        kind: ToolKind::FrpClient,
        package_name: "frp-client",
        display_name: "FRP Client",
        description: "Edit and run a rathole-based FRP client from the Play toolbox",
    },
];

#[derive(Clone, Copy)]
struct ToolDefinition {
    kind: ToolKind,
    package_name: &'static str,
    display_name: &'static str,
    description: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolKind {
    CurlHelper,
    FrpClient,
}

impl ToolKind {
    fn display_name(self) -> &'static str {
        tool_definition(self).display_name
    }

    fn package_name(self) -> &'static str {
        tool_definition(self).package_name
    }

    fn from_package_name(package_name: &str) -> Option<Self> {
        TOOL_DEFINITIONS
            .iter()
            .find(|tool| tool.package_name == package_name)
            .map(|tool| tool.kind)
    }
}

enum ToolRuntime {
    CurlHelper(curl_helper::CurlHelperApp),
    FrpClient(frp_client::FrpClientApp),
}

impl ToolRuntime {
    fn open(kind: ToolKind, ctx: &egui::Context) -> Self {
        match kind {
            ToolKind::CurlHelper => Self::CurlHelper(curl_helper::CurlHelperApp::new(ctx)),
            ToolKind::FrpClient => Self::FrpClient(frp_client::FrpClientApp::new()),
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Self::CurlHelper(_) => ToolKind::CurlHelper.display_name(),
            Self::FrpClient(_) => ToolKind::FrpClient.display_name(),
        }
    }

    fn viewport_builder(&self) -> ViewportBuilder {
        match self {
            Self::CurlHelper(_) => ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_min_inner_size([800.0, 500.0])
                .with_title(self.display_name()),
            Self::FrpClient(_) => ViewportBuilder::default()
                .with_inner_size([860.0, 760.0])
                .with_min_inner_size([760.0, 640.0])
                .with_title(self.display_name()),
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        match self {
            Self::CurlHelper(app) => app.show(ctx),
            Self::FrpClient(app) => app.show(ctx),
        }
    }
}

struct ToolWindow {
    id: u64,
    runtime: ToolRuntime,
    open: bool,
}

impl ToolWindow {
    fn new(id: u64, runtime: ToolRuntime) -> Self {
        Self {
            id,
            runtime,
            open: true,
        }
    }

    fn viewport_id(&self) -> ViewportId {
        ViewportId::from_hash_of(("play-gui-tool", self.id))
    }
}

struct PlayGuiApp {
    filter: String,
    next_tool_id: u64,
    open_tools: Vec<Arc<Mutex<ToolWindow>>>,
    settings: PlayGuiSettings,
    settings_path: PathBuf,
    settings_status: String,
    startup: StartupPreference,
}

impl PlayGuiApp {
    fn new(ctx: &egui::Context) -> Self {
        let (mut settings, settings_path, settings_notice) = PlayGuiSettings::load_or_default();
        let startup = StartupPreference::load();
        let mut app = Self {
            filter: String::new(),
            next_tool_id: 1,
            open_tools: Vec::new(),
            settings,
            settings_path,
            settings_status: settings_notice.unwrap_or_default(),
            startup,
        };

        if let Some(package_name) = app.settings.default_tool_package.clone() {
            if let Some(kind) = ToolKind::from_package_name(&package_name) {
                app.open_tool(kind, ctx);
            } else {
                app.settings.default_tool_package = None;
                let message = format!("Ignored unknown default tool `{package_name}`.");
                app.settings_status = if let Err(error) = app.settings.save(&app.settings_path) {
                    format!("{message} Failed to update preferences: {error:#}")
                } else {
                    message
                };
            }
        }

        app
    }

    fn matches_filter(&self, package_name: &str, display_name: &str, description: &str) -> bool {
        let query = self.filter.trim().to_ascii_lowercase();
        if query.is_empty() {
            return true;
        }

        [package_name, display_name, description]
            .into_iter()
            .any(|value| value.to_ascii_lowercase().contains(&query))
    }

    fn open_tool(&mut self, kind: ToolKind, ctx: &egui::Context) {
        let window = ToolWindow::new(self.next_tool_id, ToolRuntime::open(kind, ctx));
        self.next_tool_id += 1;
        self.open_tools.push(Arc::new(Mutex::new(window)));
    }

    fn default_tool_label(&self) -> String {
        match self.settings.default_tool_package.as_deref() {
            Some(package_name) => ToolKind::from_package_name(package_name)
                .map(|kind| kind.display_name().to_string())
                .unwrap_or_else(|| format!("Unknown ({package_name})")),
            None => "Disabled".to_string(),
        }
    }

    fn set_default_tool(&mut self, default_tool_package: Option<String>) {
        self.settings.default_tool_package = default_tool_package;
        self.settings_status = match self.settings.save(&self.settings_path) {
            Ok(()) => match self.settings.default_tool_package.as_deref() {
                Some(package_name) => ToolKind::from_package_name(package_name)
                    .map(|kind| {
                        format!(
                            "Default auto-open tool set to {}.",
                            kind.display_name()
                        )
                    })
                    .unwrap_or_else(|| format!("Saved unknown default tool `{package_name}`.")),
                None => "Default auto-open tool disabled.".to_string(),
            },
            Err(error) => format!("Failed to save preferences: {error:#}"),
        };
    }

    fn set_startup_enabled(&mut self, enabled: bool) {
        let Some(app_executable) = self.startup.app_executable.as_deref() else {
            self.startup.status =
                "Startup registration is unavailable because the executable path could not be resolved."
                    .to_string();
            return;
        };

        match startup::set_enabled(app_executable, enabled)
            .and_then(|_| startup::is_enabled(app_executable))
        {
            Ok(actual_enabled) => {
                self.startup.enabled = actual_enabled;
                self.startup.status = if actual_enabled {
                    "Play GUI will start automatically when you sign in.".to_string()
                } else {
                    "Play GUI will not start automatically when you sign in.".to_string()
                };
            }
            Err(error) => {
                self.startup.status = format!("Failed to update startup registration: {error:#}");
            }
        }
    }

    fn render_tool_windows(&mut self, ctx: &egui::Context) {
        for tool_window in &self.open_tools {
            let tool_window = Arc::clone(tool_window);
            let (viewport_id, viewport_builder) = {
                let window = tool_window.lock().expect("tool window lock poisoned");
                (window.viewport_id(), window.runtime.viewport_builder())
            };

            ctx.show_viewport_deferred(viewport_id, viewport_builder, move |tool_ctx, _class| {
                let mut window = tool_window.lock().expect("tool window lock poisoned");

                if tool_ctx.input(|input| input.viewport().close_requested()) {
                    window.open = false;
                    tool_ctx.send_viewport_cmd(ViewportCommand::Close);
                    return;
                }

                window.runtime.show(tool_ctx);
            });
        }

        self.open_tools
            .retain(|window| window.lock().expect("tool window lock poisoned").open);
    }
}

impl eframe::App for PlayGuiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let _ = frame;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Play Toolbox");
            ui.label("Pure egui toolbox entrance with in-process desktop tools.");
            ui.add_space(10.0);

            render_preferences(ui, self);
            ui.add_space(10.0);

            render_toolbar(ui, self);
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut visible_count = 0usize;

                for tool in TOOL_DEFINITIONS {
                    if !self.matches_filter(tool.package_name, tool.display_name, tool.description) {
                        continue;
                    }

                    visible_count += 1;
                    render_tool_section(ui, self, tool, ctx);
                    ui.add_space(10.0);
                }

                if visible_count == 0 {
                    ui.label("No tools matched the current filter.");
                }
            });

            if !self.open_tools.is_empty() {
                ui.add_space(12.0);
                ui.separator();
                ui.label(format!("Open tool windows: {}", self.open_tools.len()));
            }
        });

        self.render_tool_windows(ctx);
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

fn render_preferences(ui: &mut egui::Ui, app: &mut PlayGuiApp) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Preferences");
        ui.add_space(6.0);

        let mut selected_default_tool = app.settings.default_tool_package.clone();

        ui.horizontal(|ui| {
            ui.label("Default tool");
            egui::ComboBox::from_id_salt("play-gui-default-tool")
                .selected_text(app.default_tool_label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected_default_tool, None, "Disabled");
                    for tool in TOOL_DEFINITIONS {
                        ui.selectable_value(
                            &mut selected_default_tool,
                            Some(tool.package_name.to_string()),
                            tool.display_name,
                        );
                    }
                });
        });

        ui.label("The selected tool opens automatically when Play GUI starts.");

        if selected_default_tool != app.settings.default_tool_package {
            app.set_default_tool(selected_default_tool);
        }

        ui.add_space(8.0);

        if app.startup.supported {
            let mut enabled = app.startup.enabled;
            if ui
                .checkbox(&mut enabled, "Start Play GUI automatically when you sign in")
                .changed()
            {
                app.set_startup_enabled(enabled);
            }

            ui.label("This uses the current executable path and takes effect on the next sign-in.");
        } else {
            ui.label("Sign-in startup is currently supported on macOS and Windows only.");
        }

        if !app.settings_status.is_empty() {
            ui.add_space(6.0);
            ui.label(&app.settings_status);
        }

        if !app.startup.status.is_empty() {
            ui.label(&app.startup.status);
        }
    });
}

fn render_tool_section(
    ui: &mut egui::Ui,
    app: &mut PlayGuiApp,
    tool: &ToolDefinition,
    ctx: &egui::Context,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading(tool.display_name);
        ui.label(tool.description);
        ui.add_space(6.0);
        ui.monospace(format!("package: {}", tool.package_name));
        if app.settings.default_tool_package.as_deref() == Some(tool.package_name) {
            ui.label("Default auto-open tool");
        }
        ui.add_space(8.0);

        if ui.button(format!("Open {}", tool.display_name)).clicked() {
            app.open_tool(tool.kind, ctx);
        }
    });
}

fn tool_definition(kind: ToolKind) -> &'static ToolDefinition {
    TOOL_DEFINITIONS
        .iter()
        .find(|tool| tool.kind == kind)
        .expect("tool definition missing")
}

struct StartupPreference {
    supported: bool,
    enabled: bool,
    app_executable: Option<PathBuf>,
    status: String,
}

impl StartupPreference {
    fn load() -> Self {
        if !startup::is_supported() {
            return Self {
                supported: false,
                enabled: false,
                app_executable: None,
                status: String::new(),
            };
        }

        let app_executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                return Self {
                    supported: true,
                    enabled: false,
                    app_executable: None,
                    status: format!("Failed to resolve current executable: {error}"),
                };
            }
        };

        match startup::is_enabled(&app_executable) {
            Ok(enabled) => Self {
                supported: true,
                enabled,
                app_executable: Some(app_executable),
                status: String::new(),
            },
            Err(error) => Self {
                supported: true,
                enabled: false,
                app_executable: Some(app_executable),
                status: format!("Failed to read startup registration: {error:#}"),
            },
        }
    }
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
        Box::new(|cc| Ok(Box::new(PlayGuiApp::new(&cc.egui_ctx)))),
    )
}
