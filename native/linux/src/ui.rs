use crate::{
    bluetooth::Command,
    cube::{Cube, Move},
    io::{self, adjusted, number, seconds, value_str},
    model::Model,
    render::{self, Animator},
};
use eframe::egui::{self, Color32, FontId, Margin, RichText, Stroke, Vec2};
use serde_json::{Value, json};
use std::time::Instant;

use crate::design::{self, Icon, display, font, medium, palette};

pub fn style(ctx: &egui::Context) {
    design::install_fonts(ctx);
    design::apply_theme(ctx, "paper");
}
pub struct App {
    pub(crate) shell: crate::native_ui::State,
    pub(crate) ctx: egui::Context,
    pub model: Model,
    pub(crate) panel: Option<crate::panel::Bridge>,
    pub page: String,
    pub animator: Animator,
    pub selected: String,
    pub variant: usize,
    pub(crate) case_analysis: bool,
    pub stage: String,
    pub(crate) search: String,
    pub(crate) training: Option<crate::training_ui::TrainingRun>,
    pub(crate) show_devices: bool,
    pub(crate) manual: bool,
    pub(crate) keyboard: crate::keyboard::Timer,
    pub(crate) history_session: String,
    pub(crate) history_kind: String,
    pub(crate) practice_metrics: Value,
    pub(crate) manual_value: String,
    pub(crate) review: Option<Value>,
    pub(crate) review_phase: Option<String>,
    pub(crate) review_reconstruction: bool,
    pub(crate) review_raw: bool,
    pub(crate) replay: Animator,
    pub(crate) replay_index: usize,
    pub(crate) playing: Option<Instant>,
    pub(crate) review_playhead: f64,
    pub(crate) review_step_cursor: Option<usize>,
    pub(crate) review_tab: usize,
    pub(crate) review_solver_key: String,
    pub(crate) notes: String,
    pub(crate) delete_confirm: bool,
    pub(crate) reset_confirm: bool,
    pub isolated: bool,
    pub(crate) background: bool,
    quit_requested: bool,
    pub(crate) window_visible: bool,
    pub(crate) copied_until: Option<Instant>,
    pub(crate) applied_theme: String,
}
impl App {
    pub fn new(ctx: egui::Context, isolated: bool) -> Self {
        style(&ctx);
        Self {
            shell: Default::default(),
            panel: (!isolated).then(|| crate::panel::Bridge::new(ctx.clone())),
            ctx: ctx.clone(),
            model: Model::new(ctx, isolated),
            page: "Practice".into(),
            animator: Animator::default(),
            selected: "pll-ua".into(),
            variant: 0,
            case_analysis: false,
            stage: "PLL".into(),
            search: String::new(),
            training: None,
            show_devices: false,
            manual: false,
            keyboard: Default::default(),
            history_session: "All sessions".into(),
            history_kind: "All attempts".into(),
            practice_metrics: Value::Null,
            manual_value: String::new(),
            review: None,
            review_phase: None,
            review_reconstruction: false,
            review_raw: false,
            replay: Animator::default(),
            replay_index: 0,
            playing: None,
            review_playhead: 0.,
            review_step_cursor: None,
            review_tab: 0,
            review_solver_key: String::new(),
            notes: String::new(),
            delete_confirm: false,
            reset_confirm: false,
            isolated,
            background: false,
            quit_requested: false,
            window_visible: true,
            copied_until: None,
            applied_theme: String::new(),
        }
    }
    pub fn draw(&mut self, ctx: &egui::Context) {
        self.handle_cube_controls();
        self.handle_background_window(ctx);
        crate::native_ui::draw(self, ctx);
    }
    pub(crate) fn handle_cube_controls(&mut self) {
        let commands = self
            .panel
            .as_ref()
            .map(|p| p.take_cube_controls())
            .unwrap_or_default();
        for command in commands {
            self.apply_cube_control(command);
        }
    }
    pub(crate) fn apply_cube_control(&mut self, command: crate::panel::CubeControl) {
        use crate::panel::{CubeControl, device_key};
        match command {
            CubeControl::Scan if !self.model.connected && !self.model.connecting => {
                self.model.scan()
            }
            CubeControl::Connect(key) => {
                let address = if key == "saved" {
                    (!self.model.settings.mac.is_empty()).then(|| self.model.settings.mac.clone())
                } else {
                    self.model
                        .devices
                        .iter()
                        .find(|d| device_key(&d.address) == key)
                        .map(|d| d.address.clone())
                };
                if let Some(address) = address {
                    self.model.connect(address);
                } else {
                    self.model.message = "Cube unavailable. Scan again.".into();
                }
            }
            CubeControl::Disconnect => self.model.disconnect(),
            CubeControl::PowerOn if !self.model.powered => self.model.command(Command::PowerOn),
            _ => {}
        }
    }
    pub(crate) fn quit(&mut self, ctx: &egui::Context) {
        self.quit_requested = true;
        self.background = false;
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
    }
    fn handle_background_window(&mut self, ctx: &egui::Context) {
        let quit_shortcut = ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Q));
        if self.quit_requested
            || quit_shortcut
            || self.panel.as_ref().is_some_and(|p| p.take_quit())
        {
            self.quit(ctx);
            return;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.keep_in_background(ctx);
            return;
        }
        if self.panel.as_ref().is_some_and(|p| p.take_show()) {
            self.window_visible = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        let hide = self.panel.as_ref().is_some_and(|p| p.take_hide());
        if hide {
            self.keep_in_background(ctx);
        }
    }
    pub(crate) fn keep_in_background(&mut self, ctx: &egui::Context) {
        self.background = true;
        self.window_visible = false;
        self.keyboard.cancel();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
    #[allow(dead_code)]
    pub fn draw_legacy(&mut self, ctx: &egui::Context) {
        self.model
            .capture_view_frame(self.animator.live_view_frame());
        self.model.tick();
        if self.model.connected {
            self.keyboard.cancel();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.keyboard.cancel();
            self.review = None;
            self.show_devices = false;
            self.manual = false;
            self.reset_confirm = false;
        }
        if self.page == "Practice"
            && !self.model.connected
            && !self.manual
            && self.review.is_none()
            && !self.show_devices
            && !self.reset_confirm
            && !ctx.wants_keyboard_input()
            && !ctx.input(|i| i.modifiers.any())
        {
            if ctx.input(|i| i.key_pressed(egui::Key::Space))
                && let Some(seconds) = self.keyboard.press(Instant::now())
            {
                self.model.manual(seconds);
            }
            if ctx.input(|i| i.key_released(egui::Key::Space)) {
                self.keyboard.release(Instant::now());
            }
        } else if self.keyboard.holding() {
            self.keyboard.cancel();
        }
        if let Some(cube) = self.model.visual_reset.take() {
            self.animator.snap(cube);
        }
        self.animator.set_orientation(self.model.orientation);
        self.model
            .capture_view_frame(self.animator.live_view_frame());
        for (mv, clock, recovered) in self.model.animation_moves.drain(..) {
            self.animator.push_timed(mv, clock, recovered);
        }
        if self.model.active.is_some()
            || self.model.countdown.is_some()
            || self.keyboard.running()
            || self.keyboard.holding()
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        if self.applied_theme != self.model.settings.theme {
            design::apply_theme(ctx, &self.model.settings.theme);
            self.applied_theme = self.model.settings.theme.clone();
        }
        let p = palette(ctx);
        let compact = ctx.content_rect().width() < 900.;
        let solving = self.keyboard.running()
            || self
                .model
                .active
                .as_ref()
                .is_some_and(|a| a.started.is_some());
        if compact {
            egui::TopBottomPanel::bottom("mobile-navigation")
                .frame(
                    egui::Frame::new()
                        .fill(p.bg)
                        .inner_margin(Margin::symmetric(12, 5)),
                )
                .show(ctx, |ui| {
                    ui.set_height(56.);
                    if !solving {
                        self.navigation(ui, true);
                    }
                });
        }
        egui::TopBottomPanel::top("toolbar")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(
                        if compact { 20 } else { 48 },
                        if compact { 13 } else { 30 },
                    )),
            )
            .show(ctx, |ui| {
                let width = ui.available_width().min(1040.);
                let rect = egui::Rect::from_center_size(
                    egui::pos2(
                        ui.available_rect_before_wrap().center().x,
                        ui.cursor().top() + 17.,
                    ),
                    Vec2::new(width, 34.),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Facet").font(medium(20.)).color(if solving {
                                        p.faint
                                    } else {
                                        p.text
                                    }),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.page = "Practice".into();
                        }
                        if !compact && !solving {
                            ui.add_space(34.);
                            self.navigation(ui, false);
                        }
                        if !solving {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if design::icon_button(ui, Icon::Settings, "Settings").clicked()
                                    {
                                        self.page = "Settings".into();
                                    }
                                    let label = if self.model.connected {
                                        format!(
                                            "{}{}",
                                            self.model.device_name,
                                            self.model
                                                .battery
                                                .map(|b| format!(" · {b}%"))
                                                .unwrap_or_default()
                                        )
                                    } else if self.model.connecting {
                                        "Connecting…".into()
                                    } else if self.model.powered {
                                        "Connect cube".into()
                                    } else {
                                        "Bluetooth off".into()
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(label).font(font(11.)).color(p.muted),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        if self.model.connected {
                                            self.page = "Settings".into();
                                        } else {
                                            self.show_devices = true;
                                            if self.model.powered {
                                                self.model.scan();
                                            }
                                        }
                                    }
                                    dot(
                                        ui,
                                        if self.model.connected {
                                            p.green
                                        } else if self.model.powered {
                                            p.faint
                                        } else {
                                            p.amber
                                        },
                                    );
                                },
                            );
                        }
                    })
                });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(if compact { 20 } else { 48 }, 4)),
            )
            .show(ctx, |ui| {
                if self.page == "Practice" {
                    self.practice(ui);
                } else if self.page == "History" {
                    self.history_view(ui);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.add_space(18.);
                            let title = match self.page.as_str() {
                                "Library" => "Algorithms",
                                "Training" => "Training",
                                "Insights" => "Statistics",
                                "Case" => "Case study",
                                _ => "Settings",
                            };
                            ui.label(RichText::new(title).font(font(25.)).color(p.text));
                            ui.add_space(22.);
                            if !self.model.message.is_empty() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new(self.model.message.clone())
                                            .font(font(12.))
                                            .color(p.muted),
                                    );
                                    if ui.small_button("Dismiss").clicked() {
                                        self.model.message.clear();
                                    }
                                });
                                ui.add_space(12.);
                            }
                            match self.page.as_str() {
                                "Library" => self.case_grid(ui),
                                "Training" => self.training_page(ui),
                                "Case" => self.case_detail(ui),
                                "Insights" => self.insights_view(ui),
                                _ => self.settings(ui),
                            }
                        });
                }
            });
        self.dialogs(ctx);
    }
    fn navigation(&mut self, ui: &mut egui::Ui, compact: bool) {
        let pages = [
            ("Practice", "timer", Icon::Timer),
            ("Training", "train", Icon::Train),
            ("Library", "algorithms", Icon::Library),
            ("Insights", "insights", Icon::Chart),
            ("History", "solves", Icon::History),
        ];
        let p = palette(ui.ctx());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = if compact { 4. } else { 14. };
            let width = if compact {
                (ui.available_width() - 16.) / 5.
            } else {
                0.
            };
            for (page, label, icon) in pages {
                let selected = self.page == page || (page == "Library" && self.page == "Case");
                if compact {
                    if design::nav(ui, label, icon, selected, true, width).clicked() {
                        self.page = page.into();
                    }
                } else {
                    let color = if selected { p.accent } else { p.faint };
                    let galley = ui.painter().layout_no_wrap(label.into(), font(12.), color);
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(galley.size().x + 6., 30.),
                        egui::Sense::click(),
                    );
                    ui.painter().galley(
                        egui::pos2(rect.left() + 3., rect.center().y - galley.size().y / 2.),
                        galley,
                        if response.hovered() { p.text } else { color },
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::SelectableLabel, true, label)
                    });
                    if response.clicked()
                        || response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        self.page = page.into();
                    }
                    response.on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            }
        });
    }
    pub(crate) fn bluetooth_button(&mut self, ui: &mut egui::Ui) {
        if !self.model.powered {
            ui.horizontal(|ui| {
                dot(ui, palette(ui.ctx()).amber);
                caption(ui, &self.model.adapter_status);
                if ui.button("Turn on Bluetooth").clicked() {
                    self.model.command(Command::PowerOn);
                }
            });
        } else if self.model.connected {
            if ui.button("Disconnect").clicked() {
                self.model.disconnect();
            }
        } else if ui
            .add_enabled(
                !self.model.connecting,
                egui::Button::new(if self.model.connecting {
                    "Connecting…"
                } else {
                    "Connect cube"
                }),
            )
            .clicked()
        {
            self.show_devices = true;
            self.model.scan();
        }
    }
    pub(crate) fn connect_dialog(&mut self) {
        self.show_devices = true;
        self.model.scan();
    }
    fn appearance(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        ui.label(RichText::new("Appearance").font(medium(14.)));
        ui.add_space(12.);
        ui.columns(2, |columns| {
            for (i, (key, name)) in [("carbon", "Dark"), ("paper", "Light")].iter().enumerate() {
                let c = design::colors(key);
                let ui = &mut columns[i];
                let selected = self.model.settings.theme == *key
                    || (*key == "graphite"
                        && !matches!(self.model.settings.theme.as_str(), "snappy" | "paper"));
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 76.),
                    egui::Sense::click(),
                );
                ui.painter().rect_filled(rect, 3, c.bg);
                ui.painter().rect_stroke(
                    rect,
                    3,
                    Stroke::new(
                        if selected { 2.0f32 } else { 1.0f32 },
                        if selected { p.accent } else { p.line },
                    ),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    rect.left_top() + Vec2::new(16., 12.),
                    egui::Align2::LEFT_TOP,
                    "Aa",
                    display(24.),
                    c.text,
                );
                ui.painter().text(
                    rect.left_bottom() + Vec2::new(16., -17.),
                    egui::Align2::LEFT_CENTER,
                    name,
                    medium(14.),
                    c.text,
                );
                if selected {
                    design::icon(
                        ui.painter(),
                        egui::Rect::from_center_size(
                            rect.right_bottom() + Vec2::new(-17., -17.),
                            Vec2::splat(14.),
                        ),
                        Icon::Check,
                        c.accent,
                    );
                }
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::SelectableLabel, true, *name)
                });
                if response.clicked() {
                    self.model.set_theme(key);
                    design::apply_theme(ui.ctx(), key);
                }
            }
        });
    }
    pub(crate) fn case_detail(&mut self, ui: &mut egui::Ui) {
        let Some(case) = self
            .model
            .catalog
            .cases
            .iter()
            .find(|c| c.id == self.selected)
            .cloned()
        else {
            return;
        };
        self.variant = self.variant.min(case.variants.len() - 1);
        let variant = case.variants[self.variant].clone();
        if ui.button("← Library").clicked() {
            self.page = "Library".into();
        }
        ui.add_space(16.);
        ui.horizontal(|ui| {
            crate::training_ui::case_pattern(
                ui,
                Cube::parse(&variant.positions["D"].state).unwrap(),
                &case.stage,
                96.,
            );
            ui.vertical(|ui| {
                caption(ui, &case.stage);
                ui.heading(&case.name);
                let favorite = self.model.preferences[&case.id]["favorite"] == true;
                if ui
                    .button(if favorite { "Favorited" } else { "Favorite" })
                    .clicked()
                {
                    if !self.model.preferences[&case.id].is_object() {
                        self.model.preferences[&case.id] = json!({"variant":self.variant});
                    }
                    self.model.preferences[&case.id]["favorite"] = json!(!favorite);
                    self.model.update_preferences();
                }
            });
        });
        ui.add_space(12.);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.case_analysis, false, "Algorithms");
            ui.selectable_value(&mut self.case_analysis, true, "Analysis");
        });
        ui.add_space(12.);
        if !self.case_analysis {
            heading(ui, "Algorithm");
            ui.label(RichText::new(&variant.text).font(FontId::monospace(23.)));
            ui.add_space(14.);
            ui.horizontal_wrapped(|ui| {
                for i in 0..case.variants.len() {
                    if ui
                        .selectable_label(i == self.variant, format!("Variant {}", i + 1))
                        .clicked()
                    {
                        self.variant = i;
                        if !self.model.preferences[&case.id].is_object() {
                            self.model.preferences[&case.id] = json!({});
                        }
                        self.model.preferences[&case.id]["variant"] = json!(i);
                        self.model.update_preferences();
                    }
                }
            });
            ui.add_space(14.);
            if primary(ui, "Practice this case").clicked() {
                self.start_training(&case.id, self.variant);
            }
            ui.horizontal_wrapped(|ui| {
                caption(ui, "Learning status");
                for (value, label) in [
                    ("new", "New"),
                    ("learning", "Learning"),
                    ("confident", "Learned"),
                ] {
                    let current = self.model.preferences[&case.id]["status"]
                        .as_str()
                        .unwrap_or("new");
                    if ui.selectable_label(current == value, label).clicked() {
                        if !self.model.preferences[&case.id].is_object() {
                            self.model.preferences[&case.id] = json!({});
                        }
                        self.model.preferences[&case.id]["status"] = json!(value);
                        self.model.update_preferences();
                    }
                }
            });
            return;
        }
        heading(ui, "Your results");
        let performance = self
            .model
            .stats
            .iter()
            .find(|p| value_str(p, "caseId") == case.id)
            .cloned()
            .or_else(|| crate::case_analysis::local_performance(&self.model.records, &case));
        if performance.is_none() {
            egui::Frame::new()
                .fill(palette(ui.ctx()).surface)
                .corner_radius(12)
                .inner_margin(20)
                .show(ui, |ui| {
                    ui.label(RichText::new("No measured attempts").font(medium(14.)));
                    if ui.button("Practice").clicked() {
                        self.start_training(&case.id, self.variant);
                    }
                });
            return;
        }
        let stats = performance.as_ref().map(crate::case_analysis::summary);
        ui.columns(3, |c| {
            metric(
                &mut c[0],
                "Mean recognition",
                stats
                    .as_ref()
                    .and_then(|s| s["meanRecognition"].as_f64())
                    .map(seconds)
                    .unwrap_or("—".into()),
                &format!(
                    "{} cue samples",
                    stats
                        .as_ref()
                        .map(|s| number(s, "recognitionCount") as usize)
                        .unwrap_or(0)
                ),
            );
            metric(
                &mut c[1],
                "Mean execution",
                stats
                    .as_ref()
                    .and_then(|s| s["meanExecution"].as_f64())
                    .map(seconds)
                    .unwrap_or("—".into()),
                &format!(
                    "{} executions",
                    stats
                        .as_ref()
                        .map(|s| number(s, "count") as usize)
                        .unwrap_or(0)
                ),
            );
            let samples = stats
                .as_ref()
                .and_then(|s| s["samples"].as_array())
                .cloned()
                .unwrap_or_default();
            let delta = if samples.len() >= 10 {
                Some(
                    mean(&samples[samples.len() - 5..], "execution")
                        - mean(&samples[..5], "execution"),
                )
            } else {
                None
            };
            metric(
                &mut c[2],
                "Change",
                delta
                    .map(|d| format!("{}{:0.2}", if d > 0. { "+" } else { "" }, d / 1000.))
                    .unwrap_or("—".into()),
                "last 5 vs first 5 · seconds",
            );
        });
        ui.add_space(15.);
        let samples = stats
            .as_ref()
            .and_then(|s| s["samples"].as_array())
            .cloned()
            .unwrap_or_default();
        render::chart(
            ui,
            &samples
                .iter()
                .map(|s| number(s, "execution"))
                .collect::<Vec<_>>(),
            170.,
        );
        if let Some(p) = performance {
            heading(ui, "Algorithms you actually use");
            if let Some(algorithms) = p["algorithms"].as_array() {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    egui::Grid::new("alg-stats")
                        .min_col_width(125.)
                        .spacing(Vec2::new(20., 16.))
                        .show(ui, |ui| {
                            for label in ["Algorithm", "Executions", "Recognition", "Execution"] {
                                caption(ui, label);
                            }
                            ui.end_row();
                            for a in algorithms {
                                ui.label(value_str(a, "name"));
                                ui.label(format!("{}", number(a, "count") as usize));
                                ui.label(
                                    a["meanRecognition"]
                                        .as_f64()
                                        .map(seconds)
                                        .unwrap_or("—".into()),
                                );
                                ui.label(seconds(number(a, "meanExecution")));
                                ui.end_row();
                            }
                        });
                });
            }
            if p["suggestion"].is_object() {
                heading(
                    ui,
                    if p["suggestion"]["kind"] == "measured" {
                        "Measured alternative"
                    } else {
                        "Next experiment"
                    },
                );
                ui.label(value_str(&p["suggestion"], "title"));
                caption(ui, value_str(&p["suggestion"], "reason"));
                if let Some(i) = case
                    .variants
                    .iter()
                    .position(|v| v.id == value_str(&p["suggestion"], "algorithmId"))
                    && ui.button("Try this variant").clicked()
                {
                    self.variant = i;
                    self.model.prepare_case(&case.id, i);
                    self.page = "Practice".into();
                }
            }
        } else {
            caption(ui, "Complete five repetitions to establish a baseline.");
        }
        ui.add_space(24.);
        caption(
            ui,
            "Recognition includes reaction time after the cue. Full-solve transition gaps are measured separately. Recovered timestamps are excluded from these averages.",
        );
    }
    pub(crate) fn settings(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        ui.spacing_mut().item_spacing = Vec2::new(8., 8.);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Settings").font(medium(18.)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("Quit Facet").font(medium(12.)))
                    .on_hover_text("Ctrl+Q · stop Facet, including background tracking")
                    .clicked()
                {
                    self.quit(ui.ctx());
                }
                if ui
                    .button(RichText::new("Keep in background").font(font(12.)))
                    .on_hover_text("Hide the timer and keep Bluetooth tracking running")
                    .clicked()
                {
                    self.keep_in_background(ui.ctx());
                }
            });
        });
        ui.add_space(24.);
        self.appearance(ui);
        settings_section(ui, "Cube");
        self.bluetooth_button(ui);
        ui.add_space(10.);
        ui.collapsing("Manual address", |ui| {
            design::field(ui, "Bluetooth address", &mut self.model.settings.mac, false);
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.model.powered && !self.model.connected,
                    egui::Button::new("Connect saved"),
                )
                .clicked()
            {
                self.model.connect(self.model.settings.mac.clone());
            }
            if ui
                .add_enabled(self.model.connected, egui::Button::new("Read state"))
                .clicked()
            {
                self.model.read_state();
            }
            if ui
                .add_enabled(self.model.connected, egui::Button::new("Calibrate solved"))
                .clicked()
            {
                self.reset_confirm = true;
            }
        });
        settings_section(ui, "Practice");
        design::field(ui, "Session", &mut self.model.settings.session, false);
        caption(ui, "Cross color");
        let colors = [
            ("U", "White"),
            ("D", "Yellow"),
            ("F", "Green"),
            ("B", "Blue"),
            ("R", "Red"),
            ("L", "Orange"),
        ];
        let columns = if ui.available_width() < 460. { 3 } else { 6 };
        for row in colors.chunks(columns) {
            ui.columns(columns, |cells| {
                for (cell, (face, name)) in cells.iter_mut().zip(row.iter()) {
                    if cell
                        .add_sized(
                            Vec2::new(cell.available_width(), 32.),
                            egui::Button::selectable(self.model.settings.cross == *face, *name),
                        )
                        .clicked()
                    {
                        self.model.settings.cross = face.to_string();
                    }
                }
            });
        }
        ui.label(
            RichText::new("For drills; full solves detect the cross.")
                .font(font(12.))
                .color(p.faint),
        );
        settings_section(ui, "Sync");
        caption(ui, &self.model.sync_status);
        ui.collapsing("Connection details", |ui| {
            design::field(
                ui,
                "Server endpoint",
                &mut self.model.settings.endpoint,
                false,
            );
            design::field(ui, "Access key", &mut self.model.settings.token, true);
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Import connection").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Connection", &["json"])
                    .pick_file()
            {
                match std::fs::read(&path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<io::Settings>(&b).ok())
                {
                    Some(s) if ["U", "D", "R", "L", "F", "B"].contains(&s.cross.as_str()) => {
                        self.model.settings = s;
                        self.model.save_settings();
                    }
                    _ => self.model.message = "Invalid connection file".into(),
                }
            }
            if ui.button("Sync now").clicked() {
                self.model.sync();
            }
        });
        settings_section(ui, "Backups");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export backup").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_file_name("facet-backup.json")
                    .save_file()
            {
                match io::atomic_write(
                    &path,
                    &serde_json::to_vec_pretty(&self.model.backup()).unwrap(),
                ) {
                    Ok(()) => self.model.message = "Backup exported".into(),
                    Err(e) => self.model.message = e.to_string(),
                }
            }
            if ui.button("Import backup").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("csTimer or native backup", &["json"])
                    .pick_file()
                && let Ok(bytes) = std::fs::read(path)
                && bytes.len() <= 25 * 1024 * 1024
                && let Ok(v) = serde_json::from_slice(&bytes)
            {
                self.model.import_backup(v);
            }
        });
        ui.add_space(24.);
        if primary(ui, "Save changes").clicked() {
            self.model.save_settings();
        }
        ui.add_space(16.);
        ui.collapsing("About & licenses", |ui| {
            caption(ui, "Facet · Rust / BlueZ");
            caption(ui, "GAN protocol: gan-web-bluetooth · MIT");
            caption(ui, "Algorithms: Tao Yu / Alg-Trainer · MIT");
        });
    }
    pub(crate) fn open_review(&mut self, r: Value) {
        let r = crate::recording::for_review(r);
        {
            use std::hash::{Hash, Hasher};
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            r.to_string().hash(&mut hash);
            self.review_solver_key = format!("reconstruction:{:x}", hash.finish());
        }
        self.replay
            .snap(Cube::parse(value_str(&r, "initialState")).unwrap_or_default());
        self.replay
            .set_replay_cross(crate::recording::reconstruction_cross(&r));
        self.replay.set_recorded_orientation(&r, 0., false);
        self.replay_index = 0;
        self.review_playhead = 0.;
        self.review_tab = 0;
        self.review_step_cursor = None;
        self.playing = None;
        self.notes = value_str(&r, "notes").into();
        self.delete_confirm = false;
        self.review = Some(r);
        self.review_phase = None;
        self.review_reconstruction = false;
        self.review_raw = false;
    }
    pub(crate) fn toggle_review_playback(&mut self) {
        self.review_step_cursor = None;
        let Some(record) = self.review.as_ref() else {
            return;
        };
        let count = record["moves"].as_array().map_or(0, Vec::len);
        if count == 0 {
            return;
        }
        if let Some(start) = self.playing.take() {
            self.review_playhead =
                (start.elapsed().as_secs_f64() * 1000.).min(number(record, "duration"));
            return;
        }
        if self.replay_index >= count {
            self.replay_index = 0;
            self.review_playhead = 0.;
            self.replay
                .snap(Cube::parse(value_str(record, "initialState")).unwrap_or_default());
        }
        self.playing = Instant::now().checked_sub(std::time::Duration::from_secs_f64(
            self.review_playhead.clamp(0., 86_400_000.) / 1000.,
        ));
    }
    pub(crate) fn open_phase_review(&mut self, record: Value, phase: &str) {
        self.open_review(record);
        self.seek_review_phase(phase);
    }
    pub(crate) fn seek_review_time(&mut self, time: f64) {
        if !time.is_finite() || time < 0. {
            return;
        }
        let index = self
            .review
            .as_ref()
            .and_then(|r| r["moves"].as_array())
            .map(|moves| {
                moves
                    .iter()
                    .take_while(|m| m["t"].as_f64().is_some_and(|t| t <= time))
                    .count()
            })
            .unwrap_or(0);
        self.seek_review_turn(index);
        self.review_playhead = time;
    }
    pub(crate) fn seek_review_turn(&mut self, index: usize) {
        self.review_step_cursor = None;
        let Some(record) = self.review.as_ref() else {
            return;
        };
        let Some(moves) = record["moves"].as_array() else {
            return;
        };
        let index = index.min(moves.len());
        let start = Cube::parse(value_str(record, "initialState")).unwrap_or_default();
        let turns = moves[..index]
            .iter()
            .filter_map(|event| Move::parse(value_str(event, "move")).ok())
            .collect::<Vec<_>>();
        self.replay.snap(start.apply(&turns));
        self.replay_index = index;
        self.review_playhead = crate::recording::turn_elapsed(record, index);
        self.playing = None;
    }
    pub(crate) fn seek_review_phase(&mut self, phase: &str) {
        self.review_step_cursor = None;
        let Some(record) = self.review.as_ref() else {
            return;
        };
        let Some(index) = phase_start_index(record, phase) else {
            return;
        };
        let moves = record["moves"].as_array().cloned().unwrap_or_default();
        let start = Cube::parse(value_str(record, "initialState")).unwrap_or_default();
        let turns = moves[..index]
            .iter()
            .filter_map(|event| Move::parse(value_str(event, "move")).ok())
            .collect::<Vec<_>>();
        self.replay.snap(start.apply(&turns));
        self.replay_index = index;
        self.review_playhead = crate::recording::turn_elapsed(record, index);
        self.playing = None;
        self.review_phase = Some(phase.into());
    }
    pub(crate) fn step_review(&mut self, forward: bool) {
        let Some(record) = self.review.as_ref() else {
            return;
        };
        let positions = crate::review_navigation::positions(record, self.review_raw);
        let time = self
            .playing
            .map(|start| start.elapsed().as_secs_f64() * 1000.)
            .unwrap_or(self.review_playhead);
        let current = crate::review_navigation::Position {
            raw_index: self.replay_index,
            time,
        };
        let cursor = crate::review_navigation::adjacent(
            &positions,
            current,
            self.review_step_cursor,
            forward,
        );
        let target = positions[cursor];
        self.seek_review_turn(target.raw_index);
        self.review_playhead = target.time;
        self.review_step_cursor = Some(cursor);
    }
    fn handle_review_shortcuts(&mut self, ctx: &egui::Context) {
        if self.review.is_some() && !self.delete_confirm {
            let toggle=ctx.input(|i|i.events.iter().any(|e|matches!(e,egui::Event::Key{key:egui::Key::Space,pressed:true,repeat:false,modifiers,..} if !modifiers.any())));
            let left = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft) && !i.modifiers.any());
            let right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) && !i.modifiers.any());
            ctx.input_mut(|i| {
                for key in [
                    egui::Key::Space,
                    egui::Key::ArrowLeft,
                    egui::Key::ArrowRight,
                ] {
                    i.consume_key(egui::Modifiers::NONE, key);
                }
            });
            if left {
                self.step_review(false);
            } else if right {
                self.step_review(true);
            } else if toggle {
                self.toggle_review_playback();
            }
        }
    }
    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        self.handle_review_shortcuts(ctx);
        if self.show_devices {
            let p = palette(ctx);
            let width = (ctx.content_rect().width() - 64.).clamp(240., 360.);
            let response = egui::Modal::new(egui::Id::new("cube-connection"))
                .backdrop_color(Color32::from_black_alpha(100))
                .frame(
                    egui::Frame::new()
                        .fill(p.surface)
                        .stroke(Stroke::new(1., p.line))
                        .corner_radius(8)
                        .inner_margin(24),
                )
                .show(ctx, |ui| {
                    ui.set_width(width);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Your cube").font(medium(20.)).color(p.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(RichText::new("×").font(font(20.)))
                                .on_hover_text("Close")
                                .clicked()
                            {
                                self.show_devices = false;
                            }
                        });
                    });
                    ui.add_space(20.);
                    ui.horizontal(|ui| {
                        dot(
                            ui,
                            if self.model.connected {
                                p.green
                            } else if !self.model.powered {
                                p.amber
                            } else {
                                p.accent
                            },
                        );
                        ui.label(
                            RichText::new(if !self.model.powered {
                                "Bluetooth is off"
                            } else if self.model.connected {
                                "Connected"
                            } else if self.model.connecting {
                                "Connecting…"
                            } else if self.model.scanning {
                                "Finding your cube…"
                            } else {
                                "Nearby cubes"
                            })
                            .font(medium(14.)),
                        );
                        if self.model.scanning {
                            ui.spinner();
                        }
                    });
                    ui.add_space(8.);
                    ui.label(
                        RichText::new(if !self.model.powered {
                            "Enable Bluetooth to find your cube."
                        } else if self.model.connected {
                            "Ready for your next solve."
                        } else {
                            "Wake your cube with a turn. Close other cube apps."
                        })
                        .font(font(14.))
                        .color(p.muted),
                    );
                    ui.add_space(20.);
                    if !self.model.powered {
                        if primary(ui, "Turn on Bluetooth").clicked() {
                            self.model.command(Command::PowerOn);
                        }
                    } else if self.model.connected {
                        ui.label(RichText::new(&self.model.device_name).font(medium(16.)));
                        ui.add_space(16.);
                        if ui.button("Disconnect").clicked() {
                            self.model.disconnect();
                        }
                    } else {
                        for d in self.model.devices.clone() {
                            let response = egui::Frame::new()
                                .fill(p.raised)
                                .stroke(Stroke::new(1., p.line))
                                .corner_radius(4)
                                .inner_margin(14)
                                .show(ui, |ui| {
                                    ui.set_min_width(width - 28.);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&d.name).font(medium(16.)));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new("Pair  →")
                                                        .font(medium(14.))
                                                        .color(p.accent),
                                                );
                                            },
                                        );
                                    });
                                })
                                .response
                                .interact(egui::Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            response.clone().on_hover_text(&d.address);
                            if response.clicked() && !self.model.connecting {
                                self.model.connect(d.address);
                            }
                            ui.add_space(8.);
                        }
                        if self.model.devices.is_empty()
                            && !self.model.scanning
                            && !self.model.connecting
                        {
                            caption(ui, "No cubes found. Keep yours nearby and try again.");
                            ui.add_space(16.);
                        }
                        if ui
                            .add_enabled(
                                !self.model.scanning && !self.model.connecting,
                                egui::Button::new(RichText::new("Scan again").font(medium(14.)))
                                    .min_size(Vec2::new(width, 36.)),
                            )
                            .clicked()
                        {
                            self.model.scan();
                        }
                    }
                });
            if response.should_close() {
                self.show_devices = false;
            }
        }
        if self.manual {
            let mut open = true;
            egui::Window::new("Enter a time")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    caption(ui, "Seconds");
                    ui.text_edit_singleline(&mut self.manual_value);
                    if primary(ui, "Save time").clicked()
                        && let Ok(v) = self.manual_value.parse()
                    {
                        self.model.manual(v);
                        self.manual = false;
                        self.manual_value.clear();
                    }
                });
            self.manual &= open;
        }
        if self.reset_confirm {
            egui::Window::new("Reset cube")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("All six faces must be physically solved.");
                    caption(
                        ui,
                        "This marks the connected cube as solved. It does not turn the cube.",
                    );
                    if self.model.active.is_some() {
                        caption(
                            ui,
                            "The current attempt will end; its recorded turns will be kept.",
                        );
                    }
                    if !self.model.connected {
                        caption(ui, "Reconnect your cube to reset its state.");
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.reset_confirm = false;
                        }
                        if ui
                            .add_enabled(
                                self.model.connected,
                                egui::Button::new("Cube is solved · reset"),
                            )
                            .clicked()
                        {
                            self.model.interrupt("cube-reset");
                            self.keyboard.cancel();
                            self.model.calibrate();
                            self.reset_confirm = false;
                        }
                    });
                });
        }
        if let Some(r) = self.review.clone() {
            crate::review_ui::draw(self, ctx, &r);
        }
    }
}
// Prefer recorded event counts to timestamps: two packets can have the same
// timestamp on a phase boundary. Older imports without counts use time instead.
fn phase_start_index(record: &Value, phase: &str) -> Option<usize> {
    let events = record["moves"].as_array()?;
    let splits = record["analysis"]["splits"].as_array()?;
    let selected = splits
        .iter()
        .position(|split| value_str(split, "stage") == phase)?;
    if selected == 0 {
        return Some(0);
    }
    let counts = splits
        .iter()
        .map(|split| {
            split["rawMoves"]
                .as_u64()
                .or_else(|| split["moves"].as_u64())
        })
        .collect::<Option<Vec<_>>>();
    if let Some(counts) = counts
        && counts.iter().sum::<u64>() == events.len() as u64
    {
        return Some(counts[..selected].iter().sum::<u64>() as usize);
    }
    let start = splits[selected]["start"]
        .as_f64()
        .filter(|t| t.is_finite() && *t >= 0.)?;
    Some(
        events
            .iter()
            .take_while(|event| number(event, "t") <= start)
            .count(),
    )
}

