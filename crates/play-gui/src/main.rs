use std::sync::{Arc, Mutex};

use eframe::egui::{self, ViewportBuilder, ViewportCommand, ViewportId};

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

#[derive(Clone, Copy)]
enum ToolKind {
    CurlHelper,
    FrpClient,
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
            Self::CurlHelper(_) => "Curl Helper",
            Self::FrpClient(_) => "FRP Client",
        }
    }

    fn package_name(&self) -> &'static str {
        match self {
            Self::CurlHelper(_) => "curl-helper",
            Self::FrpClient(_) => "frp-client",
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
}

impl PlayGuiApp {
    fn new() -> Self {
        Self {
            filter: String::new(),
            next_tool_id: 1,
            open_tools: Vec::new(),
        }
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

                let title = window.runtime.display_name();
                let package_name = window.runtime.package_name();
                let mut request_close = false;

                egui::TopBottomPanel::top(format!("play_gui_tool_toolbar_{}", window.id))
                    .frame(
                        egui::Frame::side_top_panel(&tool_ctx.style())
                            .inner_margin(egui::Margin::symmetric(12.0, 10.0)),
                    )
                    .show(tool_ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Close").clicked() {
                                request_close = true;
                            }
                            ui.separator();
                            ui.heading(title);
                            ui.label(format!("package: {package_name}"));
                        });
                    });

                if request_close {
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
        ui.add_space(8.0);

        if ui.button(format!("Open {}", tool.display_name)).clicked() {
            app.open_tool(tool.kind, ctx);
        }
    });
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
