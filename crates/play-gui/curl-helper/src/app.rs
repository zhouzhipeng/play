use eframe::egui;
use std::collections::HashSet;

use crate::curl_parser;
use crate::executor::{ExecutionResult, Executor};
use crate::storage::{AppData, CurlItem, Group, Storage};

/// One entry in the current execution batch shown on the right panel.
struct BatchEntry {
    curl_id: String,
    name: String,
    command: String,
    result: Option<ExecutionResult>,
    checked: bool,
}

/// A line in the diff view
enum DiffLine {
    Same,
    Added,
    Removed,
}

/// Simple line-based diff using longest common subsequence.
fn compute_diff(left: &str, right: &str) -> Vec<DiffLine> {
    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();
    let n = left_lines.len();
    let m = right_lines.len();

    // LCS table
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if left_lines[i - 1] == right_lines[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack
    let mut result = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && left_lines[i - 1] == right_lines[j - 1] {
            result.push(DiffLine::Same);
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            result.push(DiffLine::Added);
            j -= 1;
        } else {
            result.push(DiffLine::Removed);
            i -= 1;
        }
    }
    result.reverse();
    result
}

pub struct CurlHelperApp {
    // Data
    curls: Vec<CurlItem>,
    groups: Vec<Group>,
    active_index: Option<usize>,

    // Groups
    active_group_id: Option<String>,
    new_group_name: String,
    editing_group_id: Option<String>,
    editing_group_name: String,

    // Drag & drop
    dragging_curl_id: Option<String>,
    group_rects: Vec<(Option<String>, egui::Rect)>,
    card_rects: Vec<(usize, egui::Rect)>, // (curl index, rect) for reorder

    // Find & Replace
    show_find_replace: bool,
    find_text: String,
    replace_text: String,
    replace_scope_all: bool,

    // Execution
    executor: Executor,
    running: HashSet<String>,

    // Current batch results (right panel)
    current_batch: Vec<BatchEntry>,

    // Diff view
    show_diff: bool,
    diff_left_label: String,
    diff_right_label: String,
    diff_lines: Vec<DiffLine>,
    diff_left_body: String,
    diff_right_body: String,

    // Search filter
    search_filter: String,

    // Storage
    storage: Storage,
    dirty: bool,
    theme_initialized: bool,
}

// ── Icon drawing helpers ─────────────────────────────────────────────

fn icon_button(
    ui: &mut egui::Ui,
    _id_str: &str,
    size: f32,
    hover: &str,
    draw: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
) -> egui::Response {
    let desired = egui::vec2(size, size);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let color = if response.hovered() {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_rgb(160, 160, 160)
    };
    if ui.is_rect_visible(rect) {
        draw(ui.painter(), rect, color);
    }
    response.on_hover_text(hover)
}

fn draw_play(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.height() * 0.32;
    let points = vec![
        egui::pos2(c.x - s * 0.5, c.y - s),
        egui::pos2(c.x + s * 0.8, c.y),
        egui::pos2(c.x - s * 0.5, c.y + s),
    ];
    painter.add(egui::Shape::convex_polygon(points, color, egui::Stroke::NONE));
}

fn draw_delete(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.height() * 0.24;
    let stroke = egui::Stroke::new(1.8, color);
    painter.line_segment([egui::pos2(c.x - s, c.y - s), egui::pos2(c.x + s, c.y + s)], stroke);
    painter.line_segment([egui::pos2(c.x + s, c.y - s), egui::pos2(c.x - s, c.y + s)], stroke);
}

fn draw_copy(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.height() * 0.22;
    let stroke = egui::Stroke::new(1.3, color);
    let r1 = egui::Rect::from_min_size(egui::pos2(c.x - s * 1.0, c.y - s * 0.9), egui::vec2(s * 1.4, s * 1.6));
    painter.rect_stroke(r1, egui::Rounding::same(1.5), stroke);
    let r2 = egui::Rect::from_min_size(egui::pos2(c.x - s * 0.4, c.y - s * 0.3), egui::vec2(s * 1.4, s * 1.6));
    painter.rect_filled(r2, egui::Rounding::same(1.5), egui::Color32::from_rgb(30, 35, 45));
    painter.rect_stroke(r2, egui::Rounding::same(1.5), stroke);
}

fn draw_grip(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.height() * 0.3;
    let stroke = egui::Stroke::new(1.8, color);
    for dy in [-1.0_f32, 0.0, 1.0] {
        let y = c.y + dy * s;
        painter.line_segment([egui::pos2(c.x - s * 0.8, y), egui::pos2(c.x + s * 0.8, y)], stroke);
    }
}

/// Full-width left-aligned selectable row for group list.
fn group_row(ui: &mut egui::Ui, width: f32, height: f32, selected: bool, label: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let visuals = ui.visuals();
    if selected {
        ui.painter().rect_filled(rect, egui::Rounding::same(3.0), visuals.selection.bg_fill);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, egui::Rounding::same(3.0), visuals.widgets.hovered.bg_fill);
    }
    let text_color = if selected { egui::Color32::WHITE } else { visuals.text_color() };
    ui.painter().text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        text_color,
    );
    resp
}

fn status_color(code: Option<i32>) -> egui::Color32 {
    match code {
        Some(c) if (200..300).contains(&c) => egui::Color32::from_rgb(80, 200, 120),
        Some(c) if (300..400).contains(&c) => egui::Color32::from_rgb(255, 200, 50),
        _ => egui::Color32::from_rgb(255, 80, 80),
    }
}

// ── App impl ────────────────────────────────────────────────────────