#[cfg(test)]
mod phase_review_tests {
    use super::*;

    #[test]
    fn raw_turn_review_reconstructs_and_seeks_without_analysis() {
        let mut app = App::new(egui::Context::default(), true);
        let initial = Cube::default()
            .turn(Move::parse("R").unwrap())
            .turn(Move::parse("U").unwrap());
        app.open_review(json!({
            "source":"bluetooth", "status":"complete", "quality":[],
            "duration":1000., "cross":"U", "initialState":initial.to_string(),
            "moves":[{"move":"U'","t":0.},{"move":"R'","t":1000.}]
        }));
        assert_eq!(app.review.as_ref().unwrap()["analysis"]["moves"], 2);
        app.seek_review_turn(1);
        assert_eq!(app.replay_index, 1);
        assert!(app.replay.cube == initial.turn(Move::parse("U'").unwrap()));
        app.seek_review_turn(99);
        assert_eq!(app.replay_index, 2);
        assert!(app.replay.cube == Cube::default());
        app.seek_review_turn(0);
        assert!(app.replay.cube == initial);
        assert!(app.playing.is_none());
    }

    #[test]
    fn phase_seek_uses_event_boundary_even_with_equal_timestamps() {
        let record = json!({
            "moves": [{"t":0}, {"t":100}, {"t":100}, {"t":200}],
            "analysis": {"splits": [
                {"stage":"Cross", "start":0, "moves":2},
                {"stage":"F2L", "start":100, "moves":1},
                {"stage":"OLL", "start":100, "moves":0},
                {"stage":"PLL", "start":100, "moves":1}
            ]}
        });
        assert_eq!(phase_start_index(&record, "Cross"), Some(0));
        assert_eq!(phase_start_index(&record, "F2L"), Some(2));
        assert_eq!(phase_start_index(&record, "PLL"), Some(3));
        assert_eq!(phase_start_index(&record, "Unknown"), None);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.draw(ctx);
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        render::destroy_glow(&self.ctx);
        if !self.isolated {
            self.model.shutdown();
        }
    }
}
pub(crate) fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(font(12.))
            .color(palette(ui.ctx()).muted),
    );
}
fn settings_section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(20.);
    ui.separator();
    ui.add_space(12.);
    ui.label(
        RichText::new(label)
            .font(medium(14.))
            .color(palette(ui.ctx()).text),
    );
    ui.add_space(4.);
}
pub(crate) fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(20.);
    ui.label(
        RichText::new(text)
            .font(medium(14.))
            .color(palette(ui.ctx()).text),
    );
    ui.add_space(6.);
}
pub(crate) fn metric(ui: &mut egui::Ui, label: &str, value: String, detail: &str) {
    caption(ui, label);
    ui.label(RichText::new(value).font(design::mono_bold(22.)));
    caption(ui, detail);
}
pub(crate) fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    design::primary(ui, text)
}
pub(crate) fn dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(7.), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3., color);
}
pub(crate) fn mean(v: &[Value], key: &str) -> f64 {
    v.iter().map(|s| number(s, key)).sum::<f64>() / v.len().max(1) as f64
}

#[cfg(test)]
mod review_playback_tests {
    use super::*;
    #[test]
    fn companion_controls_validate_devices_and_keep_the_window_hidden() {
        use crate::panel::{CubeControl, device_key};
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        app.keep_in_background(&ctx);
        app.apply_cube_control(CubeControl::Scan);
        assert!(!app.model.scanning, "Scanning needs a powered adapter");
        app.model.powered = true;
        app.apply_cube_control(CubeControl::Connect("unknown".into()));
        assert!(!app.model.connecting);
        app.apply_cube_control(CubeControl::Scan);
        assert!(app.model.scanning);
        let address = "00:00:00:00:00:01".to_string();
        app.model.devices = vec![crate::bluetooth::DeviceInfo {
            address: address.clone(),
            name: "Synthetic cube".into(),
        }];
        app.apply_cube_control(CubeControl::Connect(device_key(&address)));
        assert!(app.model.connecting);
        assert_eq!(app.model.settings.mac, address);
        app.apply_cube_control(CubeControl::Disconnect);
        assert!(!app.model.connecting);
        app.apply_cube_control(CubeControl::Connect("saved".into()));
        assert!(app.model.connecting);
        assert!(!app.window_visible);
        assert!(app.background);
        let snapshot = crate::native_ui::companion_snapshot(&app);
        assert_eq!(snapshot["connection"]["saved"], true);
        assert_eq!(
            snapshot["connection"]["devices"][0]["id"],
            device_key(&address)
        );
        assert!(!snapshot.to_string().contains(&address));
    }