impl CurlHelperApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::setup_fonts(&cc.egui_ctx);

        let storage = Storage::new();
        let mut data = storage.load();
        for c in &mut data.curls {
            c.selected = false;
        }
        let active = if data.curls.is_empty() { None } else { Some(0) };

        Self {
            curls: data.curls,
            groups: data.groups,
            active_index: active,
            active_group_id: None,
            new_group_name: String::new(),
            editing_group_id: None,
            editing_group_name: String::new(),
            dragging_curl_id: None,
            group_rects: Vec::new(),
            card_rects: Vec::new(),
            show_find_replace: false,
            find_text: String::new(),
            replace_text: String::new(),
            replace_scope_all: true,
            executor: Executor::new(),
            running: HashSet::new(),
            current_batch: Vec::new(),
            show_diff: false,
            diff_left_label: String::new(),
            diff_right_label: String::new(),
            diff_lines: Vec::new(),
            diff_left_body: String::new(),
            diff_right_body: String::new(),
            search_filter: String::new(),
            storage,
            dirty: false,
            theme_initialized: false,
        }
    }

    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // Load macOS system CJK font as fallback
        let font_paths = [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ];

        for path in &font_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "cjk".to_owned(),
                    egui::FontData::from_owned(data),
                );

                // Append as fallback to both families
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    family.push("cjk".to_owned());
                }
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    family.push("cjk".to_owned());
                }

                break;
            }
        }

        ctx.set_fonts(fonts);
    }

    fn check_results(&mut self) {
        while let Ok(result) = self.executor.receiver.try_recv() {
            self.running.remove(&result.curl_id);
            // Fill into the batch entry
            if let Some(entry) = self.current_batch.iter_mut().find(|e| e.curl_id == result.curl_id && e.result.is_none()) {
                entry.result = Some(result.clone());
            }
            // Also persist into the CurlItem
            if let Some(curl) = self.curls.iter_mut().find(|c| c.id == result.curl_id) {
                curl.results.insert(0, result);
                curl.results.truncate(50);
            }
            self.dirty = true;
        }
    }

    /// Start executing a single curl command and add it to the batch.
    fn execute_single(&mut self, index: usize) {
        if index >= self.curls.len() {
            return;
        }
        let curl = &self.curls[index];
        if self.running.contains(&curl.id) {
            return;
        }
        // Clear previous batch and start a fresh one
        self.current_batch.clear();
        self.current_batch.push(BatchEntry {
            curl_id: curl.id.clone(),
            name: curl.display_name(),
            command: curl.command.clone(),
            result: None,
            checked: false,
        });
        self.running.insert(curl.id.clone());
        self.executor.execute(curl.id.clone(), curl.command.clone());
    }

    /// Start executing multiple curl commands as a batch.
    fn execute_batch(&mut self, indices: &[usize]) {
        self.current_batch.clear();
        for &i in indices {
            if i >= self.curls.len() {
                continue;
            }
            let curl = &self.curls[i];
            if self.running.contains(&curl.id) {
                continue;
            }
            self.current_batch.push(BatchEntry {
                curl_id: curl.id.clone(),
                name: curl.display_name(),
                command: curl.command.clone(),
                result: None,
            checked: false,
            });
            self.running.insert(curl.id.clone());
            self.executor.execute(curl.id.clone(), curl.command.clone());
        }
    }

    fn auto_save(&mut self) {
        if self.dirty {
            let data = AppData {
                curls: self.curls.clone(),
                groups: self.groups.clone(),
            };
            self.storage.save(&data);
            self.dirty = false;
        }
    }

    // ── Toolbar ──────────────────────────────────────────────────────
    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new("+ Add Command").size(14.0)).clicked() {
                let mut curl = CurlItem::new(String::new(), String::new());
                curl.group_id = self.active_group_id.clone();
                self.curls.insert(0, curl);
                if let Some(a) = self.active_index {
                    self.active_index = Some(a + 1);
                }
                self.active_index = Some(0);
                self.dirty = true;
            }

            ui.separator();

            let filter = self.search_filter.to_lowercase();
            let is_visible = |c: &CurlItem| -> bool {
                // Group filter
                if let Some(ref gid) = self.active_group_id {
                    if c.group_id.as_deref() != Some(gid) {
                        return false;
                    }
                }
                // Search filter
                if !filter.is_empty() {
                    return c.name.to_lowercase().contains(&filter)
                        || c.command.to_lowercase().contains(&filter);
                }
                true
            };
            let visible_count = self.curls.iter().filter(|c| is_visible(c)).count();
            let all_selected = visible_count > 0 && self.curls.iter().filter(|c| is_visible(c)).all(|c| c.selected);
            if ui
                .button(egui::RichText::new(if all_selected { "Deselect All" } else { "Select All" }).size(14.0))
                .clicked()
            {
                // First: deselect all
                for c in &mut self.curls {
                    c.selected = false;
                }
                // Then: select visible (only when toggling on)
                if !all_selected {
                    let filter2 = self.search_filter.to_lowercase();
                    for c in &mut self.curls {
                        let vis = match &self.active_group_id {
                            Some(gid) => c.group_id.as_deref() == Some(gid),
                            None => true,
                        };
                        if vis && (filter2.is_empty() || c.name.to_lowercase().contains(&filter2) || c.command.to_lowercase().contains(&filter2)) {
                            c.selected = true;
                        }
                    }
                }
            }

            ui.separator();

            let selected_count = self.curls.iter().filter(|c| c.selected).count();
            let btn_text = if selected_count > 0 {
                format!("Execute Selected ({})", selected_count)
            } else {
                "Execute Selected".to_string()
            };
            if ui
                .add_enabled(selected_count > 0, egui::Button::new(egui::RichText::new(btn_text).size(14.0)))
                .clicked()
            {
                let indices: Vec<usize> = self.curls.iter().enumerate().filter(|(_, c)| c.selected).map(|(i, _)| i).collect();
                self.execute_batch(&indices);
            }

            ui.separator();

            let fr_label = if self.show_find_replace { "Close Find" } else { "Find & Replace" };
            if ui.button(egui::RichText::new(fr_label).size(14.0)).clicked() {
                self.show_find_replace = !self.show_find_replace;
            }

            if !self.running.is_empty() {
                ui.separator();
                ui.spinner();
                ui.label(
                    egui::RichText::new(format!("{} running...", self.running.len()))
                        .color(egui::Color32::from_rgb(255, 200, 50)),
                );
            }
        });
    }

    // ── Groups panel ─────────────────────────────────────────────────
    fn render_groups_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Groups").size(14.0).strong());
        ui.add_space(4.0);

        self.group_rects.clear();
        let is_dragging = self.dragging_curl_id.is_some();
        let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());

        let row_width = ui.available_width();
        let row_height = 24.0;

        // Deferred group actions
        let mut exec_group_id: Option<Option<String>> = None; // None=all, Some(gid)
        let mut export_group_id: Option<Option<String>> = None;
        let mut import_group_id: Option<Option<String>> = None;

        // "All"
        {
            let count = self.curls.len();
            let resp = group_row(ui, row_width, row_height, self.active_group_id.is_none(), &format!("All ({})", count));
            let row_rect = resp.rect;
            self.group_rects.push((None, row_rect));
            if resp.clicked() && !is_dragging {
                self.active_group_id = None;
                self.search_filter.clear();
            }
            resp.context_menu(|ui| {
                if ui.button("Execute All").clicked() {
                    exec_group_id = Some(None);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export All").clicked() {
                    export_group_id = Some(None);
                    ui.close_menu();
                }
                if ui.button("Import").clicked() {
                    import_group_id = Some(None);
                    ui.close_menu();
                }
            });
            if is_dragging {
                if let Some(pos) = pointer_pos {
                    if row_rect.contains(pos) {
                        ui.painter().rect_filled(row_rect, egui::Rounding::same(3.0), egui::Color32::from_rgba_premultiplied(100, 200, 255, 40));
                        ui.painter().rect_stroke(row_rect, egui::Rounding::same(3.0), egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)));
                    }
                }
            }
        }

        ui.separator();

        let mut delete_group: Option<usize> = None;
        let mut duplicate_group: Option<usize> = None;
        for gi in 0..self.groups.len() {
            let gid = self.groups[gi].id.clone();
            let count = self.curls.iter().filter(|c| c.group_id.as_deref() == Some(&gid)).count();

            if self.editing_group_id.as_deref() == Some(&gid) {
                ui.horizontal(|ui| {
                    let te = ui.add(egui::TextEdit::singleline(&mut self.editing_group_name).desired_width(ui.available_width() - 30.0));
                    if te.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.groups[gi].name = self.editing_group_name.clone();
                        self.editing_group_id = None;
                        self.dirty = true;
                    }
                });
                continue;
            }

            let selected = self.active_group_id.as_deref() == Some(&gid);
            let resp = group_row(ui, row_width, row_height, selected, &format!("{} ({})", self.groups[gi].name, count));
            let row_rect = resp.rect;
            self.group_rects.push((Some(gid.clone()), row_rect));

            if resp.clicked() && !is_dragging {
                self.active_group_id = Some(gid.clone());
                self.search_filter.clear();
            }

            resp.context_menu(|ui| {
                if ui.button("Execute All in Group").clicked() {
                    exec_group_id = Some(Some(gid.clone()));
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Rename").clicked() {
                    self.editing_group_id = Some(gid.clone());
                    self.editing_group_name = self.groups[gi].name.clone();
                    ui.close_menu();
                }
                if ui.button("Duplicate Group").clicked() {
                    duplicate_group = Some(gi);
                    ui.close_menu();
                }
                if ui.button("Delete Group").clicked() {
                    delete_group = Some(gi);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export Group").clicked() {
                    export_group_id = Some(Some(gid.clone()));
                    ui.close_menu();
                }
                if ui.button("Import into Group").clicked() {
                    import_group_id = Some(Some(gid.clone()));
                    ui.close_menu();
                }
            });

            if is_dragging {
                if let Some(pos) = pointer_pos {
                    if row_rect.contains(pos) {
                        ui.painter().rect_filled(row_rect, egui::Rounding::same(3.0), egui::Color32::from_rgba_premultiplied(100, 200, 255, 40));
                        ui.painter().rect_stroke(row_rect, egui::Rounding::same(3.0), egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)));
                    }
                }
            }
        }

        if let Some(gi) = duplicate_group {
            let old_gid = self.groups[gi].id.clone();
            let new_group = Group::new(format!("{} (copy)", self.groups[gi].name));
            let new_gid = new_group.id.clone();
            self.groups.insert(gi + 1, new_group);
            // Duplicate all commands in the group
            let dups: Vec<CurlItem> = self.curls.iter()
                .filter(|c| c.group_id.as_deref() == Some(&old_gid))
                .map(|c| {
                    let mut dup = c.clone();
                    dup.id = uuid::Uuid::new_v4().to_string();
                    dup.group_id = Some(new_gid.clone());
                    dup.results.clear();
                    dup.selected = false;
                    dup
                })
                .collect();
            self.curls.extend(dups);
            self.dirty = true;
        }

        if let Some(gi) = delete_group {
            let gid = self.groups[gi].id.clone();
            for curl in &mut self.curls {
                if curl.group_id.as_deref() == Some(&gid) {
                    curl.group_id = None;
                }
            }
            if self.active_group_id.as_deref() == Some(&gid) {
                self.active_group_id = None;
            }
            self.groups.remove(gi);
            self.dirty = true;
        }

        // ── Execute all in group ──
        if let Some(gid_opt) = exec_group_id {
            let indices: Vec<usize> = self.curls.iter().enumerate()
                .filter(|(_, c)| match &gid_opt {
                    Some(gid) => c.group_id.as_deref() == Some(gid),
                    None => true,
                })
                .map(|(i, _)| i)
                .collect();
            self.execute_batch(&indices);
        }

        // ── Export group ──
        if let Some(gid_opt) = export_group_id {
            let items: Vec<&CurlItem> = self.curls.iter()
                .filter(|c| match &gid_opt {
                    Some(gid) => c.group_id.as_deref() == Some(gid),
                    None => true,
                })
                .collect();
            let group_name = match &gid_opt {
                Some(gid) => self.groups.iter().find(|g| g.id == *gid).map(|g| g.name.clone()).unwrap_or("group".into()),
                None => "all".into(),
            };
            // Strip results before export
            let export_items: Vec<CurlItem> = items.iter().map(|c| {
                let mut cl = (*c).clone();
                cl.results.clear();
                cl
            }).collect();
            if let Ok(json) = serde_json::to_string_pretty(&export_items) {
                let default_name = format!("{}.json", group_name);
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(&default_name)
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    let _ = std::fs::write(path, json);
                }
            }
        }

        // ── Import group ──
        if let Some(gid_opt) = import_group_id {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut items) = serde_json::from_str::<Vec<CurlItem>>(&content) {
                        for item in &mut items {
                            item.id = uuid::Uuid::new_v4().to_string();
                            item.group_id = gid_opt.clone();
                            item.selected = false;
                        }
                        self.curls.extend(items);
                        self.dirty = true;
                    }
                }
            }
        }

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let te = ui.add(egui::TextEdit::singleline(&mut self.new_group_name).desired_width(ui.available_width() - 32.0).hint_text("New group"));
            if (ui.button("+").clicked() || (te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && !self.new_group_name.trim().is_empty() {
                self.groups.push(Group::new(self.new_group_name.trim().to_string()));
                self.new_group_name.clear();
                self.dirty = true;
            }
        });
    }

    // ── Left panel: command cards ────────────────────────────────────
    fn render_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add(egui::TextEdit::singleline(&mut self.search_filter).desired_width(ui.available_width()).hint_text("Filter commands..."));
        });
        ui.add_space(4.0);

        if self.curls.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(egui::RichText::new("No commands yet.\nClick '+ Add' to create one.").size(14.0).color(egui::Color32::GRAY));
            });
            return;
        }

        let filter = self.search_filter.to_lowercase();
        let active_gid = self.active_group_id.clone();
        let visible_indices: Vec<usize> = (0..self.curls.len())
            .filter(|&i| {
                let c = &self.curls[i];
                if let Some(ref gid) = active_gid {
                    if c.group_id.as_deref() != Some(gid) {
                        return false;
                    }
                }
                if !filter.is_empty() {
                    return c.name.to_lowercase().contains(&filter) || c.command.to_lowercase().contains(&filter);
                }
                true
            })
            .collect();

        if visible_indices.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(egui::RichText::new("No matching commands").size(13.0).color(egui::Color32::GRAY));
            });
            return;
        }

        let mut delete_idx: Option<usize> = None;
        let mut execute_idx: Option<usize> = None;
        let mut copy_idx: Option<usize> = None;
        let mut new_active: Option<usize> = None;
        let mut drag_started_id: Option<String> = None;

        self.card_rects.clear();
        let is_dragging = self.dragging_curl_id.is_some();
        let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());

        egui::ScrollArea::vertical().id_salt("curl_cards").show(ui, |ui| {
            for &i in &visible_indices {
                let is_active = self.active_index == Some(i);
                let is_running = self.running.contains(&self.curls[i].id);
                let is_dragged_card = self.dragging_curl_id.as_deref() == Some(&self.curls[i].id);

                // Dim the card being dragged
                let alpha = if is_dragged_card { 0.3 } else { 1.0 };

                let stroke = if is_active {
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 160, 255).gamma_multiply(alpha))
                } else {
                    egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60))
                };
                let card_fill = if is_active {
                    egui::Color32::from_rgb(30, 35, 45).gamma_multiply(alpha)
                } else {
                    egui::Color32::from_rgb(25, 25, 30).gamma_multiply(alpha)
                };

                // Draw insertion indicator above this card if dragging over it
                if is_dragging && !is_dragged_card {
                    if let Some(pos) = pointer_pos {
                        // Check if pointer is in the top half of the gap above this card
                        let card_top = ui.cursor().min.y;
                        if pos.y < card_top + 2.0 && pos.y > card_top - 6.0 {
                            let line_rect = egui::Rect::from_min_size(
                                egui::pos2(ui.min_rect().left(), card_top - 2.0),
                                egui::vec2(ui.available_width(), 3.0),
                            );
                            ui.painter().rect_filled(line_rect, egui::Rounding::same(1.5), egui::Color32::from_rgb(100, 180, 255));
                        }
                    }
                }

                let frame_resp = egui::Frame::none()
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(stroke)
                    .rounding(egui::Rounding::same(6.0))
                    .fill(card_fill)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Drag grip
                            let grip_resp = icon_button(ui, &format!("grip_{}", i), 18.0, "Drag to group", |p, r, _c| {
                                draw_grip(p, r, egui::Color32::from_rgb(140, 140, 140));
                            });
                            let grip_drag = ui.interact(grip_resp.rect, egui::Id::new(format!("grip_drag_{}", self.curls[i].id)), egui::Sense::drag());
                            if grip_resp.drag_started() || grip_drag.drag_started() {
                                drag_started_id = Some(self.curls[i].id.clone());
                            }

                            ui.checkbox(&mut self.curls[i].selected, "");

                            let name_hint = format!("Command #{}", i + 1);
                            let name_resp = ui.add(
                                egui::TextEdit::singleline(&mut self.curls[i].name)
                                    .desired_width(ui.available_width() - 80.0)
                                    .hint_text(&name_hint)
                                    .font(egui::TextStyle::Body),
                            );
                            if name_resp.changed() { self.dirty = true; }
                            if name_resp.clicked() || name_resp.gained_focus() { new_active = Some(i); }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if icon_button(ui, &format!("del_{}", i), 20.0, "Delete", draw_delete).clicked() {
                                    delete_idx = Some(i);
                                }
                                if icon_button(ui, &format!("copy_{}", i), 20.0, "Duplicate", draw_copy).clicked() {
                                    copy_idx = Some(i);
                                }
                                if is_running {
                                    ui.spinner();
                                } else if icon_button(ui, &format!("run_{}", i), 20.0, "Execute", |p, r, _c| {
                                    draw_play(p, r, egui::Color32::from_rgb(80, 200, 120));
                                }).clicked() {
                                    execute_idx = Some(i);
                                    new_active = Some(i);
                                }
                            });
                        });

                        ui.add_space(4.0);

                        let editor_id = format!("editor_{}", self.curls[i].id);
                        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                            let mut job = curl_parser::highlight(string);
                            job.wrap.max_width = wrap_width;
                            ui.fonts(|f| f.layout_job(job))
                        };
                        let resp = ui.add(
                            egui::TextEdit::multiline(&mut self.curls[i].command)
                                .id(egui::Id::new(&editor_id))
                                .desired_width(f32::INFINITY)
                                .desired_rows(3)
                                .hint_text("curl ...")
                                .font(egui::TextStyle::Monospace)
                                .layouter(&mut layouter),
                        );
                        if resp.changed() { self.dirty = true; }
                        if resp.clicked() || resp.gained_focus() { new_active = Some(i); }

                        // Editable parameters panel (only for active card)
                        if is_active {
                            let params: Vec<_> = curl_parser::extract_params(&self.curls[i].command).into_iter().filter(|(cat, _, _)| cat != "Method").collect();
                            if !params.is_empty() {
                                let params_id = format!("params_{}", self.curls[i].id);
                                egui::CollapsingHeader::new(
                                    egui::RichText::new(format!("Parameters ({})", params.len()))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(160, 170, 190)),
                                )
                                .id_salt(&params_id)
                                .default_open(true)
                                .show(ui, |ui| {
                                    egui::Grid::new(format!("params_grid_{}", self.curls[i].id))
                                        .num_columns(2)
                                        .spacing([6.0, 3.0])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for (_cat, key, val) in &params {
                                                // Key
                                                ui.label(egui::RichText::new(key).size(12.0).color(egui::Color32::from_rgb(156, 220, 254)));

                                                // Editable value
                                                let mut edit_val = val.clone();
                                                let te_resp = ui.add(
                                                    egui::TextEdit::singleline(&mut edit_val)
                                                        .desired_width(ui.available_width().max(120.0))
                                                        .font(egui::FontId::monospace(12.0)),
                                                );
                                                if te_resp.changed() && edit_val != *val {
                                                    self.curls[i].command = curl_parser::replace_param_value(
                                                        &self.curls[i].command,
                                                        key,
                                                        val,
                                                        &edit_val,
                                                    );
                                                    self.dirty = true;
                                                }
                                                ui.end_row();
                                            }
                                        });
                                });
                            }
                        }

                        // Last result indicator
                        if let Some(last) = self.curls[i].results.first() {
                            ui.horizontal(|ui| {
                                let code_str = last.status_code.map(|c| c.to_string()).unwrap_or("ERR".into());
                                ui.colored_label(status_color(last.status_code), egui::RichText::new(&code_str).size(11.0));
                                ui.label(egui::RichText::new(format!("{}ms", last.duration_ms)).size(11.0).color(egui::Color32::from_rgb(130, 130, 130)));
                                ui.label(egui::RichText::new(last.timestamp.format("%H:%M:%S").to_string()).size(11.0).color(egui::Color32::from_rgb(100, 100, 100)));
                            });
                        }
                    });

                // Store card rect for reorder
                self.card_rects.push((i, frame_resp.response.rect));

                ui.add_space(4.0);
            }
        });

        // Deferred actions
        if let Some(id) = drag_started_id { self.dragging_curl_id = Some(id); }
        if let Some(i) = new_active {
            if self.active_index != Some(i) { self.active_index = Some(i); }
        }
        if let Some(i) = execute_idx {
            self.execute_single(i);
        }
        if let Some(i) = copy_idx {
            let mut dup = self.curls[i].clone();
            dup.id = uuid::Uuid::new_v4().to_string();
            dup.name = if dup.name.is_empty() { String::new() } else { format!("{} (copy)", dup.name) };
            dup.results.clear();
            dup.selected = false;
            self.curls.insert(i + 1, dup);
            if let Some(active) = self.active_index {
                if active > i { self.active_index = Some(active + 1); }
            }
            self.dirty = true;
        }
        if let Some(i) = delete_idx {
            self.curls.remove(i);
            if self.curls.is_empty() {
                self.active_index = None;
            } else if let Some(active) = self.active_index {
                if active >= self.curls.len() {
                    self.active_index = Some(self.curls.len() - 1);
                } else if active > i {
                    self.active_index = Some(active - 1);
                }
            }
            self.dirty = true;
        }
    }

    // ── Handle drag-to-group drop ────────────────────────────────────
    fn handle_drag_drop(&mut self, ctx: &egui::Context) {
        if self.dragging_curl_id.is_none() { return; }
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let curl_name = self.curls.iter().find(|c| Some(&c.id) == self.dragging_curl_id.as_ref()).map(|c| c.display_name()).unwrap_or_default();
            let short = if curl_name.len() > 25 { format!("{}...", &curl_name[..25]) } else { curl_name };
            egui::Area::new(egui::Id::new("drag_ghost")).fixed_pos(pos + egui::vec2(12.0, 12.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).inner_margin(egui::Margin::symmetric(8.0, 4.0)).show(ui, |ui| {
                    ui.label(egui::RichText::new(short).size(12.0));
                });
            });
        }
        if ctx.input(|i| i.pointer.primary_released()) {
            let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
            if let (Some(curl_id), Some(pos)) = (self.dragging_curl_id.take(), pointer_pos) {
                // 1. Check if dropped on a group
                let mut handled = false;
                for (gid, rect) in &self.group_rects {
                    if rect.contains(pos) {
                        if let Some(curl) = self.curls.iter_mut().find(|c| c.id == curl_id) {
                            curl.group_id = gid.clone();
                            self.dirty = true;
                        }
                        handled = true;
                        break;
                    }
                }

                // 2. If not on a group, check for reorder among cards
                if !handled {
                    let from = self.curls.iter().position(|c| c.id == curl_id);
                    if let Some(from_idx) = from {
                        // Find insertion point: the card whose rect center-y is closest below the pointer
                        let mut target_idx: Option<usize> = None;
                        for (card_idx, card_rect) in &self.card_rects {
                            if *card_idx == from_idx {
                                continue;
                            }
                            let mid_y = card_rect.center().y;
                            if pos.y < mid_y {
                                target_idx = Some(*card_idx);
                                break;
                            }
                        }

                        // Compute final insertion position
                        let to_idx = match target_idx {
                            Some(ti) => {
                                if ti > from_idx { ti - 1 } else { ti }
                            }
                            None => {
                                // Dropped below all cards → move to end
                                self.curls.len() - 1
                            }
                        };

                        if to_idx != from_idx {
                            let item = self.curls.remove(from_idx);
                            let insert_at = to_idx.min(self.curls.len());
                            self.curls.insert(insert_at, item);
                            // Fix active_index
                            self.active_index = self.curls.iter().position(|c| c.id == curl_id);
                            self.dirty = true;
                        }
                    }
                }
            }
            self.dragging_curl_id = None;
        }
    }

    // ── Right panel: batch results ───────────────────────────────────
    fn render_results_panel(&mut self, ui: &mut egui::Ui) {
        if self.current_batch.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(egui::RichText::new("Execute a command to see results here.").size(14.0).color(egui::Color32::GRAY));
            });
            return;
        }

        // Check Diff button when ≥2 checked
        let checked_count = self.current_batch.iter().filter(|e| e.checked).count();
        if checked_count >= 2 {
            if ui.button(egui::RichText::new(format!("Check Diff ({} selected)", checked_count)).size(13.0)).clicked() {
                // Collect checked bodies
                let checked: Vec<(String, String)> = self.current_batch.iter()
                    .enumerate()
                    .filter(|(_, e)| e.checked && e.result.is_some())
                    .map(|(bi, e)| {
                        let label = if e.name.is_empty() { format!("#{}", bi + 1) } else { format!("#{} {}", bi + 1, e.name) };
                        let body = e.result.as_ref().map(|r| {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&r.body) {
                                serde_json::to_string_pretty(&json).unwrap_or_else(|_| r.body.clone())
                            } else {
                                r.body.clone()
                            }
                        }).unwrap_or_default();
                        (label, body)
                    })
                    .collect();

                if checked.len() >= 2 {
                    self.diff_left_label = checked[0].0.clone();
                    self.diff_right_label = checked[1].0.clone();
                    self.diff_left_body = checked[0].1.clone();
                    self.diff_right_body = checked[1].1.clone();
                    self.diff_lines = compute_diff(&checked[0].1, &checked[1].1);
                    self.show_diff = true;
                }
            }
            ui.add_space(4.0);
        }

        egui::ScrollArea::vertical().id_salt("batch_results").show(ui, |ui| {
            for bi in 0..self.current_batch.len() {
                let header_label = if self.current_batch[bi].name.is_empty() {
                    format!("#{}", bi + 1)
                } else {
                    format!("#{} — {}", bi + 1, self.current_batch[bi].name)
                };

                egui::Frame::none()
                    .inner_margin(egui::Margin::same(10.0))
                    .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 55, 60)))
                    .rounding(egui::Rounding::same(6.0))
                    .fill(egui::Color32::from_rgb(25, 27, 33))
                    .show(ui, |ui| {
                        // ── Header: checkbox + name + status ──
                        ui.horizontal(|ui| {
                            // Checkbox for diff comparison
                            if self.current_batch[bi].result.is_some() {
                                ui.checkbox(&mut self.current_batch[bi].checked, "");
                            }

                            // Truncate name to fit
                            let truncated = if header_label.len() > 40 {
                                format!("{}...", &header_label[..40])
                            } else {
                                header_label.clone()
                            };

                            ui.add(egui::Label::new(
                                egui::RichText::new(&truncated).size(13.0).strong().color(egui::Color32::from_rgb(180, 200, 255))
                            ).truncate());

                            if let Some(ref res) = self.current_batch[bi].result {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format!("{}ms", res.duration_ms)).size(11.0).color(egui::Color32::from_rgb(130, 130, 130)));
                                    let code_str = res.status_code.map(|c| c.to_string()).unwrap_or("ERR".into());
                                    ui.colored_label(status_color(res.status_code), egui::RichText::new(&code_str).size(13.0).strong());
                                });
                            } else {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.spinner();
                                    ui.label(egui::RichText::new("Running...").size(11.0).color(egui::Color32::from_rgb(255, 200, 50)));
                                });
                            }
                        });

                        ui.add_space(4.0);

                        // ── Command ──
                        let cmd_id = format!("cmd_display_{}", bi);
                        egui::CollapsingHeader::new(egui::RichText::new("Command").size(12.0).color(egui::Color32::from_rgb(140, 140, 140)))
                            .id_salt(&cmd_id)
                            .default_open(false)
                            .show(ui, |ui| {
                                let mut cmd = self.current_batch[bi].command.clone();
                                let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                                    let mut job = curl_parser::highlight(string);
                                    job.wrap.max_width = wrap_width;
                                    ui.fonts(|f| f.layout_job(job))
                                };
                                ui.add(
                                    egui::TextEdit::multiline(&mut cmd)
                                        .desired_width(f32::INFINITY)
                                        .font(egui::TextStyle::Monospace)
                                        .layouter(&mut layouter)
                                        .interactive(false),
                                );
                            });

                        if let Some(ref res) = self.current_batch[bi].result {
                            if let Some(ref err) = res.error {
                                ui.add_space(2.0);
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
                            }

                            // ── Response Body ──
                            ui.add_space(4.0);
                            let body_id = format!("body_{}", bi);
                            egui::CollapsingHeader::new(egui::RichText::new("Response Body").size(12.0))
                                .id_salt(&body_id)
                                .default_open(true)
                                .show(ui, |ui| {
                                    let body = &res.body;
                                    let display = if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                                        serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.clone())
                                    } else {
                                        body.clone()
                                    };
                                    let mut text = display;
                                    ui.add(
                                        egui::TextEdit::multiline(&mut text)
                                            .desired_width(f32::INFINITY)
                                            .font(egui::TextStyle::Monospace)
                                            .interactive(true),
                                    );
                                });

                            // ── Response Headers ──
                            if !res.headers.is_empty() {
                                let hdr_id = format!("headers_{}", bi);
                                egui::CollapsingHeader::new(egui::RichText::new("Response Headers").size(12.0).color(egui::Color32::from_rgb(140, 140, 140)))
                                    .id_salt(&hdr_id)
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        let mut h = res.headers.clone();
                                        ui.add(
                                            egui::TextEdit::multiline(&mut h)
                                                .desired_width(f32::INFINITY)
                                                .font(egui::TextStyle::Monospace)
                                                .interactive(true),
                                        );
                                    });
                            }
                        }
                    });

                ui.add_space(6.0);
            }
        });
    }

    // ── Diff dialog (side-by-side) ──────────────────────────────────
    fn render_diff_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_diff;
        egui::Window::new("Response Diff")
            .open(&mut open)
            .resizable(true)
            .default_width(1100.0)
            .default_height(650.0)
            .show(ctx, |ui| {
                // Stats
                let added = self.diff_lines.iter().filter(|l| matches!(l, DiffLine::Added)).count();
                let removed = self.diff_lines.iter().filter(|l| matches!(l, DiffLine::Removed)).count();
                let same = self.diff_lines.iter().filter(|l| matches!(l, DiffLine::Same)).count();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{} unchanged", same)).size(11.0).color(egui::Color32::from_rgb(150, 150, 150)));
                    ui.colored_label(egui::Color32::from_rgb(255, 120, 120), egui::RichText::new(format!("{} removed", removed)).size(11.0));
                    ui.colored_label(egui::Color32::from_rgb(120, 255, 120), egui::RichText::new(format!("{} added", added)).size(11.0));
                });
                ui.add_space(2.0);

                // Build per-line diff markers for left and right
                // left_markers[line_idx] = true if that line is a "removed" line
                // right_markers[line_idx] = true if that line is an "added" line
                let left_src_lines: Vec<&str> = self.diff_left_body.lines().collect();
                let right_src_lines: Vec<&str> = self.diff_right_body.lines().collect();

                let mut left_diff_set = std::collections::HashSet::new();
                let mut right_diff_set = std::collections::HashSet::new();
                {
                    let mut li = 0usize;
                    let mut ri = 0usize;
                    for dl in &self.diff_lines {
                        match dl {
                            DiffLine::Same => { li += 1; ri += 1; }
                            DiffLine::Removed => { left_diff_set.insert(li); li += 1; }
                            DiffLine::Added => { right_diff_set.insert(ri); ri += 1; }
                        }
                    }
                }

                let mono = egui::FontId::monospace(12.0);
                let half_w = ui.available_width() / 2.0 - 6.0;
                let gutter_w = 14.0;
                let line_h = 16.0;

                // Column headers
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(egui::vec2(half_w, line_h), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 150, 150), egui::RichText::new(&self.diff_left_label).size(12.0).strong());
                    });
                    ui.separator();
                    ui.colored_label(egui::Color32::from_rgb(150, 255, 150), egui::RichText::new(&self.diff_right_label).size(12.0).strong());
                });
                ui.separator();

                // Two side-by-side scroll areas sharing height
                let avail = ui.available_size();
                ui.horizontal(|ui| {
                    // ── Left panel ──
                    ui.allocate_ui_with_layout(
                        egui::vec2(half_w, avail.y),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            egui::ScrollArea::both()
                                .id_salt("diff_left")
                                .show(ui, |ui| {
                                    for (li, line) in left_src_lines.iter().enumerate() {
                                        let is_diff = left_diff_set.contains(&li);
                                        ui.horizontal(|ui| {
                                            // Gutter marker
                                            let (gr, _) = ui.allocate_exact_size(egui::vec2(gutter_w, line_h), egui::Sense::hover());
                                            if is_diff {
                                                ui.painter().circle_filled(gr.center(), 4.0, egui::Color32::from_rgb(255, 100, 100));
                                            }
                                            // Line number
                                            ui.label(egui::RichText::new(format!("{:>3}", li + 1)).font(mono.clone()).size(11.0).color(egui::Color32::from_rgb(80, 80, 80)));
                                            // Text
                                            let color = if is_diff {
                                                egui::Color32::from_rgb(255, 140, 140)
                                            } else {
                                                egui::Color32::from_rgb(190, 190, 190)
                                            };
                                            ui.label(egui::RichText::new(*line).font(mono.clone()).color(color));
                                        });
                                    }
                                });
                        },
                    );

                    // Vertical divider
                    let div_rect = ui.allocate_rect(
                        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(2.0, avail.y)),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(div_rect.rect, egui::Rounding::ZERO, egui::Color32::from_rgb(55, 55, 60));

                    // ── Right panel ──
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), avail.y),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            egui::ScrollArea::both()
                                .id_salt("diff_right")
                                .show(ui, |ui| {
                                    for (ri, line) in right_src_lines.iter().enumerate() {
                                        let is_diff = right_diff_set.contains(&ri);
                                        ui.horizontal(|ui| {
                                            // Gutter marker
                                            let (gr, _) = ui.allocate_exact_size(egui::vec2(gutter_w, line_h), egui::Sense::hover());
                                            if is_diff {
                                                ui.painter().circle_filled(gr.center(), 4.0, egui::Color32::from_rgb(100, 255, 100));
                                            }
                                            // Line number
                                            ui.label(egui::RichText::new(format!("{:>3}", ri + 1)).font(mono.clone()).size(11.0).color(egui::Color32::from_rgb(80, 80, 80)));
                                            // Text
                                            let color = if is_diff {
                                                egui::Color32::from_rgb(140, 255, 140)
                                            } else {
                                                egui::Color32::from_rgb(190, 190, 190)
                                            };
                                            ui.label(egui::RichText::new(*line).font(mono.clone()).color(color));
                                        });
                                    }
                                });
                        },
                    );
                });
            });
        self.show_diff = open;
    }

    // ── Find & Replace ───────────────────────────────────────────────
    fn render_find_replace(&mut self, ctx: &egui::Context) {
        egui::Window::new("Find & Replace").resizable(true).default_width(420.0).show(ctx, |ui| {
            egui::Grid::new("fr_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label("Find:");
                ui.add(egui::TextEdit::singleline(&mut self.find_text).desired_width(300.0));
                ui.end_row();
                ui.label("Replace:");
                ui.add(egui::TextEdit::singleline(&mut self.replace_text).desired_width(300.0));
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.replace_scope_all, true, "Current group");
                ui.radio_value(&mut self.replace_scope_all, false, "Selected only");
            });
            if !self.find_text.is_empty() {
                let mc: usize = self.curls.iter().filter(|c| self.replace_scope_all || c.selected).map(|c| c.command.matches(&self.find_text).count()).sum();
                let cc = self.curls.iter().filter(|c| self.replace_scope_all || c.selected).filter(|c| c.command.contains(&self.find_text)).count();
                ui.label(format!("{} matches in {} commands", mc, cc));
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let can = !self.find_text.is_empty() && self.curls.iter().filter(|c| self.replace_scope_all || c.selected).any(|c| c.command.contains(&self.find_text));
                if ui.add_enabled(can, egui::Button::new("Replace All")).clicked() {
                    let find = self.find_text.clone();
                    let replace = self.replace_text.clone();
                    for curl in &mut self.curls {
                        if self.replace_scope_all || curl.selected {
                            curl.command = curl_parser::replace_in_command(&curl.command, &find, &replace);
                        }
                    }
                    self.dirty = true;
                }
                if ui.button("Close").clicked() { self.show_find_replace = false; }
            });
        });
    }
}

// ── eframe::App ─────────────────────────────────────────────────────

impl eframe::App for CurlHelperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set dark theme once on first frame
        if !self.theme_initialized {
            ctx.set_visuals(egui::Visuals::dark());
            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
            self.theme_initialized = true;
        }

        self.check_results();

        // Only repaint periodically while tasks are running (not every frame)
        if !self.running.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }

        // Toolbar
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(12.0, 8.0)))
            .show(ctx, |ui| { self.render_toolbar(ui); });

        // Groups
        egui::SidePanel::left("groups_panel")
            .resizable(true).default_width(130.0).min_width(100.0).max_width(220.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| { self.render_groups_panel(ui); });

        // Commands
        egui::SidePanel::left("commands_panel")
            .resizable(true).default_width(620.0).min_width(350.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| { self.render_left_panel(ui); });

        // Right: results
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Results").size(15.0).strong());
                    if !self.current_batch.is_empty() {
                        let done = self.current_batch.iter().filter(|e| e.result.is_some()).count();
                        let total = self.current_batch.len();
                        ui.label(
                            egui::RichText::new(format!("({}/{})", done, total))
                                .size(13.0)
                                .color(egui::Color32::from_rgb(130, 130, 130)),
                        );
                    }
                });
                ui.add_space(4.0);
                self.render_results_panel(ui);
            });

        // Drag & drop overlay
        self.handle_drag_drop(ctx);

        // Find & Replace
        if self.show_find_replace { self.render_find_replace(ctx); }

        // Diff dialog
        if self.show_diff { self.render_diff_dialog(ctx); }

        self.auto_save();
    }
}