    #[test]
    fn window_close_keeps_companion_running() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        app.background = true;
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let output = ctx.run(input, |ctx| app.handle_background_window(ctx));
        assert!(app.background);
        assert!(!app.window_visible);
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(!commands.contains(&egui::ViewportCommand::Close));
        assert!(commands.contains(&egui::ViewportCommand::CancelClose));
        assert!(commands.contains(&egui::ViewportCommand::Visible(false)));
    }

    #[test]
    fn explicit_quit_is_not_cancelled_by_window_close_handling() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        app.quit(&ctx);
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let output = ctx.run(input, |ctx| app.handle_background_window(ctx));
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.contains(&egui::ViewportCommand::Close));
        assert!(!commands.contains(&egui::ViewportCommand::CancelClose));
    }

    #[test]
    fn ctrl_q_exits_instead_of_hiding_the_daemon() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        app.background = true;
        let mut input = egui::RawInput::default();
        input.modifiers = egui::Modifiers::CTRL;
        input.events.push(egui::Event::Key {
            key: egui::Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        });
        let output = ctx.run(input, |ctx| app.handle_background_window(ctx));
        assert!(!app.background);
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.contains(&egui::ViewportCommand::Close));
        assert!(!commands.contains(&egui::ViewportCommand::CancelClose));
    }

    #[test]
    fn space_toggles_once_and_does_not_leak_to_focused_controls() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        app.open_review(json!({"id":"space-test","kind":"solve","source":"bluetooth","status":"complete","initialState":Cube::default().to_string(),"duration":3000.,"moves":[{"move":"R","t":0},{"move":"U","t":3000}],"quality":[]}));
        for (repeat, expected) in [(false, true), (true, true), (false, false)] {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat,
                modifiers: egui::Modifiers::NONE,
            });
            let _ = ctx.run(input, |ctx| {
                app.handle_review_shortcuts(ctx);
                assert!(!ctx.input(|i| i.key_pressed(egui::Key::Space)));
            });
            assert_eq!(app.playing.is_some(), expected);
            assert!(!app.keyboard.running());
            if repeat {
                let mut release = egui::RawInput::default();
                release.events.push(egui::Event::Key {
                    key: egui::Key::Space,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
                let _ = ctx.run(release, |ctx| app.handle_review_shortcuts(ctx));
            }
        }
    }
    #[test]
    fn arrow_steps_pause_playback_and_rebuild_exact_grouped_or_raw_states() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        let events = [
            ("R", 0.),
            ("L'", 60.),
            ("R", 120.),
            ("L'", 180.),
            ("U", 400.),
        ]
        .into_iter()
        .map(|(m, t)| json!({"move":m,"t":t}))
        .collect::<Vec<_>>();
        app.open_review(json!({"id":"step-test","kind":"solve","source":"bluetooth","status":"complete","initialState":Cube::default().to_string(),"cross":"D","duration":400.,"moves":events,"quality":[]}));
        let arrow = |app: &mut App, key| {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            let _ = ctx.run(input, |ctx| {
                app.handle_review_shortcuts(ctx);
                assert!(!ctx.input(|i| i.key_pressed(key)));
            });
            let mut release = egui::RawInput::default();
            release.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            let _ = ctx.run(release, |_| {});
        };
        arrow(&mut app, egui::Key::ArrowRight);
        assert_eq!(app.replay_index, 4);
        assert_eq!(app.review_playhead, 180.);
        assert!(app.playing.is_none());
        arrow(&mut app, egui::Key::ArrowLeft);
        assert_eq!(app.replay_index, 0);
        assert_eq!(app.review_playhead, 0.);
        app.review_raw = true;
        app.review_step_cursor = None;
        arrow(&mut app, egui::Key::ArrowRight);
        assert_eq!(app.replay_index, 1);
        assert_eq!(app.review_playhead, 0.);
        arrow(&mut app, egui::Key::ArrowRight);
        assert_eq!(app.replay_index, 2);
        assert_eq!(app.review_playhead, 60.);
        app.toggle_review_playback();
        app.step_review(true);
        assert!(app.playing.is_none());
        assert_eq!(app.replay_index, 3);
        assert!(!app.keyboard.running());
    }
    #[test]
    fn pause_resume_keeps_the_playhead_and_replay_restarts_at_the_end() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx, true);
        app.open_review(json!({"id":"replay-test","status":"complete","source":"bluetooth","kind":"solve","initialState":Cube::default().to_string(),"duration":3000.,"moves":[{"move":"R","t":0},{"move":"U","t":3000}],"quality":[]}));
        app.replay_index = 1;
        app.playing = Some(Instant::now() - std::time::Duration::from_millis(1200));
        app.toggle_review_playback();
        assert!(app.playing.is_none());
        assert!((1200.0..1400.0).contains(&app.review_playhead));
        app.toggle_review_playback();
        assert!((1200.0..1400.0).contains(&(app.playing.unwrap().elapsed().as_secs_f64() * 1000.)));
        app.playing = None;
        app.replay_index = 2;
        app.toggle_review_playback();
        assert_eq!(app.replay_index, 0);
        assert_eq!(app.review_playhead, 0.);
    }
}
