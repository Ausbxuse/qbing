//! Native composition measured from assets/design-contract.json. No DOM/WebView.
use crate::{
    cube::{Cube, scramble_progress},
    design::{self, font, medium, mono, mono_bold, palette, semibold},
    io::{self, number, value_str},
    recording,
    ui::App,
};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use serde_json::{Value, json};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct State {
    rail_averages: bool,
    frame_elapsed: f64,
    frame_analysis: Value,
    cache: std::cell::RefCell<SessionCache>,
    graph_cache: std::cell::RefCell<crate::graph::Cache>,
    pub graph_metric: crate::graph::Metric,
    tool_tx: Option<mpsc::SyncSender<(String, String, Value)>>,
    pub tool_stage: String,
    tool_cache: std::collections::BTreeMap<String, Value>,
    tool_order: std::collections::VecDeque<String>,
    tool_errors: std::collections::BTreeMap<String, String>,
    pub result_key: String,
    pub selected: Option<String>,
    observed_result: Option<Option<String>>,
    pub tool: usize,
    pub tools_open: bool,
    pub widget: bool,
    panel_rx: Option<mpsc::Receiver<Result<(), String>>>,
    averages_open: bool,
    distribution_open: bool,
    phases_open: bool,
    pub filter: usize,
    pub query: String,
    pub group: String,
    pub tags_open: bool,
    pub cross: Option<Value>,
    pub cross_pending: Option<String>,
    pub cross_rx: Option<mpsc::Receiver<(String, Result<Value, String>)>>,
    pub bounds: std::collections::BTreeMap<String, [f32; 4]>,
}
pub fn text(
    ui: &egui::Ui,
    p: Pos2,
    anchor: Align2,
    value: impl ToString,
    font: FontId,
    color: Color32,
) {
    ui.painter().text(p, anchor, value.to_string(), font, color);
}
fn bounds(app: &mut App, name: &str, r: Rect) {
    app.shell
        .bounds
        .insert(name.into(), [r.left(), r.top(), r.width(), r.height()]);
}
fn line(ui: &egui::Ui, a: Pos2, b: Pos2, color: Color32) {
    ui.painter().line_segment([a, b], Stroke::new(1., color));
}
fn button(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    r: Rect,
    label: &str,
    selected: bool,
) -> egui::Response {
    let p = palette(ui.ctx());
    let response = ui.interact(r, ui.id().with(id), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            r,
            3.,
            if selected {
                p.accent.gamma_multiply(0.08)
            } else {
                p.text.gamma_multiply(0.025)
            },
        );
    }
    text(
        ui,
        r.center(),
        Align2::CENTER_CENTER,
        label,
        if selected { semibold(14.) } else { medium(14.) },
        if selected { p.accent } else { p.muted },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
fn icon(ui: &mut egui::Ui, id: &str, r: Rect, name: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let response = ui.interact(r, ui.id().with(id), Sense::click());
    let c = if response.hovered() {
        p.accent
    } else {
        p.faint
    };
    let o = r.center() - Vec2::splat(8.);
    let pt = |x: f32, y: f32| o + Vec2::new(x * 16. / 24., y * 16. / 24.);
    let stroke = Stroke::new(1., c);
    if paint_icon(ui, r, name, c) {
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, id));
        return response.on_hover_text(id);
    }
    match name {
        "close" => {
            ui.painter()
                .line_segment([pt(6., 6.), pt(18., 18.)], stroke);
            ui.painter()
                .line_segment([pt(18., 6.), pt(6., 18.)], stroke);
        }
        "qbing" => {
            ui.painter().rect_stroke(
                Rect::from_min_max(pt(4., 4.), pt(19., 19.)),
                3.,
                Stroke::new(2.4, p.accent),
                egui::StrokeKind::Inside,
            );
            ui.painter()
                .line_segment([pt(14., 14.), pt(22., 22.)], Stroke::new(2.4, p.accent));
        }
        "cube" => {
            ui.painter().rect_stroke(
                Rect::from_min_max(pt(4., 4.), pt(20., 20.)),
                2.,
                Stroke::new(1., p.accent),
                egui::StrokeKind::Inside,
            );
            for a in [10., 15.] {
                line(ui, pt(4., a), pt(20., a), p.accent);
                line(ui, pt(a, 4.), pt(a, 20.), p.accent);
            }
        }
        "bluetooth" => {
            for pair in [
                [7., 7., 17., 17.],
                [17., 17., 12., 21.],
                [12., 21., 12., 3.],
                [12., 3., 17., 7.],
                [17., 7., 7., 17.],
            ] {
                line(ui, pt(pair[0], pair[1]), pt(pair[2], pair[3]), c);
            }
        }
        "copy" => {
            ui.painter().rect_stroke(
                Rect::from_min_max(pt(8., 8.), pt(20., 21.)),
                1.,
                stroke,
                egui::StrokeKind::Inside,
            );
            ui.painter().line(
                vec![
                    pt(16., 8.),
                    pt(16., 3.),
                    pt(3., 3.),
                    pt(3., 16.),
                    pt(8., 16.),
                ],
                stroke,
            );
        }
        "settings" => {
            ui.painter().circle_stroke(r.center(), 6., stroke);
            ui.painter().circle_stroke(r.center(), 2., stroke);
            for n in 0..8 {
                let a = n as f32 * std::f32::consts::TAU / 8.;
                let d = Vec2::angled(a);
                line(ui, r.center() + d * 5., r.center() + d * 8., c);
            }
        }
        "star" => {
            let points = (0..10)
                .map(|n| {
                    let a = -std::f32::consts::FRAC_PI_2 + n as f32 * std::f32::consts::PI / 5.;
                    r.center() + Vec2::angled(a) * if n % 2 == 0 { 7. } else { 3.3 }
                })
                .collect::<Vec<_>>();
            ui.painter().add(egui::Shape::closed_line(points, stroke));
        }
        "tag" => {
            ui.painter().add(egui::Shape::closed_line(
                vec![
                    pt(3., 4.),
                    pt(11., 4.),
                    pt(21., 14.),
                    pt(13., 22.),
                    pt(3., 12.),
                ],
                stroke,
            ));
            ui.painter().circle_stroke(pt(8., 9.), 1., stroke);
        }
        "reset" => {
            let points = (0..30)
                .map(|n| {
                    let a = -2.6 + n as f32 * 5.2 / 29.;
                    r.center() + Vec2::angled(a) * 6.
                })
                .collect();
            ui.painter().line(points, stroke);
            ui.painter()
                .line(vec![pt(3., 4.), pt(3., 10.), pt(9., 10.)], stroke);
        }
        "play" => {
            ui.painter().add(egui::Shape::closed_line(
                vec![pt(7., 4.), pt(19., 12.), pt(7., 20.)],
                stroke,
            ));
        }
        "more" => {
            for x in [5., 12., 19.] {
                ui.painter().circle_filled(pt(x, 12.), 1., c);
            }
        }
        _ => {
            ui.painter().rect_stroke(
                Rect::from_min_max(pt(3., 5.), pt(21., 19.)),
                1.,
                stroke,
                egui::StrokeKind::Inside,
            );
            line(ui, pt(3., 10.), pt(21., 10.), c);
            line(ui, pt(14., 10.), pt(14., 19.), c);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, id));
    response.on_hover_text(id)
}
pub(crate) fn open_panel(app: &mut App, ctx: &egui::Context) {
    if !app.isolated && app.shell.panel_rx.is_none() {
        app.shell.panel_rx = Some(crate::panel::enable_top_bar(ctx.clone()));
    }
}
pub fn draw(app: &mut App, ctx: &egui::Context) {
    app.model.capture_view_frame(app.animator.live_view_frame());
    app.model.tick();
    if app.model.active.is_some() {
        (app.shell.frame_elapsed, app.shell.frame_analysis) = app.model.display_snapshot();
    } else {
        app.shell.frame_analysis = Value::Null;
    }
    if app.model.connected {
        app.keyboard.cancel();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.keyboard.cancel();
        app.review = None;
        app.show_devices = false;
        app.shell.tags_open = false;
        app.shell.tools_open = false;
        app.reset_confirm = false;
    }
    let blocked =
        app.review.is_some() || app.show_devices || app.reset_confirm || app.shell.tags_open;
    if app.page == "Practice"
        && !app.model.connected
        && !blocked
        && !ctx.wants_keyboard_input()
        && !ctx.input(|i| i.modifiers.any())
    {
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            if let Some(s) = app.keyboard.press(Instant::now()) {
                app.model.manual(s);
            }
        }
        if ctx.input(|i| i.key_released(egui::Key::Space)) {
            app.keyboard.release(Instant::now());
        }
    } else if app.keyboard.holding() {
        app.keyboard.cancel();
    }
    follow_new_result(app);
    if let Some(c) = app.model.visual_reset.take() {
        app.animator.snap(c);
    }
    app.animator.set_orientation(app.model.orientation);
    app.model.capture_view_frame(app.animator.live_view_frame());
    for (m, t, r) in app.model.animation_moves.drain(..) {
        app.animator.push_timed(m, t, r);
    }
    if app.applied_theme != app.model.settings.theme {
        design::apply_theme(ctx, &app.model.settings.theme);
        app.applied_theme = app.model.settings.theme.clone();
    }
    if let Some(rx) = &app.shell.cross_rx {
        if let Ok((id, result)) = rx.try_recv() {
            if app.shell.cross_pending.as_ref() == Some(&id) {
                match result {
                    Ok(value) => {
                        app.shell.cross = Some(value.clone());
                        app.shell.tool_cache.insert(id.clone(), value);
                    }
                    Err(reason) => {
                        app.shell.cross = None;
                        app.shell.tool_errors.insert(id.clone(), reason);
                    }
                }
                app.shell.tool_order.retain(|key| key != &id);
                app.shell.tool_order.push_back(id);
                while app.shell.tool_order.len() > 16 {
                    if let Some(key) = app.shell.tool_order.pop_front() {
                        app.shell.tool_cache.remove(&key);
                        app.shell.tool_errors.remove(&key);
                    }
                }

                app.shell.cross_pending = None;
            }
        }
    }
    if app.keyboard.running()
        || app.keyboard.holding()
        || app.model.active.is_some()
        || app.model.countdown.is_some()
    {
        ctx.request_repaint_after(Duration::from_millis(16));
    } else {
        ctx.request_repaint_after(Duration::from_secs(1));
    }
    app.shell.bounds.clear();
    if app.panel.as_ref().is_some_and(|p| p.take_panel_request()) {
        open_panel(app, ctx);
    }
    if let Some(result) = app
        .shell
        .panel_rx
        .as_ref()
        .and_then(|rx| rx.try_recv().ok())
    {
        app.model.message = result.err().unwrap_or_default();
        app.shell.panel_rx = None;
    }
    if let Some(panel) = &app.panel {
        if panel.take_widget() {
            app.shell.widget = true;
        }
        panel.publish(companion_snapshot(app));
    }
    if !app.window_visible {
        // Keep the authoritative model and panel snapshot alive without drawing
        // the dashboard or accumulating visual turn animations in the background.
        if let Some(cube) = app.model.state {
            app.animator.snap(cube);
        }
        if app.shell.widget {
            companion(app, ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(if app.model.active.is_some() {
            33
        } else {
            1000
        }));
        return;
    }
    if ctx.input(|i| i.viewport().minimized.unwrap_or(false)) && !app.shell.widget {
        if let Some(cube) = app.model.state {
            app.animator.snap(cube);
        }
        app.model.animation_moves.clear();
        ctx.request_repaint_after(Duration::from_millis(if app.model.active.is_some() {
            33
        } else {
            1000
        }));
        return;
    }
    let p = palette(ctx);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(p.bg))
        .show(ctx, |ui| {
            let r = ui.max_rect();
            let mobile = r.width() <= 760.;
            let header = if mobile { 88. } else { 56. };
            let footer = if mobile { 48. } else { 0. };
            let hr = Rect::from_min_size(r.min, Vec2::new(r.width(), header));
            bounds(app, ".app-header", hr);
            header_view(app, ui, hr, mobile);
            let body = Rect::from_min_max(
                Pos2::new(r.left(), r.top() + header),
                Pos2::new(r.right(), r.bottom() - footer),
            );
            if app.page == "Practice" {
                timer_layout(app, ui, body, mobile);
            } else if app.page == "History" {
                let w = body.width().min(1120.);
                let rr = Rect::from_center_size(body.center(), Vec2::new(w, body.height()));
                history(app, ui, rr, true);
            } else {
                let inset = if mobile { 16. } else { 32. };
                let content = body.shrink2(Vec2::new(inset, 24.));
                let width = content.width().min(match app.page.as_str() {
                    "Settings" => 680.,
                    "Case" => 760.,
                    _ => 1120.,
                });
                let content =
                    Rect::from_center_size(content.center(), Vec2::new(width, content.height()));
                ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| match app.page.as_str() {
                        "Library" => app.case_grid(ui),
                        "Case" => app.case_detail(ui),
                        "Training" => app.training_page(ui),
                        "Insights" => app.insights_view(ui),
                        _ => app.settings(ui),
                    });
                });
            }
            if mobile {
                let nr = Rect::from_min_max(Pos2::new(r.left(), r.bottom() - footer), r.max);
                line(ui, nr.left_top(), nr.right_top(), p.line);
                let pages = [
                    ("Practice", "Timer"),
                    ("History", "History"),
                    ("Library", "Practice"),
                ];
                for (i, (page, label)) in pages.iter().enumerate() {
                    let rr = Rect::from_min_size(
                        nr.min + Vec2::new(i as f32 * nr.width() / 3., 0.),
                        Vec2::new(nr.width() / 3., 48.),
                    );
                    if button(ui, ("mobile", page), rr, label, &app.page == page).clicked() {
                        app.page = (*page).into();
                    }
                }
            }
        });
    if app.shell.tools_open {
        egui::Window::new("Tools")
            .collapsible(false)
            .resizable(false)
            .default_width(340.)
            .show(ctx, |ui| {
                tools(app, ui, ui.available_rect_before_wrap());
            });
    }
    if app.shell.distribution_open {
        let mut open = true;
        egui::Window::new("Session statistics")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                let records = session_records(app);
                let w = (ctx.content_rect().width() - 64.).clamp(260., 440.);
                let times = records
                    .iter()
                    .map(io::adjusted)
                    .filter(|n| n.is_finite())
                    .collect::<Vec<_>>();
                if !times.is_empty() {
                    ui.label(format!(
                        "Best {}   Mean {}",
                        io::seconds(times.iter().copied().fold(f64::INFINITY, f64::min)),
                        io::seconds(times.iter().sum::<f64>() / times.len() as f64)
                    ));
                }
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, 144.), Sense::hover());
                histogram(app, ui, r, &records);
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, 60.), Sense::hover());
                insights(app, ui, r, &records, true);
            });
        app.shell.distribution_open = open;
    }
    if app.shell.averages_open {
        let mut open = true;
        egui::Window::new("Averages")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(336., 90.), Sense::hover());
                averages(app, ui, r, false);
            });
        app.shell.averages_open = open;
    }
    if app.shell.phases_open {
        let mut open = true;
        let rec = selected(app);
        egui::Window::new("CFOP")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(328., 172.), Sense::hover());
                phases(app, ui, r, rec.as_ref(), true, false, true);
            });
        app.shell.phases_open = open;
    }
    if app.shell.tags_open {
        tag_dialog(app, ctx);
    }
    if app.shell.widget {
        companion(app, ctx);
    }
    let error = app.model.message.to_lowercase();
    if ["failed", "error", "invalid", "unsupported", "could not"]
        .iter()
        .any(|word| error.contains(word))
    {
        egui::Area::new(egui::Id::new("native-error"))
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0., -20.))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.raised)
                    .stroke(Stroke::new(1., p.line))
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(&app.model.message)
                                    .font(font(12.))
                                    .color(p.error),
                            );
                            if ui.small_button("×").clicked() {
                                app.model.message.clear();
                            }
                        });
                    });
            });
    }
    app.dialogs(ctx);
}
fn header_view(app: &mut App, ui: &mut egui::Ui, r: Rect, mobile: bool) {
    let p = palette(ui.ctx());
    line(ui, r.left_bottom(), r.right_bottom(), p.line);
    icon(
        ui,
        "Timer",
        Rect::from_min_size(
            r.min + Vec2::new(if mobile { 12. } else { 20. }, 8.),
            Vec2::new(26., 40.),
        ),
        "qbing",
    );
    text(
        ui,
        r.min + Vec2::new(if mobile { 46. } else { 54. }, 28.),
        Align2::LEFT_CENTER,
        "Qbing",
        semibold(14.),
        p.text,
    );
    if mobile {
        session_button(
            app,
            ui,
            Rect::from_min_size(r.min + Vec2::new(16., 48.), Vec2::new(160., 32.)),
        );
    } else {
        let widths = [64., 78., 82.];
        let start = r.center().x - widths.iter().sum::<f32>() / 2.;
        let mut x = start;
        for (i, (page, label)) in [
            ("Practice", "Timer"),
            ("History", "History"),
            ("Library", "Practice"),
        ]
        .iter()
        .enumerate()
        {
            let rr = Rect::from_min_size(Pos2::new(x, r.top() + 10.), Vec2::new(widths[i], 36.));
            if button(ui, ("nav", page), rr, label, &app.page == page).clicked() {
                app.page = (*page).into();
            }
            x += widths[i];
        }
    }
    if mobile
        && icon(
            ui,
            "Open companion",
            Rect::from_min_size(
                Pos2::new(r.right() - 44., r.top() + 52.),
                Vec2::new(28., 28.),
            ),
            "widget",
        )
        .clicked()
    {
        open_panel(app, ui.ctx());
    }
    let close = Rect::from_min_size(
        Pos2::new(
            r.right() - if mobile { 44. } else { 52. },
            r.top() + if mobile { 6. } else { 12. },
        ),
        Vec2::new(28., 32.),
    );
    if icon(ui, "Close window", close, "close").clicked() {
        app.keep_in_background(ui.ctx());
    }
    let settings = Rect::from_min_size(
        Pos2::new(
            r.right() - if mobile { 80. } else { 88. },
            r.top() + if mobile { 6. } else { 12. },
        ),
        Vec2::new(28., 32.),
    );
    if icon(ui, "Options", settings, "settings").clicked() {
        app.page = "Settings".into();
    }
    let device = Rect::from_min_size(
        Pos2::new(
            settings.left() - if mobile { 52. } else { 130. },
            settings.top(),
        ),
        Vec2::new(
            if mobile { 44. } else { 120. },
            if mobile { 44. } else { 32. },
        ),
    );
    if !mobile
        && button(
            ui,
            "Open companion",
            Rect::from_min_size(
                Pos2::new(
                    device.left() - if r.width() < 980. { 42. } else { 114. },
                    device.top(),
                ),
                Vec2::new(if r.width() < 980. { 34. } else { 106. }, 32.),
            ),
            if r.width() < 980. { "↗" } else { "Companion" },
            false,
        )
        .clicked()
    {
        open_panel(app, ui.ctx());
    }
    if !app.model.connected && !mobile {
        ui.painter().rect_filled(device, 4., p.accent);
    }
    let response = ui.interact(device, ui.id().with("bluetooth"), Sense::click());
    let label = if app.model.connected {
        app.model.device_name.as_str()
    } else if app.model.connecting {
        "Pairing…"
    } else {
        "Connect"
    };
    if mobile {
        paint_icon(
            ui,
            device,
            "bluetooth",
            if app.model.connected {
                p.green
            } else {
                p.accent
            },
        );
    } else {
        text(
            ui,
            device.center(),
            Align2::CENTER_CENTER,
            label,
            medium(14.),
            if app.model.connected {
                p.muted
            } else {
                p.on_accent
            },
        );
    }
    ui.painter().circle_filled(
        Pos2::new(device.right() - 6., device.top() + 6.),
        2.5,
        if app.model.connected {
            p.green
        } else {
            p.faint
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Connect Bluetooth cube")
    });
    if response.clicked() {
        app.show_devices = true;
        if !app.model.connected && app.model.powered && !app.model.connecting {
            app.model.scan();
        }
    }
}
fn session_button(app: &mut App, ui: &mut egui::Ui, r: Rect) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
        let mut session = app.model.settings.session.clone();
        let mut sessions = app
            .model
            .records
            .iter()
            .map(|r| value_str(r, "session").to_string())
            .collect::<Vec<_>>();
        sessions.push(session.clone());
        sessions.sort();
        sessions.dedup();
        egui::ComboBox::from_id_salt("native-session")
            .selected_text(RichText::new(&session).font(font(14.)))
            .width(r.width() - 16.)
            .show_ui(ui, |ui| {
                for name in sessions {
                    ui.selectable_value(&mut session, name.clone(), name);
                }
                if ui.button("New session").clicked() {
                    session = format!(
                        "Session {}",
                        app.model
                            .records
                            .iter()
                            .map(|r| value_str(r, "session"))
                            .collect::<std::collections::HashSet<_>>()
                            .len()
                            + 1
                    );
                }
            });
        if session != app.model.settings.session {
            app.model.set_session(&session);
        }
    });
}
#[derive(Default)]
struct SessionCache {
    revision: u64,
    len: usize,
    session: String,
    best: Option<f64>,
    records: std::sync::Arc<Vec<Value>>,
    ao5: std::sync::Arc<Vec<Option<f64>>>,
}
fn session_records(app: &App) -> std::sync::Arc<Vec<Value>> {
    let mut cache = app.shell.cache.borrow_mut();
    if cache.revision != app.model.revision
        || cache.len != app.model.records.len()
        || cache.session != app.model.settings.session
    {
        cache.records = std::sync::Arc::new(
            app.model
                .records
                .iter()
                .filter(|r| {
                    value_str(r, "session") == app.model.settings.session
                        && value_str(r, "kind") == "solve"
                })
                .cloned()
                .collect(),
        );
        cache.ao5 = std::sync::Arc::new(crate::native_stats::rolling_ao5(&cache.records));
        cache.best = cache
            .records
            .iter()
            .map(io::adjusted)
            .filter(|v| v.is_finite() && *v > 0.)
            .min_by(f64::total_cmp);
        cache.revision = app.model.revision;
        cache.len = app.model.records.len();
        cache.session = app.model.settings.session.clone();
    }
    cache.records.clone()
}

fn is_best(app: &App, record: &Value) -> bool {
    if record["kind"] != "solve" || value_str(record, "session") != app.model.settings.session {
        return false;
    }
    let _ = session_records(app);
    app.shell
        .cache
        .borrow()
        .best
        .is_some_and(|best| io::adjusted(record) == best)
}
// History is a temporary review context. A newly completed result must own
// the clock and every associated metric in the same frame.
fn follow_new_result(app: &mut App) {
    let current = app
        .model
        .latest
        .as_ref()
        .map(|r| value_str(r, "id").to_owned());
    if app
        .shell
        .observed_result
        .as_ref()
        .is_some_and(|previous| previous != &current)
    {
        app.shell.selected = None;
    }
    app.shell.observed_result = Some(current);
}
fn selected(app: &App) -> Option<Value> {
    app.shell
        .selected
        .as_ref()
        .and_then(|id| app.model.records.iter().find(|r| value_str(r, "id") == id))
        .cloned()
        .or_else(|| app.model.latest.clone())
        .or_else(|| session_records(app).last().cloned())
}
fn timer_layout(app: &mut App, ui: &mut egui::Ui, r: Rect, mobile: bool) {
    let layout = crate::workspace_layout::timer(r, mobile);
    let rail = layout.rail;
    app.shell.rail_averages = rail > 0. && (r.width() <= 1180. || r.height() + 56. <= 740.);
    let center = layout.center;
    bounds(app, ".timer-pane", center);
    if rail > 0. {
        history(
            app,
            ui,
            Rect::from_min_size(r.min, Vec2::new(rail, r.height())),
            false,
        );
        let tr = Rect::from_min_max(
            Pos2::new(center.right() + 16., r.top() + 24.),
            Pos2::new(r.right() - 16., r.bottom() - 16.),
        );
        bounds(app, "#review-tools", tr);
        ui.scope_builder(egui::UiBuilder::new().max_rect(tr), |ui| {
            tools(app, ui, tr);
        });
    }
    let p = palette(ui.ctx());
    let clock = layout.clock;
    bounds(app, ".clock-anchor", clock);
    let cr = layout.cube;
    let size = cr.size();
    bounds(app, ".cube-stage", cr);
    ui.scope_builder(egui::UiBuilder::new().max_rect(cr), |ui| {
        app.animator.draw(ui, size);
    });
    if icon(
        ui,
        "Reset cube view",
        Rect::from_min_size(cr.right_bottom() - Vec2::new(4., 26.), Vec2::splat(26.)),
        "reset",
    )
    .clicked()
    {
        app.animator.recenter_orientation();
    }
    let sr = layout.scramble;
    bounds(app, ".scramble-region", sr);
    scramble(app, ui, sr, mobile);
    let record = selected(app);
    let live = app
        .model
        .active
        .as_ref()
        .is_some_and(|a| a.started.is_some());
    let running = live || app.keyboard.running();
    let time = app.keyboard.elapsed().or_else(|| {
        if live {
            Some(app.shell.frame_elapsed)
        } else {
            None
        }
    });
    let value = if app.model.cue_ready {
        "▷".into()
    } else if let Some(start) = app.model.countdown {
        format!("{}", 3_u64.saturating_sub(start.elapsed().as_secs()))
    } else if let Some(t) = time {
        seconds(t)
    } else {
        record
            .as_ref()
            .map(|r| seconds(io::adjusted(r)))
            .unwrap_or("0.00".into())
    };
    if app.model.cue_ready
        && ui
            .interact(clock, ui.id().with("cue"), Sense::click())
            .clicked()
    {
        app.model.start_cue();
    }
    let estimated = if live {
        app.shell.frame_analysis["estimated"] == true
    } else {
        !app.keyboard.running()
            && record.as_ref().is_some_and(|r| {
                io::adjusted(r).is_finite() && recording::execution_duration_estimated(r)
            })
    };
    let value = if estimated {
        format!("≈ {value}")
    } else {
        value
    };
    let color = if app.keyboard.ready()
        || app
            .model
            .active
            .as_ref()
            .is_some_and(|a| a.started.is_none())
    {
        p.green
    } else if !running && record.as_ref().is_some_and(|r| is_best(app, r)) {
        p.green
    } else {
        p.text
    };
    text(
        ui,
        clock.center() + Vec2::new(0., if mobile { 6. } else { 0. }),
        Align2::CENTER_CENTER,
        &value,
        mono(layout.clock_font),
        color,
    );
    let metrics = if app.keyboard.running() {
        (None, None)
    } else if live {
        let a = app.shell.frame_analysis.clone();
        (
            a["tps"]
                .as_f64()
                .map(|n| format!("{}{n:.2}", if a["estimated"] == true { "≈ " } else { "" })),
            a["moves"].as_u64().map(|n| n.to_string()),
        )
    } else {
        (
            record
                .as_ref()
                .and_then(recording::execution_metrics)
                .map(|m| m.label()),
            record
                .as_ref()
                .and_then(recording::execution_moves)
                .map(|n| n.to_string()),
        )
    };
    let my = clock.bottom() + 14.;
    let cx = clock.center().x;
    text(
        ui,
        Pos2::new(cx - 52., my),
        Align2::RIGHT_CENTER,
        metrics.0.unwrap_or("—".into()),
        mono(if mobile { 16. } else { 18. }),
        p.cyan,
    );
    text(
        ui,
        Pos2::new(cx - 46., my),
        Align2::LEFT_CENTER,
        "tps",
        font(12.),
        p.faint,
    );
    text(
        ui,
        Pos2::new(cx + 42., my),
        Align2::RIGHT_CENTER,
        metrics.1.unwrap_or("—".into()),
        mono(if mobile { 16. } else { 18. }),
        p.text,
    );
    text(
        ui,
        Pos2::new(cx + 48., my),
        Align2::LEFT_CENTER,
        "moves",
        font(12.),
        p.faint,
    );
    if !running && layout.actions {
        let y = my + 38.;
        for (i, label) in ["+2", "DNF"].iter().enumerate() {
            let rr = Rect::from_center_size(
                Pos2::new(cx - 67. + i as f32 * 42., y),
                Vec2::new(38., 28.),
            );
            if button(
                ui,
                ("penalty", label),
                rr,
                label,
                record.as_ref().is_some_and(|r| r["penalty"] == *label),
            )
            .clicked()
            {
                if let Some(mut rec) = record.clone() {
                    rec["penalty"] = json!(if rec["penalty"] == *label { "" } else { *label });
                    app.model.save(rec);
                }
            }
        }
        if icon(
            ui,
            "Favorite solve",
            Rect::from_center_size(Pos2::new(cx + 23., y), Vec2::splat(28.)),
            "star",
        )
        .clicked()
        {
            if let Some(mut rec) = record.clone() {
                let f = rec["localReview"]["favorite"].as_bool().unwrap_or(false);
                if !rec["localReview"].is_object() {
                    rec["localReview"] = json!({});
                }
                rec["localReview"]["favorite"] = json!(!f);
                app.model.save(rec);
            }
        }
        if icon(
            ui,
            "Tag solve",
            Rect::from_center_size(Pos2::new(cx + 57., y), Vec2::splat(28.)),
            "tag",
        )
        .clicked()
        {
            app.shell.tags_open = true;
        }
        if !app.shell.rail_averages {
            if let Some(ar) = layout.averages {
                bounds(app, ".average-table", ar);
                averages(app, ui, ar, mobile);
            }
        }
    }
    let pr = layout.phase;
    bounds(app, ".phase-section", pr);
    if layout.collapsed {
        if button(
            ui,
            "Show CFOP",
            Rect::from_min_size(pr.min, Vec2::new(76., 32.)),
            "CFOP  ↗",
            false,
        )
        .clicked()
        {
            app.shell.phases_open = true;
        }
        let rr = Rect::from_min_size(pr.right_top() - Vec2::new(62., 0.), Vec2::new(62., 32.));
        if button(ui, "Open tools", rr, "Tools", false).clicked() {
            app.shell.tools_open = true;
        }
    } else {
        phases(
            app,
            ui,
            pr,
            record.as_ref(),
            layout.compact,
            layout.dense,
            rail == 0.,
        );
    }
    if layout.averages.is_none() && !app.shell.rail_averages {
        let rr = Rect::from_min_size(
            pr.min + Vec2::new(if layout.collapsed { 82. } else { 48. }, 2.),
            Vec2::new(80., 24.),
        );
        if button(ui, "Show averages", rr, "Averages", false).clicked() {
            app.shell.averages_open = true;
        }
    }
    if !app.model.message.is_empty() && app.model.message.to_lowercase().contains("error") {
        text(
            ui,
            Pos2::new(cx, r.bottom() - 10.),
            Align2::CENTER_BOTTOM,
            &app.model.message,
            font(12.),
            p.error,
        );
    }
}
fn scramble_tokens(app: &App) -> Vec<(String, u8)> {
    let Some(target) = app.model.target.as_ref() else {
        return vec![];
    };
    let progress = scramble_progress(
        app.model
            .state
            .filter(|s| scramble_progress(*s, &target.moves).matched)
            .or(app.model.guidance_checkpoint)
            .unwrap_or_default(),
        &target.moves,
    );
    let correction = app
        .model
        .recovery
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    scramble_guidance_tokens(&target.moves, &progress, &correction)
}
fn scramble_guidance_tokens(
    moves: &[crate::cube::Move],
    progress: &crate::cube::ScrambleProgress,
    correction: &[String],
) -> Vec<(String, u8)> {
    let mut tokens = vec![];
    for i in 0..=moves.len() {
        if i == progress.completed {
            for (j, c) in correction.iter().enumerate() {
                tokens.push((c.clone(), if j == 0 { 2 } else { 3 }));
            }
        }
        let Some(m) = moves.get(i) else {
            continue;
        };
        let kind = if i == progress.completed && progress.partial_moves.contains(&i) {
            4
        } else if progress.done.contains(&i) {
            1
        } else if i == progress.completed && progress.next_moves.contains(&i) {
            2
        } else {
            0
        };
        tokens.push((
            m.to_string(),
            if !correction.is_empty() && matches!(kind, 2 | 4) {
                0
            } else {
                kind
            },
        ));
    }
    tokens
}

fn scramble_token(ui: &egui::Ui, pos: Pos2, text: &str, font: FontId, kind: u8, width: f32) {
    let p = palette(ui.ctx());
    let pad = if font.size < 12. { 1. } else { 4. };
    let rect = Rect::from_min_size(
        pos - Vec2::new(pad, 1.),
        Vec2::new(width + pad * 2., font.size * 1.4),
    );
    if kind == 2 {
        ui.painter().rect_filled(rect, 2., p.accent);
    }
    if kind == 4 {
        ui.painter()
            .rect_filled(rect, 2., p.amber.gamma_multiply(0.12));
        ui.painter().rect_filled(
            Rect::from_min_max(rect.min, Pos2::new(rect.center().x, rect.bottom())),
            2.,
            p.amber.gamma_multiply(0.35),
        );
        ui.painter()
            .rect_stroke(rect, 2., Stroke::new(1., p.amber), egui::StrokeKind::Inside);
    }
    let color = match kind {
        1 => p.muted.gamma_multiply(0.35),
        2 => p.on_accent,
        3 => p.error,
        _ => p.muted,
    };
    scramble_glyph(ui, pos, text, font, color, kind == 1);
}
fn scramble_glyph(ui: &egui::Ui, pos: Pos2, t: &str, f: FontId, c: Color32, finished: bool) {
    if finished {
        for (dx, dy, weight) in [
            (0., 0., 0.24),
            (-0.65, 0., 0.13),
            (0.65, 0., 0.13),
            (0., -0.65, 0.13),
            (0., 0.65, 0.13),
            (-0.45, -0.45, 0.06),
            (0.45, 0.45, 0.06),
            (-0.45, 0.45, 0.06),
            (0.45, -0.45, 0.06),
        ] {
            text(
                ui,
                pos + Vec2::new(dx, dy),
                Align2::LEFT_TOP,
                t,
                f.clone(),
                c.gamma_multiply(weight),
            );
        }
    } else {
        text(ui, pos, Align2::LEFT_TOP, t, f, c);
    }
}
fn scramble(app: &mut App, ui: &mut egui::Ui, r: Rect, mobile: bool) {
    let p = palette(ui.ctx());
    let Some(target) = &app.model.target else {
        text(
            ui,
            r.center(),
            Align2::CENTER_CENTER,
            if app.model.scramble_loading {
                "…"
            } else {
                "—"
            },
            mono(18.),
            p.faint,
        );
        return;
    };
    let tokens = scramble_tokens(app);
    let f = mono(if r.height() < 50. {
        12.
    } else if mobile {
        16.
    } else {
        18.
    });
    let gap = f.size * 0.6 + 2.;
    let widths = tokens
        .iter()
        .map(|(t, _)| {
            ui.painter()
                .layout_no_wrap(t.clone(), f.clone(), p.text)
                .size()
                .x
                + gap
        })
        .collect::<Vec<_>>();
    // Balanced two-row wrapping, using measured glyph advances rather than guessed widths.
    let sum = widths.iter().sum::<f32>();
    let rows = (sum / r.width()).ceil().max(1.) as usize;
    let target_width = sum / rows as f32;
    let mut lines = vec![vec![]];
    let mut used = 0.;
    for (i, w) in widths.iter().enumerate() {
        if used + *w > target_width + *w * 0.6 && lines.len() < rows {
            lines.push(vec![]);
            used = 0.;
        }
        lines.last_mut().unwrap().push(i);
        used += *w;
    }
    for (j, items) in lines.iter().enumerate() {
        let w = items.iter().map(|i| widths[*i]).sum::<f32>();
        let mut x = r.center().x - w / 2.;
        let y = r.top() + j as f32 * (f.size * 1.65);
        for i in items {
            let (t, kind) = &tokens[*i];
            scramble_token(ui, Pos2::new(x, y), t, f.clone(), *kind, widths[*i] - gap);
            x += widths[*i];
        }
    }
    if let Some(tps) = app.model.scramble.live_stats()["tps"].as_f64() {
        text(
            ui,
            r.right_bottom() + Vec2::new(-24., -4.),
            Align2::RIGHT_BOTTOM,
            format!("{tps:.2} tps"),
            font(12.),
            p.faint,
        );
    }
    if icon(
        ui,
        "Copy scramble",
        Rect::from_min_size(r.right_bottom() - Vec2::new(22., 26.), Vec2::splat(26.)),
        "copy",
    )
    .clicked()
    {
        ui.ctx().copy_text(
            target
                .moves
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
}
fn averages(app: &App, ui: &egui::Ui, r: Rect, _mobile: bool) {
    let p = palette(ui.ctx());
    let records = session_records(app);
    for (i, n) in [5, 12].iter().enumerate() {
        let a = crate::practice_ui::average_metrics(&records, *n);
        let x = r.left()
            + if i == 0 {
                r.width() / 4.
            } else {
                r.width() * 3. / 4.
            };
        text(
            ui,
            Pos2::new(x, r.top() + 10.),
            Align2::CENTER_CENTER,
            format!("ao{n}"),
            medium(12.),
            p.faint,
        );
        text(
            ui,
            Pos2::new(x, r.top() + 36.),
            Align2::CENTER_CENTER,
            a.time
                .map(|v| io::seconds((v / 10.).round() * 10.))
                .unwrap_or("—".into()),
            mono_bold(18.),
            p.text,
        );
        text(
            ui,
            Pos2::new(x - 4., r.top() + 64.),
            Align2::RIGHT_CENTER,
            format!("{} tps", a.tps_label()),
            mono(12.),
            p.muted,
        );
        text(
            ui,
            Pos2::new(x + 4., r.top() + 64.),
            Align2::LEFT_CENTER,
            a.moves
                .map(|v| format!("{v:.1} moves"))
                .unwrap_or("— moves".into()),
            mono(12.),
            p.muted,
        );
    }
}

fn phases(
    app: &mut App,
    ui: &mut egui::Ui,
    r: Rect,
    record: Option<&Value>,
    compact: bool,
    dense: bool,
    show_tools: bool,
) {
    let p = palette(ui.ctx());
    line(ui, r.left_top(), r.right_top(), p.line);
    text(
        ui,
        r.min + Vec2::new(0., if dense { 8. } else { 21. }),
        Align2::LEFT_TOP,
        "CFOP",
        medium(12.),
        p.muted,
    );
    if button(
        ui,
        "Replay",
        Rect::from_min_size(
            Pos2::new(r.right() - 62., r.top() + 8.),
            Vec2::new(62., 26.),
        ),
        "▷ Replay",
        false,
    )
    .clicked()
    {
        if let Some(rec) = record {
            app.open_review(rec.clone());
        }
    }
    if show_tools
        && button(
            ui,
            "Tools",
            Rect::from_min_size(
                Pos2::new(r.right() - 117., r.top() + 8.),
                Vec2::new(50., 26.),
            ),
            "Tools",
            false,
        )
        .clicked()
    {
        app.shell.tools_open = true;
    }
    let analysis = if app
        .model
        .active
        .as_ref()
        .is_some_and(|a| a.started.is_some())
    {
        app.shell.frame_analysis.clone()
    } else {
        record
            .filter(|r| recording::execution_moves(r).is_some())
            .map(|r| r["analysis"].clone())
            .unwrap_or(Value::Null)
    };
    let list = analysis["splits"].as_array();
    let colors = if app.model.settings.theme == "paper" {
        [0x16bda2, 0x5a8fff, 0xedc94b, 0xa282ee]
    } else {
        [0x3bd6b6, 0x79a6ff, 0xf2d255, 0xba9bff]
    };
    let durations: Vec<f32> = ["Cross", "F2L", "OLL", "PLL"]
        .iter()
        .map(|name| {
            list.and_then(|s| s.iter().find(|s| value_str(s, "stage") == *name))
                .map(|s| number(s, "time") as f32)
                .unwrap_or(1.)
                .max(0.001)
        })
        .collect();
    let total: f32 = durations.iter().sum();
    let mut bar_x = r.left();
    for (i, duration) in durations.iter().enumerate() {
        let width = (r.width() - 6.) * duration / total;
        let color = colors[i];
        let color = Color32::from_rgb((color >> 16) as u8, (color >> 8) as u8, color as u8);
        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(bar_x, r.top() + if dense { 32. } else { 45. }),
                Vec2::new(width, 4.),
            ),
            0.,
            color.gamma_multiply(0.95),
        );
        bar_x += width + 2.;
    }
    for (i, name) in ["Cross", "F2L", "OLL", "PLL"].iter().enumerate() {
        let cols = if compact { 2 } else { 4 };
        let w = r.width() / cols as f32;
        let rr = Rect::from_min_size(
            Pos2::new(
                r.left() + (i % cols) as f32 * w,
                r.top()
                    + if dense { 42. } else { 58. }
                    + (i / cols) as f32 * if dense { 32. } else { 56. },
            ),
            Vec2::new(w, if dense { 32. } else { 56. }),
        );
        if i % cols > 0 {
            line(ui, rr.left_top(), rr.left_bottom(), p.line);
        }
        text(
            ui,
            rr.min + Vec2::new(12., 2.),
            Align2::LEFT_TOP,
            name,
            font(12.),
            p.muted,
        );
        let split = list.and_then(|s| s.iter().find(|s| value_str(s, "stage") == *name));
        if let Some(s) = split {
            let duration = number(s, "time");
            let raw_before = list
                .unwrap()
                .iter()
                .take_while(|v| value_str(v, "stage") != *name)
                .map(|s| number(s, "rawMoves") as usize)
                .sum::<usize>();
            let live = app.model.active.as_ref().filter(|a| a.started.is_some());
            let events = live
                .map(|a| a.moves.as_slice())
                .or_else(|| record.and_then(|r| r["moves"].as_array().map(Vec::as_slice)));
            let pause = if i == 0 {
                0.
            } else {
                events
                    .and_then(|m| m.get(raw_before))
                    .map(|m| (number(m, "t") - number(s, "start")).clamp(0., duration))
                    .unwrap_or(if s["live"] == true { duration } else { 0. })
            };
            crate::phase_time::paint(
                ui,
                rr.right_top() + Vec2::new(-12., 0.),
                s["time"].as_f64().filter(|v| v.is_finite() && *v >= 0.),
                pause,
                i == 0,
                if live.is_some() {
                    s["estimated"] == true
                } else {
                    record.is_some_and(|r| recording::split_timing_estimated(r, name))
                },
                if dense { 14. } else { 16. },
            );
            if !dense {
                text(
                    ui,
                    rr.min + Vec2::new(12., 36.),
                    Align2::LEFT_TOP,
                    format!("{} moves", s["moves"].as_u64().unwrap_or(0)),
                    mono(12.),
                    p.muted,
                );
                text(
                    ui,
                    rr.right_top() + Vec2::new(-12., 36.),
                    Align2::RIGHT_TOP,
                    s["tps"]
                        .as_f64()
                        .map(|n| format!("{n:0.2} tps"))
                        .unwrap_or("— tps".into()),
                    mono(12.),
                    p.muted,
                );
            }
        } else {
            text(
                ui,
                rr.right_top() - Vec2::new(12., 0.),
                Align2::RIGHT_TOP,
                "—",
                mono(18.),
                p.faint,
            );
        }
        if ui
            .interact(rr, ui.id().with(("phase", name)), Sense::click())
            .clicked()
        {
            app.shell.tool = if i == 0 {
                0
            } else if i == 1 {
                2
            } else {
                1
            };
            if show_tools {
                app.shell.tools_open = true;
            }
        }
    }
}
fn best_review_link(app: &mut App, ui: &mut egui::Ui, record: &Value, rect: Rect, full: bool) {
    let response = ui
        .interact(
            rect,
            ui.id().with(("best-solve-review", full)),
            Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Open best solve reconstruction");
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            "Open best solve reconstruction",
        )
    });
    if response.clicked()
        || (response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)))
    {
        app.open_review(record.clone());
        app.review_reconstruction = true;
    }
}
fn history(app: &mut App, ui: &mut egui::Ui, r: Rect, full: bool) {
    let p = palette(ui.ctx());
    bounds(app, ".history-pane", r);
    if !full {
        ui.painter().rect_filled(r, 0., p.surface);
        line(ui, r.right_top(), r.right_bottom(), p.line);
    }
    let mobile = r.width() < 900.;
    let condensed = full && r.height() < 640.;
    let ultra = full && r.height() < 400.;
    let inset = if full {
        if mobile { 16. } else { 32. }
    } else {
        16.
    };
    let inner = r.shrink2(Vec2::new(inset, 0.));
    bounds(
        app,
        if full {
            ".history-content"
        } else {
            ".history-rail-content"
        },
        inner,
    );
    let records = session_records(app);
    let row_ao5 = app.shell.cache.borrow().ao5.clone();
    let y = r.top()
        + if condensed {
            28.
        } else if full {
            38.
        } else {
            34.
        };
    text(
        ui,
        Pos2::new(inner.left(), y),
        Align2::LEFT_CENTER,
        if full {
            "History"
        } else {
            &app.model.settings.session
        },
        medium(if full { 18. } else { 14. }),
        p.text,
    );
    let mut top = y + if condensed {
        24.
    } else if full {
        42.
    } else {
        28.
    };
    if condensed
        && button(
            ui,
            "Session statistics",
            Rect::from_min_size(
                Pos2::new(inner.right() - 112., y - 14.),
                Vec2::new(112., 28.),
            ),
            "Distribution ↗",
            false,
        )
        .clicked()
    {
        app.shell.distribution_open = true;
    }
    if full {
        if !ultra {
            let names = ["Best", "Mean", "ao5", "ao12"];
            let vals = records
                .iter()
                .map(io::adjusted)
                .filter(|n| n.is_finite())
                .collect::<Vec<_>>();
            let best = records
                .iter()
                .filter(|r| io::adjusted(r).is_finite())
                .min_by(|a, b| io::adjusted(a).total_cmp(&io::adjusted(b)));
            let sw = if mobile {
                inner.width()
            } else {
                inner.width() - 360.
            };
            let two_rows = inner.width() < 520. && !condensed;
            let columns = if two_rows { 2 } else { 4 };
            let summary_top = top;
            for (i, name) in names.iter().enumerate() {
                let x = inner.left() + (i % columns) as f32 * sw / columns as f32;
                let top = summary_top + (i / columns) as f32 * 98.;
                if i == 0
                    && let Some(record) = best
                {
                    best_review_link(
                        app,
                        ui,
                        record,
                        Rect::from_min_size(
                            Pos2::new(x, top - 4.),
                            Vec2::new(sw / columns as f32 - 8., 92.),
                        ),
                        true,
                    );
                }
                text(
                    ui,
                    Pos2::new(x, top),
                    Align2::LEFT_TOP,
                    name,
                    medium(12.),
                    p.faint,
                );
                let time = match i {
                    0 => best.map(|r| seconds(io::adjusted(r))),
                    1 => {
                        if vals.is_empty() {
                            None
                        } else {
                            Some(seconds(vals.iter().sum::<f64>() / vals.len() as f64))
                        }
                    }
                    _ => crate::practice_ui::average_metrics(&records, if i == 2 { 5 } else { 12 })
                        .time
                        .map(|v| io::seconds((v / 10.).round() * 10.)),
                };
                text(
                    ui,
                    Pos2::new(x, top + 22.),
                    Align2::LEFT_TOP,
                    time.unwrap_or("—".into()),
                    mono_bold(if sw / (columns as f32) < 100. {
                        17.
                    } else {
                        22.
                    }),
                    if i == 0 { p.green } else { p.text },
                );
                let a = if i >= 2 {
                    Some(crate::practice_ui::average_metrics(
                        &records,
                        if i == 2 { 5 } else { 12 },
                    ))
                } else {
                    None
                };
                let tps = if i == 0 {
                    best.and_then(recording::execution_metrics)
                        .map(|m| m.label())
                } else if let Some(a) = &a {
                    a.tps.map(|n| format!("{n:0.2}"))
                } else {
                    let m = records
                        .iter()
                        .filter(|r| io::adjusted(r).is_finite())
                        .filter_map(recording::execution_metrics)
                        .map(|m| m.tps)
                        .collect::<Vec<_>>();
                    (!m.is_empty())
                        .then(|| format!("{:0.2}", m.iter().sum::<f64>() / m.len() as f64))
                };
                let moves = if i == 0 {
                    best.and_then(recording::execution_moves).map(|n| n as f64)
                } else if let Some(a) = &a {
                    a.moves
                } else {
                    let m = records
                        .iter()
                        .filter(|r| io::adjusted(r).is_finite())
                        .filter_map(recording::execution_moves)
                        .collect::<Vec<_>>();
                    (!m.is_empty()).then(|| m.iter().sum::<usize>() as f64 / m.len() as f64)
                };
                text(
                    ui,
                    Pos2::new(x, top + 50.),
                    Align2::LEFT_TOP,
                    format!("{} tps", tps.unwrap_or("—".into())),
                    mono(12.),
                    p.muted,
                );
                text(
                    ui,
                    Pos2::new(x, top + 68.),
                    Align2::LEFT_TOP,
                    moves
                        .map(|n| {
                            if i == 0 {
                                format!("{n:.0} moves")
                            } else {
                                format!("{n:.1} moves")
                            }
                        })
                        .unwrap_or("— moves".into()),
                    mono(12.),
                    p.muted,
                );
            }
            if !mobile {
                pace(
                    app,
                    ui,
                    Rect::from_min_size(Pos2::new(inner.right() - 320., top), Vec2::new(320., 84.)),
                    &records,
                    true,
                );
            }
            top += if two_rows { 194. } else { 112. };
            if !condensed {
                histogram(
                    app,
                    ui,
                    Rect::from_min_size(
                        Pos2::new(inner.left(), top + 14.),
                        Vec2::new(
                            if mobile {
                                inner.width()
                            } else {
                                inner.width() - 360.
                            },
                            if mobile { 96. } else { 128. },
                        ),
                    ),
                    &records,
                );
                insights(
                    app,
                    ui,
                    Rect::from_min_size(
                        Pos2::new(
                            if mobile {
                                inner.left()
                            } else {
                                inner.right() - 320.
                            },
                            top + if mobile { 116. } else { 24. },
                        ),
                        Vec2::new(
                            if mobile { inner.width() } else { 320. },
                            if mobile { 54. } else { 110. },
                        ),
                    ),
                    &records,
                    mobile,
                );
                top += if mobile { 184. } else { 160. };
            }
        }
        let filter_rect = Rect::from_min_size(
            Pos2::new(inner.left(), top),
            Vec2::new(if mobile { 178. } else { 258. }, 36.),
        );
        ui.painter().rect_stroke(
            filter_rect,
            3.,
            Stroke::new(1., p.line),
            egui::StrokeKind::Inside,
        );
        let widths = if mobile {
            [36., 52., 44., 42.]
        } else {
            [40., 58., 48., 108.]
        };
        let mut x = filter_rect.left() + 2.;
        for (i, label) in ["All", "Moves", "DNF", if mobile { "" } else { "Favorites" }]
            .iter()
            .enumerate()
        {
            let rr = Rect::from_min_size(Pos2::new(x, top + 2.), Vec2::new(widths[i], 32.));
            if button(ui, ("history-filter", i), rr, label, app.shell.filter == i).clicked() {
                app.shell.filter = i;
            }
            if i == 3 {
                paint_icon(
                    ui,
                    Rect::from_center_size(
                        Pos2::new(rr.left() + 12., rr.center().y),
                        Vec2::splat(14.),
                    ),
                    "star",
                    p.faint,
                );
            }
            x += widths[i];
        }
        let tag_rect = Rect::from_min_size(
            Pos2::new(
                if mobile {
                    inner.right() - 98.
                } else {
                    filter_rect.right() + 12.
                },
                top,
            ),
            Vec2::new(98., 36.),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(tag_rect), |ui| {
            ui.spacing_mut().interact_size.y = 36.;
            ui.spacing_mut().button_padding.y = 7.;
            egui::ComboBox::from_id_salt("history-tag-filter")
                .selected_text(if app.shell.group.is_empty() {
                    "All tags"
                } else {
                    &app.shell.group
                })
                .width(98.)
                .show_ui(ui, |ui| {
                    for value in [
                        "",
                        "review",
                        "practice",
                        "milestone",
                        "blue",
                        "mint",
                        "amber",
                        "rose",
                    ] {
                        ui.selectable_value(
                            &mut app.shell.group,
                            value.to_string(),
                            if value.is_empty() { "All tags" } else { value },
                        );
                    }
                });
        });
        let search_rect = Rect::from_min_size(
            Pos2::new(
                if mobile {
                    inner.left()
                } else {
                    inner.right() - 280.
                },
                top + if mobile { 42. } else { 0. },
            ),
            Vec2::new(if mobile { inner.width() } else { 280. }, 36.),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(search_rect), |ui| {
            ui.add_sized(
                search_rect.size(),
                egui::TextEdit::singleline(&mut app.shell.query)
                    .hint_text("Find a solve or tag")
                    .font(font(14.))
                    .margin(egui::Margin::symmetric(8, 7)),
            );
        });
        if mobile {
            top += 42.;
        }
        top += 52.;
    } else {
        let best_record = records
            .iter()
            .filter(|r| io::adjusted(r).is_finite())
            .min_by(|a, b| io::adjusted(a).total_cmp(&io::adjusted(b)));
        let best = best_record.map(io::adjusted);
        let values = records
            .iter()
            .map(io::adjusted)
            .filter(|t| t.is_finite())
            .collect::<Vec<_>>();
        let mean = (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);
        for (i, (label, value)) in [("Best", best), ("Mean", mean)].into_iter().enumerate() {
            let x = inner.left() + i as f32 * inner.width() / 2.;
            if i == 0
                && let Some(record) = best_record
            {
                best_review_link(
                    app,
                    ui,
                    record,
                    Rect::from_min_size(
                        Pos2::new(x, top - 6.),
                        Vec2::new(inner.width() / 2. - 8., 28.),
                    ),
                    false,
                );
            }
            text(
                ui,
                Pos2::new(x, top + 8.),
                Align2::LEFT_CENTER,
                label,
                font(12.),
                p.faint,
            );
            text(
                ui,
                Pos2::new(x + 38., top + 8.),
                Align2::LEFT_CENTER,
                value.map(seconds).unwrap_or("—".into()),
                mono_bold(14.),
                if i == 0 { p.green } else { p.text },
            );
        }
        top += 27.;
        pace(
            app,
            ui,
            Rect::from_min_size(Pos2::new(inner.left(), top), Vec2::new(inner.width(), 84.)),
            &records,
            false,
        );
        top += 104.;
        if app.shell.rail_averages {
            let ar =
                Rect::from_min_size(Pos2::new(inner.left(), top), Vec2::new(inner.width(), 52.));
            bounds(app, ".average-table", ar);
            for (i, n) in [5, 12].iter().enumerate() {
                let a = crate::practice_ui::average_metrics(&records, *n);
                let y = top + 13. + i as f32 * 26.;
                text(
                    ui,
                    Pos2::new(inner.left(), y),
                    Align2::LEFT_CENTER,
                    format!("ao{n}"),
                    font(12.),
                    p.faint,
                );
                text(
                    ui,
                    Pos2::new(inner.left() + 80., y),
                    Align2::RIGHT_CENTER,
                    a.time
                        .map(|v| io::seconds((v / 10.).round() * 10.))
                        .unwrap_or("—".into()),
                    mono_bold(14.),
                    p.text,
                );
                text(
                    ui,
                    Pos2::new(inner.right() - 42., y),
                    Align2::RIGHT_CENTER,
                    a.tps_label(),
                    mono(12.),
                    p.muted,
                );
                text(
                    ui,
                    Pos2::new(inner.right(), y),
                    Align2::RIGHT_CENTER,
                    a.moves.map(|m| format!("{m:.1}")).unwrap_or("—".into()),
                    mono(12.),
                    p.muted,
                );
            }
            top += 66.;
        }
    }
    let row_width = inner.width() - if full && !mobile { 10. } else { 0. };
    let col = if full && !mobile {
        let ph = (row_width - 672.) / 4.;
        vec![
            10. / row_width,
            118. / row_width,
            182. / row_width,
            246. / row_width,
            310. / row_width,
            (310. + ph) / row_width,
            (310. + 2. * ph) / row_width,
            (310. + 3. * ph) / row_width,
            (310. + 4. * ph) / row_width,
            (374. + 4. * ph) / row_width,
        ]
    } else if full {
        vec![
            10. / row_width,
            (row_width - 256.) / row_width,
            (row_width - 194.) / row_width,
            (row_width - 140.) / row_width,
            (row_width - 94.) / row_width,
        ]
    } else {
        // Keep the final value inside the selection fill, matching the number
        // column's 10 px inset. Headers and rows share these same anchors.
        vec![
            10. / row_width,
            86. / row_width,
            132. / row_width,
            176. / row_width,
            1. - 10. / row_width,
        ]
    };

    for (i, label) in ["#", "Time", "ao5", "TPS", "Moves"].iter().enumerate() {
        text(
            ui,
            Pos2::new(
                inner.left() + col[i] * row_width,
                top + if full { 16. } else { 14. },
            ),
            if i == 0 {
                Align2::LEFT_CENTER
            } else {
                Align2::RIGHT_CENTER
            },
            label,
            font(12.),
            p.faint,
        );
    }
    line(
        ui,
        Pos2::new(inner.left(), top + 30.),
        Pos2::new(inner.right(), top + 30.),
        p.line,
    );
    let selected = selected(app).map(|r| value_str(&r, "id").to_string());
    let data = records
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, r)| match app.shell.filter {
            1 => !r["moves"].as_array().is_none_or(|m| m.is_empty()),
            2 => r["penalty"] == "DNF",
            3 => r["localReview"]["favorite"] == true,
            _ => true,
        })
        .filter(|(_, r)| {
            app.shell.group.is_empty()
                || r["localReview"]["group"] == app.shell.group
                || r["localReview"]["color"] == app.shell.group
        })
        .filter(|(i, r)| {
            app.shell.query.is_empty()
                || format!(
                    "{} {} {} {}",
                    i + 1,
                    seconds(io::adjusted(r)),
                    value_str(r, "scramble"),
                    r["localReview"]
                )
                .to_lowercase()
                .contains(&app.shell.query.to_lowercase())
        })
        .map(|(i, r)| (i, r))
        .collect::<Vec<_>>();
    if full && !mobile {
        for (i, label) in ["Cross", "F2L", "OLL", "PLL", "Pause"].iter().enumerate() {
            text(
                ui,
                Pos2::new(inner.left() + col[i + 5] * row_width, top + 16.),
                Align2::RIGHT_CENTER,
                label,
                font(12.),
                p.faint,
            );
        }
        text(
            ui,
            Pos2::new(inner.left() + row_width - 288., top + 16.),
            Align2::LEFT_CENTER,
            "When",
            font(12.),
            p.faint,
        );
        text(
            ui,
            Pos2::new(inner.right() - 140., top + 16.),
            Align2::LEFT_CENTER,
            "Tags",
            font(12.),
            p.faint,
        );
    }
    let scroll = Rect::from_min_max(
        Pos2::new(inner.left(), top + if full { 33. } else { 31. }),
        Pos2::new(inner.right(), r.bottom() - 40.),
    );
    bounds(app, ".history-list", scroll);
    ui.scope_builder(egui::UiBuilder::new().max_rect(scroll), |ui| {
        ui.set_clip_rect(scroll.intersect(ui.clip_rect()));
        ui.spacing_mut().item_spacing.y = 0.;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, if full { 32. } else { 30. }, data.len(), |ui, range| {
                for j in range {
                    let (index, record) = data[j];
                    let (rr, response) = ui.allocate_exact_size(
                        Vec2::new(row_width, if full { 32. } else { 30. }),
                        Sense::click(),
                    );
                    let active = selected.as_deref() == Some(value_str(record, "id"));
                    let best = is_best(app, record);
                    if active {
                        ui.painter()
                            .rect_filled(rr, 0., p.accent.gamma_multiply(0.055));
                        line(ui, rr.left_top(), rr.left_bottom(), p.accent);
                    }
                    if response.hovered() {
                        ui.painter()
                            .rect_filled(rr, 0., p.text.gamma_multiply(0.025));
                    }
                    let vals = [
                        (index + 1).to_string(),
                        seconds(io::adjusted(record)),
                        row_ao5[index]
                            .map(|v| seconds((v / 10.).round() * 10.))
                            .unwrap_or("—".into()),
                        recording::execution_metrics(record)
                            .map(|m| m.label())
                            .unwrap_or("—".into()),
                        recording::execution_moves(record)
                            .map(|n| n.to_string())
                            .unwrap_or("—".into()),
                    ];
                    for (i, value) in vals.iter().enumerate() {
                        text(
                            ui,
                            Pos2::new(rr.left() + col[i] * row_width, rr.center().y),
                            if i == 0 {
                                Align2::LEFT_CENTER
                            } else {
                                Align2::RIGHT_CENTER
                            },
                            value,
                            if i == 1 {
                                mono_bold(14.)
                            } else {
                                mono(if i == 0 || !full || mobile { 12. } else { 14. })
                            },
                            if i == 0 {
                                p.faint
                            } else if best && i == 1 {
                                p.green
                            } else if active && i == 1 {
                                p.accent
                            } else if i == 1 {
                                p.text
                            } else {
                                p.muted
                            },
                        );
                    }
                    if full && !mobile {
                        for (i, name) in ["Cross", "F2L", "OLL", "PLL"].iter().enumerate() {
                            let split = record["analysis"]["splits"]
                                .as_array()
                                .and_then(|s| s.iter().find(|s| s["stage"] == *name));
                            text(
                                ui,
                                Pos2::new(rr.left() + col[i + 5] * row_width, rr.center().y),
                                Align2::RIGHT_CENTER,
                                split
                                    .map(|s| seconds(number(s, "time")))
                                    .unwrap_or("—".into()),
                                mono(14.),
                                p.muted,
                            );
                        }
                    }
                    if full {
                        if !mobile {
                            let pause = record["analysis"]["splits"].as_array().map(|splits| {
                                let mut before = 0;
                                let mut sum = 0.;
                                for split in splits {
                                    if let Some(first) =
                                        record["moves"].as_array().and_then(|m| m.get(before))
                                    {
                                        sum +=
                                            (number(first, "t") - number(split, "start")).max(0.);
                                    }
                                    before += number(split, "rawMoves") as usize;
                                }
                                sum
                            });
                            text(
                                ui,
                                Pos2::new(rr.left() + col[9] * row_width, rr.center().y),
                                Align2::RIGHT_CENTER,
                                pause.map(seconds).unwrap_or("—".into()),
                                mono(14.),
                                p.muted,
                            );
                            let when = chrono::DateTime::parse_from_rfc3339(value_str(
                                record,
                                "createdAt",
                            ))
                            .map(|d| {
                                d.with_timezone(&chrono::Local)
                                    .format("%b %-d, %I:%M %p")
                                    .to_string()
                            })
                            .unwrap_or_default();
                            text(
                                ui,
                                Pos2::new(rr.right() - 288., rr.center().y),
                                Align2::LEFT_CENTER,
                                when,
                                mono(12.),
                                p.faint,
                            );
                        }
                        let offset = if mobile { 54. } else { 128. };
                        let star = Rect::from_center_size(
                            Pos2::new(rr.right() - offset, rr.center().y),
                            Vec2::splat(24.),
                        );
                        if icon(ui, &format!("Favorite {}", index + 1), star, "star").clicked() {
                            let mut r = record.clone();
                            if !r["localReview"].is_object() {
                                r["localReview"] = json!({});
                            }
                            r["localReview"]["favorite"] =
                                json!(!r["localReview"]["favorite"].as_bool().unwrap_or(false));
                            app.model.save(r);
                        }
                        if icon(
                            ui,
                            &format!("Tags {}", index + 1),
                            Rect::from_center_size(
                                star.center() + Vec2::new(24., 0.),
                                Vec2::splat(24.),
                            ),
                            "tag",
                        )
                        .clicked()
                        {
                            app.shell.selected = Some(value_str(record, "id").into());
                            app.shell.tags_open = true;
                        }
                        if icon(
                            ui,
                            &format!("Review {}", index + 1),
                            Rect::from_center_size(
                                Pos2::new(rr.right() - 12., rr.center().y),
                                Vec2::splat(24.),
                            ),
                            "more",
                        )
                        .clicked()
                        {
                            app.open_review(record.clone());
                        }
                    }
                    if response.clicked() {
                        app.shell.selected = Some(value_str(record, "id").into());
                        if full {
                            app.page = "Practice".into();
                        }
                    }
                    if response.double_clicked() {
                        app.open_review(record.clone());
                    }
                    response.context_menu(|ui| {
                        if ui.button("Review solve").clicked() {
                            app.open_review(record.clone());
                            ui.close();
                        }
                        if ui
                            .button(RichText::new("Delete solve…").color(p.amber))
                            .clicked()
                        {
                            app.open_review(record.clone());
                            app.delete_confirm = true;
                            ui.close();
                        }
                    });
                }
            });
    });
    line(
        ui,
        Pos2::new(inner.left(), r.bottom() - 40.),
        Pos2::new(inner.right(), r.bottom() - 40.),
        p.line,
    );
    text(
        ui,
        Pos2::new(inner.left(), r.bottom() - 20.),
        Align2::LEFT_CENTER,
        format!("{} solves", records.len()),
        font(12.),
        p.faint,
    );
    text(
        ui,
        Pos2::new(inner.right(), r.bottom() - 20.),
        Align2::RIGHT_CENTER,
        format!(
            "{}{}",
            if full { "Total " } else { "" },
            total_time(records.iter().map(|r| number(r, "duration")).sum())
        ),
        mono(12.),
        p.faint,
    );
}
fn graph_series(
    app: &App,
    records: &[Value],
    metric: crate::graph::Metric,
) -> std::sync::Arc<crate::graph::Series> {
    let mut cache = app.shell.graph_cache.borrow_mut();
    if cache.revision != app.model.revision
        || cache.session != app.model.settings.session
        || cache.len != records.len()
    {
        cache.series = [
            crate::graph::Metric::Time,
            crate::graph::Metric::Tps,
            crate::graph::Metric::Moves,
        ]
        .map(|m| std::sync::Arc::new(crate::graph::Series::new(records, m)));
        cache.revision = app.model.revision;
        cache.session = app.model.settings.session.clone();
        cache.len = records.len();
    }
    cache.series[metric.index()].clone()
}
fn graph_tabs(app: &mut App, ui: &egui::Ui, pos: Pos2) {
    let p = palette(ui.ctx());
    let mut x = pos.x;
    for metric in [
        crate::graph::Metric::Time,
        crate::graph::Metric::Tps,
        crate::graph::Metric::Moves,
    ] {
        let w = if metric == crate::graph::Metric::Moves {
            43.
        } else {
            36.
        };
        let r = Rect::from_min_size(Pos2::new(x, pos.y - 2.), Vec2::new(w, 24.));
        let active = app.shell.graph_metric == metric;
        if active {
            ui.painter()
                .rect_filled(r, 3, p.accent.gamma_multiply(0.07));
        }
        text(
            ui,
            r.center(),
            Align2::CENTER_CENTER,
            metric.label(),
            font(12.),
            if active { p.accent } else { p.muted },
        );
        let response = ui.interact(
            r,
            ui.id().with(("graph-metric", metric.index(), pos.y as i32)),
            Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!("Graph {}", metric.label()),
            )
        });
        if response.clicked() {
            app.shell.graph_metric = metric;
        }
        x += w + 2.;
    }
}
fn pace(app: &mut App, ui: &mut egui::Ui, r: Rect, records: &[Value], show_mean_label: bool) {
    let p = palette(ui.ctx());
    graph_tabs(app, ui, r.min);
    let metric = app.shell.graph_metric;
    let data = graph_series(app, records, metric);
    let Some(mean) = data.mean else {
        text(
            ui,
            r.min + Vec2::new(0., 30.),
            Align2::LEFT_TOP,
            format!("No {} recorded", metric.label()),
            font(12.),
            p.faint,
        );
        return;
    };
    text(
        ui,
        r.right_top() + Vec2::new(-2., 3.),
        Align2::RIGHT_TOP,
        format!(
            "{}{}",
            if show_mean_label || metric != crate::graph::Metric::Time {
                "Mean "
            } else {
                ""
            },
            metric.format(mean)
        ),
        mono(12.),
        p.faint,
    );
    ui.interact(
        Rect::from_min_max(
            Pos2::new(r.right() - 80., r.top()),
            Pos2::new(r.right(), r.top() + 24.),
        ),
        ui.id().with("graph-mean"),
        Sense::hover(),
    )
    .on_hover_text(format!(
        "Session mean · {} {} · {}/{} solves",
        metric.format(mean),
        metric.unit(),
        data.slots.iter().flatten().count(),
        data.total
    ));
    let start = data.slots.len().saturating_sub(40);
    let last = &data.slots[start..];
    let low = last.iter().flatten().copied().fold(mean, f64::min);
    let high = last.iter().flatten().copied().fold(mean, f64::max);
    let span = (high - low).max(metric.minimum());
    let low = low - (span - (high - low)) / 2.;
    let xy = |i: usize, n: f64| {
        Pos2::new(
            r.left() + 4. + i as f32 * (r.width() - 8.) / (last.len().max(2) - 1) as f32,
            r.bottom() - 6. - ((n - low) / span) as f32 * (r.height() - 34.),
        )
    };
    let my = xy(0, mean).y;
    let mut x = r.left();
    while x < r.right() {
        line(
            ui,
            Pos2::new(x, my),
            Pos2::new((x + 3.).min(r.right()), my),
            p.faint.gamma_multiply(0.5),
        );
        x += 6.;
    }
    let mut previous = None;
    for (i, value) in last.iter().enumerate() {
        let Some(n) = value else {
            previous = None;
            continue;
        };
        let point = xy(i, *n);
        if let Some(prev) = previous {
            line(ui, prev, point, p.accent);
        }
        previous = Some(point);
        let record = &records[start + i];
        let best = data.best == Some(*n);
        let response = ui.interact(
            Rect::from_center_size(point, Vec2::splat(12.)),
            ui.id().with(("pace", value_str(record, "id"))),
            Sense::click(),
        );
        if response.clicked() {
            app.shell.selected = Some(value_str(record, "id").into());
        }
        response.on_hover_text(format!(
            "{}{} {}",
            if best { "Session best · " } else { "" },
            metric.format(*n),
            metric.unit()
        ));
        ui.painter().circle_filled(
            point,
            if best { 3.5 } else { 2. },
            if best { p.green } else { p.accent },
        );
    }
}
fn histogram(app: &mut App, ui: &egui::Ui, r: Rect, records: &[Value]) {
    let p = palette(ui.ctx());
    graph_tabs(app, ui, r.min);
    let metric = app.shell.graph_metric;
    let d = graph_series(app, records, metric).histogram(metric, r.width());
    if d.count == 0 {
        text(
            ui,
            r.min + Vec2::new(0., 30.),
            Align2::LEFT_TOP,
            if records.is_empty() {
                "No solves yet."
            } else {
                "No recorded values."
            },
            font(12.),
            p.faint,
        );
        return;
    }
    text(
        ui,
        r.right_top(),
        Align2::RIGHT_TOP,
        format!("{}/{}", d.count, records.len()),
        mono(12.),
        p.faint,
    );
    let peak = *d.bins.iter().max().unwrap() as f32;
    let colors = if p.bg.r() > 200 {
        [
            Color32::from_rgb(86, 122, 158),
            Color32::from_rgb(112, 146, 179),
        ]
    } else {
        [
            Color32::from_rgb(120, 149, 178),
            Color32::from_rgb(140, 168, 194),
        ]
    };
    for (i, n) in d.bins.iter().enumerate() {
        let w = r.width() / d.bins.len() as f32;
        let h = *n as f32 / peak.max(1.) * (r.height() - 66.).max(16.);
        let bar = Rect::from_min_size(
            Pos2::new(r.left() + i as f32 * w, r.bottom() - 24. - h),
            Vec2::new(w, h),
        );
        ui.painter().rect_filled(bar, 0., colors[i % 2]);
        if *n > 0 {
            text(
                ui,
                bar.center_top() - Vec2::new(0., 4.),
                Align2::CENTER_BOTTOM,
                n,
                mono(12.),
                p.muted,
            );
        }
        if i % ((44. / w).ceil().max(1.) as usize) == 0 {
            text(
                ui,
                Pos2::new(bar.center().x, r.bottom() - 14.),
                Align2::CENTER_TOP,
                metric.axis(d.start + d.step * i as f64),
                mono(12.),
                p.faint,
            );
        }
        ui.interact(bar, ui.id().with(("bin", i)), Sense::hover())
            .on_hover_text(format!(
                "{}–{} {} · {} solves",
                metric.axis(d.start + d.step * i as f64),
                metric.axis(d.start + d.step * (i + 1) as f64),
                metric.unit(),
                n
            ));
    }
}
fn insights(app: &mut App, ui: &egui::Ui, r: Rect, records: &[Value], mobile: bool) {
    let p = palette(ui.ctx());
    let metric = app.shell.graph_metric;
    let d = graph_series(app, records, metric).histogram(metric, r.width());
    let dnf = crate::native_stats::distribution(records).dnf_rate;
    for (i, (label, value)) in [
        ("Median", d.median.map(|v| metric.format(v))),
        ("Std dev", d.spread.map(|v| metric.format(v))),
        ("P90", d.p90.map(|v| metric.format(v))),
        ("DNF", dnf.map(|v| format!("{:.0}%", v * 100.))),
    ]
    .iter()
    .enumerate()
    {
        let x = r.left()
            + if mobile {
                i as f32 * r.width() / 4.
            } else {
                (i % 2) as f32 * (r.width() / 2. + 12.)
            };
        let y = r.top() + if mobile { 0. } else { (i / 2) as f32 * 64. };
        text(
            ui,
            Pos2::new(x, y),
            Align2::LEFT_TOP,
            label,
            font(12.),
            p.faint,
        );
        text(
            ui,
            Pos2::new(x, y + 22.),
            Align2::LEFT_TOP,
            value.as_deref().unwrap_or("—"),
            mono(if mobile { 14. } else { 18. }),
            p.muted,
        );
    }
}

fn request_tool(app: &mut App, ui: &egui::Ui, key: String, route: &str, body: Value) {
    if app.shell.tool_tx.is_none() {
        let (tx, rx) = mpsc::sync_channel::<(String, String, Value)>(1);
        let (otx, orx) = mpsc::channel();
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            for (key, route, body) in rx {
                let result = crate::solver::request(&route, Some(&body)).map_err(|e| e.to_string());
                if otx.send((key, result)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        app.shell.tool_tx = Some(tx);
        app.shell.cross_rx = Some(orx);
    }
    if app
        .shell
        .tool_tx
        .as_ref()
        .unwrap()
        .try_send((key.clone(), route.to_string(), body))
        .is_ok()
    {
        app.shell.cross_pending = Some(key.clone());
        app.shell.result_key = key;
        app.shell.cross = None;
    }
}

pub(crate) fn cached_tool(
    app: &mut App,
    ui: &egui::Ui,
    key: String,
    route: &str,
    body: Value,
) -> Option<Result<Value, String>> {
    if let Some(value) = app.shell.tool_cache.get(&key) {
        return Some(Ok(value.clone()));
    }
    if let Some(error) = app.shell.tool_errors.get(&key) {
        return Some(Err(error.clone()));
    }
    if app.shell.cross_pending.is_none() {
        request_tool(app, ui, key, route, body);
    }
    None
}
fn tool_routes(app: &mut App, ui: &mut egui::Ui, key: String, body: Value, record: Option<&Value>) {
    let p = palette(ui.ctx());
    match cached_tool(app, ui, key, "native-cross-dashboard", body) {
        None => {
            ui.spinner();
        }
        Some(Err(error)) => {
            ui.label(RichText::new(error).font(font(12.)).color(p.error));
        }
        Some(Ok(value)) => {
            for route in value["routes"].as_array().into_iter().flatten() {
                let face = value_str(route, "face");
                let cross = value_str(route, "kind") == "cross";
                let name = match face {
                    "U" => "White",
                    "D" => "Yellow",
                    "R" => "Red",
                    "L" => "Orange",
                    "F" => "Green",
                    _ => "Blue",
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{name} · {}",
                            if cross { "Cross" } else { "Xcross" }
                        ))
                        .font(medium(12.))
                        .color(p.muted),
                    );
                    ui.label(
                        RichText::new(format!("{} moves", number(route, "length")))
                            .font(mono(12.))
                            .color(p.faint),
                    );
                });
                let alg = format!(
                    "{} {}",
                    value_str(route, "rotation"),
                    route["moves"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                ui.label(
                    RichText::new(if alg.trim().is_empty() {
                        "✓"
                    } else {
                        alg.trim()
                    })
                    .font(mono(14.))
                    .color(p.text),
                );
                if let Some(record) = record.filter(|_| cross) {
                    if let Some(split) = record["analysis"]["splits"]
                        .as_array()
                        .and_then(|s| s.iter().find(|s| s["stage"] == "Cross"))
                    {
                        let played = number(split, "moves");
                        let proposed = number(route, "length");
                        ui.label(
                            RichText::new(format!("Played {played:.0} → {proposed:.0} moves"))
                                .font(font(12.))
                                .color(if proposed < played { p.green } else { p.faint }),
                        );
                    }
                }
                if ui.link(RichText::new("Copy").font(font(12.))).clicked() {
                    ui.ctx().copy_text(alg.trim().to_string());
                }
                ui.add_space(4.);
            }
        }
    }
}
fn last_solve(app: &App) -> Option<Value> {
    app.model
        .records
        .iter()
        .rev()
        .find(|r| value_str(r, "session") == app.model.settings.session && r["kind"] == "solve")
        .cloned()
}
pub(crate) fn reconstruction_ready(app: &App) -> bool {
    app.review
        .as_ref()
        .is_none_or(|r| r["moves"].as_array().is_none_or(Vec::is_empty))
        || app.shell.tool_cache.contains_key(&app.review_solver_key)
        || app.shell.tool_errors.contains_key(&app.review_solver_key)
}
pub(crate) fn tool_evidence(app: &App) -> Value {
    let current = app.model.target.as_ref().map(|t| t.cube.to_string());
    let last = last_solve(app);
    let current_ready = current
        .as_ref()
        .is_some_and(|s| app.shell.tool_cache.contains_key(&format!("current:{s}")));
    let last_ready = last.as_ref().is_none_or(|r| {
        r["moves"].as_array().is_none_or(|m| m.is_empty())
            || app.shell.tool_cache.contains_key(&format!(
                "last:{}:{}",
                value_str(r, "id"),
                crate::recording::reconstruction_cross(r) as char
            ))
    });
    json!({"currentState":current,"lastRecordId":last.as_ref().map(|r|value_str(r,"id")),"selectedRecordId":app.shell.selected,"currentReady":current_ready,"lastReady":last_ready,"ready":current_ready&&last_ready})
}
fn tools(app: &mut App, ui: &mut egui::Ui, r: Rect) {
    let p = palette(ui.ctx());
    ui.scope_builder(egui::UiBuilder::new().max_rect(r),|ui|{
  ui.set_max_width(r.width());
  egui::ScrollArea::vertical().id_salt("tools-scroll").max_height(r.height().min(ui.ctx().content_rect().height()-120.).max(160.)).show(ui,|ui|{
   ui.set_max_width((r.width()-6.).max(160.));ui.spacing_mut().item_spacing.y=8.;
   ui.label(RichText::new("Current scramble").font(medium(14.)).color(p.text));
   if let Some(target)=app.model.target.as_ref(){
       let state=target.cube.to_string();
       tool_routes(app,ui,format!("current:{state}"),json!({"state":state,"faces":"UD"}),None);
   }else{ui.label(RichText::new("Generating scramble…").font(font(12.)).color(p.faint));}
   ui.add_space(8.);ui.separator();ui.add_space(8.);
   let Some(rec)=last_solve(app)else{
       ui.label(RichText::new("Last solve").font(medium(14.)));ui.label(RichText::new("No completed solve yet.").font(font(12.)).color(p.faint));return;
   };
   let n=session_records(app).iter().position(|r|r["id"]==rec["id"]).unwrap_or(0)+1;
   ui.label(RichText::new(format!("Last solve · #{n}")).font(medium(14.)).color(p.text));
   ui.horizontal(|ui|{for(i,name)in ["Cross","Algs","Technique"].iter().enumerate(){ui.selectable_value(&mut app.shell.tool,i,RichText::new(*name).font(font(12.)));}});
   match app.shell.tool {
    0=>{
     let face=(crate::recording::reconstruction_cross(&rec) as char).to_string();
     if crate::cube::Cube::parse(value_str(&rec,"initialState")).is_ok()&&!rec["moves"].as_array().is_none_or(|m|m.is_empty()){
      tool_routes(app,ui,format!("last:{}:{face}",value_str(&rec,"id")),json!({"state":rec["initialState"],"faces":face}),Some(&rec));
     }else{ui.label(RichText::new("Recorded turns are needed to compare this solve.").font(font(12.)).color(p.faint));}
    },
   1=>{
    if app.shell.tool_stage.is_empty(){app.shell.tool_stage="PLL".into();}
    ui.horizontal(|ui|{for stage in ["OLL","PLL"]{ui.selectable_value(&mut app.shell.tool_stage,stage.to_string(),stage);}});
    if rec["moves"].as_array().is_none_or(|m|m.is_empty()){ui.label(RichText::new("Record moves to compare case algorithms.").font(font(12.)).color(p.faint));return;}
    let key=format!("{}:alg:{}",value_str(&rec,"id"),app.shell.tool_stage);
    let ready=cached_tool(app,ui,key.clone(),"native-algorithms",json!({"record":rec,"stage":app.shell.tool_stage}));
    if ready.is_none(){ui.spinner();}
    else if let Some(Ok(data))=ready{ui.label(RichText::new("Played").font(medium(12.)).color(p.faint));ui.label(RichText::new(value_str(&data,"own")).font(mono(14.)).color(p.muted));ui.separator();ui.label(RichText::new("Variants to test").font(medium(14.)));ui.label(RichText::new("Move counts are not predictions of personal speed.").font(font(12.)).color(p.faint));for v in data["variants"].as_array().into_iter().flatten(){ui.label(RichText::new(format!("{} · {} moves",value_str(v,"name"),number(v,"moves"))).font(medium(12.)).color(p.muted));ui.label(RichText::new(value_str(v,"algorithm")).font(mono(14.)));if ui.link("Copy").clicked(){ui.ctx().copy_text(value_str(v,"algorithm").into());}if ui.link("Practice case").clicked(){app.model.prepare_case(value_str(v,"caseId"),0);app.page="Practice".into();}ui.separator();}}
    else{ui.label(RichText::new("No verified variant available for this reconstruction.").font(font(12.)).color(p.faint));}
    ui.label(RichText::new("Tao Yu / Alg-Trainer · MIT").font(font(12.)).color(p.faint));
   },
   _=>{
    let raw=rec["moves"].as_array().cloned().unwrap_or_default();if raw.is_empty(){ui.label("Recorded turns are needed for technique review.");return;}
    let gap=raw.windows(2).max_by(|a,b|(number(&a[1],"t")-number(&a[0],"t")).total_cmp(&(number(&b[1],"t")-number(&b[0],"t"))));
    if let Some(g)=gap{ui.label(RichText::new(format!("Largest move gap · {}s",seconds(number(&g[1],"t")-number(&g[0],"t")))).font(font(12.)).color(p.faint));ui.label(RichText::new(format!("{} → {}",value_str(&g[0],"move"),value_str(&g[1],"move"))).font(mono(14.)));}
    let moves=raw.iter().map(|m|value_str(m,"move")).collect::<Vec<_>>();let doubles=moves.iter().filter(|m|**m=="U2").count();if doubles>0{ui.label(RichText::new(format!("U2 × {doubles}")).font(medium(14.)));ui.label(RichText::new("Try an index–middle double flick for U2. Compare controlled repetitions before timing.").font(font(12.)).color(p.faint));}
    if moves.join(" ").contains("R U R' U'"){ui.label(RichText::new("R U R' U'").font(mono(14.)));ui.label(RichText::new("Practice this trigger with a stable right-thumb grip and relaxed U/U′ flicks.").font(font(12.)).color(p.faint));}
    ui.label(RichText::new("Move timing shows gaps; it does not measure your fingers or grip.").font(font(12.)).color(p.faint));if ui.button("Timed reconstruction").clicked(){app.open_review(rec.clone());app.review_reconstruction=true;}
   }
   }
  });
 });
}

fn tag_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(mut record) = selected(app) else {
        app.shell.tags_open = false;
        return;
    };
    let mut open = true;
    egui::Window::new("Tags")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            if !record["localReview"].is_object() {
                record["localReview"] = json!({});
            }
            let mut favorite = record["localReview"]["favorite"].as_bool().unwrap_or(false);
            let mut changed = ui.checkbox(&mut favorite, "Favorite").changed();
            record["localReview"]["favorite"] = json!(favorite);
            ui.horizontal(|ui| {
                for color in ["", "blue", "mint", "amber", "rose"] {
                    if ui
                        .selectable_label(
                            record["localReview"]["color"] == color,
                            if color.is_empty() { "None" } else { color },
                        )
                        .clicked()
                    {
                        record["localReview"]["color"] = json!(color);
                        changed = true;
                    }
                }
            });
            for group in ["", "review", "practice", "milestone"] {
                if ui
                    .selectable_label(
                        record["localReview"]["group"] == group,
                        if group.is_empty() { "No group" } else { group },
                    )
                    .clicked()
                {
                    record["localReview"]["group"] = json!(group);
                    changed = true;
                }
            }
            if changed {
                app.model.save(record.clone());
            }
        });
    app.shell.tags_open = open;
}

fn seconds(v: f64) -> String {
    io::seconds((v / 10.).floor() * 10.)
}

#[derive(serde::Deserialize)]
struct IconContour {
    closed: bool,
    points: Vec<[f32; 2]>,
}
fn paint_icon(ui: &egui::Ui, r: Rect, name: &str, c: Color32) -> bool {
    static ICONS: std::sync::OnceLock<std::collections::HashMap<String, Vec<IconContour>>> =
        std::sync::OnceLock::new();
    let icons = ICONS.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/icons.json"))
            .expect("Exported approved SVG contours")
    });
    let Some(contours) = icons.get(name) else {
        return false;
    };
    let size = if name == "cube" {
        18.
    } else if name == "reset" {
        14.
    } else {
        r.width().min(r.height()).min(16.)
    };
    let start = r.center() - Vec2::splat(size / 2.);
    for contour in contours {
        let points = contour
            .points
            .iter()
            .map(|p| start + Vec2::new(p[0], p[1]) * size / 24.)
            .collect();
        let stroke = Stroke::new(1.5 * size / 24., c);
        ui.painter().add(if contour.closed {
            egui::Shape::closed_line(points, stroke)
        } else {
            egui::Shape::line(points, stroke)
        });
    }
    true
}

pub(crate) fn companion_snapshot(app: &App) -> Value {
    let active = app.model.active.as_ref().filter(|a| a.started.is_some());
    let manual = app.keyboard.running();
    let running = manual || active.is_some();
    let recent = app.model.latest.as_ref();
    let duration = app
        .keyboard
        .elapsed()
        .or_else(|| active.map(|_| app.model.elapsed()))
        .or_else(|| recent.map(io::adjusted))
        .unwrap_or(0.);
    let moves = if manual {
        None
    } else {
        active
            .map(|a| recording::count_moves(&a.moves))
            .or_else(|| recent.and_then(recording::execution_moves))
    };
    let tps = if manual {
        None
    } else if active.is_some() {
        moves
            .filter(|_| duration > 0.)
            .map(|n| n as f64 * 1000. / duration)
    } else {
        recent.and_then(recording::execution_metrics).map(|m| m.tps)
    };
    let estimated = if let Some(active) = active {
        active.clock.is_none() || active.moves.first().is_some_and(|m| m["recovered"] == true)
    } else {
        recent
            .and_then(recording::execution_metrics)
            .is_some_and(|m| m.estimated)
    };
    let label = tps
        .map(|n| format!("{}{n:.2}", if estimated { "≈ " } else { "" }))
        .unwrap_or("—".into());
    let tokens = scramble_tokens(app)
        .iter()
        .map(|(text, kind)| json!({"text":text,"kind":kind}))
        .collect::<Vec<_>>();
    let devices = app
        .model
        .devices
        .iter()
        .map(|device| json!({"id":crate::panel::device_key(&device.address),"name":device.name}))
        .collect::<Vec<_>>();
    let connection = json!({"powered":app.model.powered,"connecting":app.model.connecting,"scanning":app.model.scanning,"saved":!app.model.settings.mac.is_empty(),"name":app.model.device_name,"message":app.model.message,"adapterStatus":app.model.adapter_status,"devices":devices});
    json!({"version":1,"connection":connection,"background":app.background,"windowVisible":app.window_visible,"renderPass":app.ctx.cumulative_pass_nr(),"pid":std::process::id(),"updatedAt":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,"connected":app.model.connected&&app.model.powered,"running":running,"time":seconds(duration),"tps":tps,"tpsLabel":label,"moves":moves,"tokens":tokens})
}
fn companion(app: &mut App, ctx: &egui::Context) {
    let viewport = egui::ViewportId::from_hash_of("cstimer-native-companion");
    ctx.show_viewport_immediate(
        viewport,
        egui::ViewportBuilder::default()
            .with_title("Qbing Companion")
            .with_icon(design::app_icon())
            .with_app_id("cstimer-companion")
            .with_decorations(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_inner_size([210., 58.])
            .with_min_inner_size([210., 58.])
            .with_max_inner_size([210., 58.]),
        |ctx, class| {
            if app.isolated && std::env::var("CUBE_LAB_CAPTURE_VIEW").as_deref() == Ok("companion")
            {
                assert!(
                    class == egui::ViewportClass::Immediate,
                    "Companion must be a detached native window"
                );
            }
            app.model.tick();
            if ctx.input(|i| i.viewport().close_requested()) {
                app.shell.widget = false;
            }
            let p = palette(ctx);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(p.bg)
                        .inner_margin(egui::Margin::same(4)),
                )
                .show(ctx, |ui| {
                    let rect = ui.max_rect();
                    let response = ui.interact(
                        Rect::from_min_size(rect.min, Vec2::new(rect.width() - 18., 38.)),
                        ui.id().with("drag"),
                        Sense::drag(),
                    );
                    if response.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    let tokens = scramble_tokens(app);
                    let mut f = mono(10.);
                    let measure = |f: FontId| {
                        tokens
                            .iter()
                            .map(|(t, _)| {
                                ui.painter()
                                    .layout_no_wrap(t.clone(), f.clone(), p.text)
                                    .size()
                                    .x
                                    + 4.
                            })
                            .collect::<Vec<_>>()
                    };
                    let mut widths = measure(f.clone());
                    let available = rect.width() - 16.;
                    if widths.iter().sum::<f32>() > available * 2. {
                        f.size *= available * 2. / widths.iter().sum::<f32>();
                        widths = measure(f.clone());
                    }
                    let mut x = rect.left();
                    let mut y = rect.top();
                    let total = widths.iter().sum::<f32>();
                    let target = if total > available {
                        total / 2.
                    } else {
                        available
                    };
                    let mut wrapped = false;
                    for ((token, kind), width) in tokens.iter().zip(widths) {
                        if !wrapped
                            && x > rect.left()
                            && x + width > rect.left() + target + width * 0.45
                        {
                            x = rect.left();
                            y += 15.;
                            wrapped = true;
                        }
                        scramble_token(ui, Pos2::new(x, y), token, f.clone(), *kind, width - 4.);
                        x += width;
                    }
                    let selected = app.model.latest.clone();
                    let running = app
                        .model
                        .active
                        .as_ref()
                        .is_some_and(|a| a.started.is_some())
                        || app.keyboard.running();
                    let time = app
                        .keyboard
                        .elapsed()
                        .or_else(|| running.then(|| app.model.elapsed()))
                        .or_else(|| selected.as_ref().map(io::adjusted))
                        .unwrap_or(0.);
                    let y = rect.bottom() - 9.;
                    ui.painter().circle_filled(
                        Pos2::new(rect.left() + 3., y),
                        2.5,
                        if app.model.connected {
                            p.green
                        } else {
                            p.faint
                        },
                    );
                    text(
                        ui,
                        Pos2::new(rect.left() + 14., y),
                        Align2::LEFT_CENTER,
                        seconds(time),
                        mono(14.),
                        if !running && selected.as_ref().is_some_and(|r| is_best(app, r)) {
                            p.green
                        } else {
                            p.text
                        },
                    );
                    let state = companion_snapshot(app);
                    text(
                        ui,
                        Pos2::new(rect.left() + 86., y),
                        Align2::LEFT_CENTER,
                        format!("{} tps", state["tpsLabel"].as_str().unwrap_or("—")),
                        mono(10.),
                        p.cyan,
                    );
                    text(
                        ui,
                        Pos2::new(rect.right(), y),
                        Align2::RIGHT_CENTER,
                        state["moves"]
                            .as_u64()
                            .map(|n| format!("{n} moves"))
                            .unwrap_or("— moves".into()),
                        mono(10.),
                        p.muted,
                    );
                    if button(
                        ui,
                        "Close companion",
                        Rect::from_min_size(
                            Pos2::new(rect.right() - 16., rect.top()),
                            Vec2::splat(16.),
                        ),
                        "×",
                        false,
                    )
                    .clicked()
                    {
                        app.shell.widget = false;
                    }
                    if running {
                        ctx.request_repaint_after(Duration::from_millis(16));
                    } else {
                        ctx.request_repaint_after(Duration::from_millis(if app.model.connected {
                            33
                        } else {
                            500
                        }));
                    }
                });
        },
    );
}

fn total_time(ms: f64) -> String {
    let s = (ms / 1000.) as u64;
    if s >= 3600 {
        format!("{}h {}m", s / 3600, s / 60 % 60)
    } else if s >= 60 {
        format!("{}m {}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod responsive_interaction_tests {
    use super::*;
    #[test]
    fn header_close_keeps_companion_running() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        app.background = true;
        let pos = Pos2::new(330., 22.);
        let mut requested = false;
        for pressed in [None, Some(true), Some(false)] {
            let events = pressed
                .map(|pressed| {
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ]
                })
                .unwrap_or_default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360., 400.))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        header_view(
                            &mut app,
                            ui,
                            Rect::from_min_size(Pos2::ZERO, Vec2::new(360., 88.)),
                            true,
                        )
                    });
                },
            );
            requested |= output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Visible(false));
        }
        assert!(requested);
        assert!(app.background);
        assert!(!app.window_visible);
    }
    #[test]
    fn compact_bluetooth_icon_opens_the_connection_panel() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        app.model.powered = false;
        let pos = Pos2::new(250., 28.);
        for events in [
            vec![],
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360., 400.))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        header_view(
                            &mut app,
                            ui,
                            Rect::from_min_size(Pos2::ZERO, Vec2::new(360., 88.)),
                            true,
                        )
                    });
                },
            );
        }
        assert!(
            app.show_devices,
            "The painted icon must not intercept its enclosing button's click"
        );
    }
}

#[cfg(test)]
mod best_and_tool_context_tests {
    use super::*;
    #[test]
    fn best_highlights_respect_penalties_ties_and_cache_updates() {
        let mut app = App::new(egui::Context::default(), true);
        app.model.settings.session = "test".into();
        app.model.records = vec![
            json!({"id":"a","kind":"solve","session":"test","status":"complete","duration":10000}),
            json!({"id":"dnf","kind":"solve","session":"test","status":"complete","duration":5000,"penalty":"DNF"}),
            json!({"id":"last","kind":"solve","session":"test","status":"complete","duration":10000}),
        ];
        assert!(is_best(&app, &app.model.records[0]));
        assert!(!is_best(&app, &app.model.records[1]));
        assert!(is_best(&app, &app.model.records[2]));
        app.shell.selected = Some("a".into());
        assert_eq!(last_solve(&app).unwrap()["id"], "last");
        app.model.records[2]["penalty"] = json!("+2");
        app.model.revision += 1;
        assert!(!is_best(&app, &app.model.records[2]));
    }
}

#[cfg(test)]
mod companion_tests {
    use super::*;
    #[test]
    fn companion_reports_htm_moves_and_tps_without_inventing_manual_metrics() {
        let mut app = App::new(egui::Context::default(), true);
        app.model.latest = Some(
            json!({"source":"bluetooth","status":"complete","duration":1000.,"quality":[],"moves":[{"move":"R","t":0},{"move":"R","t":30},{"move":"U2","t":1000}]}),
        );
        let value = companion_snapshot(&app);
        assert_eq!(value["moves"], 2);
        assert_eq!(value["tpsLabel"], "2.00");
        app.model.latest.as_mut().unwrap()["source"] = json!("manual");
        let value = companion_snapshot(&app);
        assert!(value["moves"].is_null());
        assert!(value["tps"].is_null());
    }
    #[test]
    fn minimized_main_window_keeps_cube_state_coherent_without_drawing_the_workspace() {
        let ctx = egui::Context::default();
        let mut app = App::new(ctx.clone(), true);
        let cube = crate::cube::Cube::default().turn(crate::cube::Move::parse("R").unwrap());
        app.model.state = Some(cube);
        let mut input = egui::RawInput::default();
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .minimized = Some(true);
        let _ = ctx.run(input, |ctx| draw(&mut app, ctx));
        assert!(app.shell.bounds.is_empty());
        assert_eq!(app.animator.cube, cube);
    }
}

#[cfg(test)]
mod result_selection_tests {
    use super::*;

    #[test]
    fn completed_result_replaces_history_selection_and_all_its_metrics() {
        let mut app = App::new(egui::Context::default(), true);
        let old = json!({"id":"old", "duration":12000., "status":"complete", "source":"bluetooth", "moves":[{"move":"R", "t":0},{"move":"U", "t":12000} ]});
        let new = json!({"id":"new", "duration":8000., "status":"complete", "source":"bluetooth", "moves":[{"move":"F2", "t":0},{"move":"U", "t":4000},{"move":"R", "t":8000}]});
        app.model.records = vec![old.clone()];
        app.model.latest = Some(old.clone());
        follow_new_result(&mut app);
        app.shell.selected = Some("old".into());
        app.model.records.push(new.clone());
        app.model.latest = Some(new);
        follow_new_result(&mut app);
        let displayed = selected(&app).unwrap();
        assert_eq!(displayed["id"], "new");
        assert_eq!(io::adjusted(&displayed), 8000.);
        assert_eq!(recording::execution_moves(&displayed), Some(3));
        assert_eq!(recording::execution_metrics(&displayed).unwrap().tps, 0.375);
        assert_eq!(companion_snapshot(&app)["time"], "8.00");
        // Reviewing history is still available after completion, and saving
        // metadata on the latest solve must not erase that deliberate choice.
        app.shell.selected = Some("old".into());
        app.model.latest.as_mut().unwrap()["penalty"] = json!("+2");
        follow_new_result(&mut app);
        assert_eq!(selected(&app).unwrap()["id"], "old");
    }

    #[test]
    fn first_completion_and_manual_results_follow_the_same_display_path() {
        let mut app = App::new(egui::Context::default(), true);
        follow_new_result(&mut app);
        app.model.records = vec![json!({"id":"history", "duration":20000.})];
        app.shell.selected = Some("history".into());
        app.model.latest =
            Some(json!({"id":"manual", "source":"manual", "status":"complete", "duration":5000.}));
        follow_new_result(&mut app);
        assert_eq!(selected(&app).unwrap()["id"], "manual");
        assert_eq!(companion_snapshot(&app)["time"], "5.00");
        assert!(companion_snapshot(&app)["tps"].is_null());
    }
}

#[cfg(test)]
mod recovery_cursor_tests {
    use super::*;
    use crate::cube::{Cube, sequence};
    #[test]
    fn cursor_follows_each_repair_then_returns_to_the_scramble() {
        let setup = sequence("R U F").unwrap();
        let checkpoint = Cube::default().apply(&sequence("R").unwrap());
        let progress = scramble_progress(checkpoint, &setup);
        let mut state = checkpoint.apply(&sequence("L B").unwrap());
        let repairs = sequence("B' L'").unwrap();
        for i in 0..repairs.len() {
            let remaining = repairs[i..]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let tokens = scramble_guidance_tokens(&setup, &progress, &remaining);
            assert_eq!(
                tokens
                    .iter()
                    .filter(|(_, k)| *k == 2)
                    .map(|(t, _)| t.as_str())
                    .collect::<Vec<_>>(),
                vec![remaining[0].as_str()]
            );
            assert_eq!(tokens.iter().find(|(t, _)| t == "R").unwrap().1, 1);
            assert_eq!(tokens.iter().find(|(t, _)| t == "U").unwrap().1, 0);
            state = state.apply(&repairs[i..i + 1]);
        }
        assert_eq!(state, checkpoint);
        let tokens = scramble_guidance_tokens(&setup, &progress, &[]);
        assert_eq!(tokens.iter().find(|(_, k)| *k == 2).unwrap().0, "U");
    }
    #[test]
    fn early_opposite_turn_dims_without_stealing_the_scramble_cursor() {
        let setup = sequence("R U2 D2 F").unwrap();
        for (raw, expected) in [
            ("R", vec![1, 2, 0, 0]),
            ("R D", vec![1, 2, 0, 0]),
            ("R D2", vec![1, 2, 1, 0]),
            ("R D2 U", vec![1, 4, 1, 0]),
            ("R D2 U2", vec![1, 1, 1, 2]),
        ] {
            let state = Cube::default().apply(&sequence(raw).unwrap());
            let progress = scramble_progress(state, &setup);
            assert!(progress.matched, "{raw}");
            let tokens = scramble_guidance_tokens(&setup, &progress, &[]);
            assert_eq!(
                tokens.iter().map(|(_, kind)| *kind).collect::<Vec<_>>(),
                expected,
                "{raw}"
            );
            assert_eq!(
                tokens
                    .iter()
                    .filter(|(_, kind)| matches!(kind, 2 | 4))
                    .count(),
                1
            );
        }
    }
    #[test]
    fn partial_and_parallel_scramble_markers_wait_while_repairing() {
        let setup = sequence("R U2 D2 F").unwrap();
        let state = Cube::default().apply(&sequence("R U").unwrap());
        let progress = scramble_progress(state, &setup);
        let repairing = scramble_guidance_tokens(&setup, &progress, &["L'".into()]);
        assert_eq!(
            repairing
                .iter()
                .filter(|(_, k)| matches!(k, 2 | 4))
                .collect::<Vec<_>>(),
            vec![&("L'".into(), 2)]
        );
        let resumed = scramble_guidance_tokens(&setup, &progress, &[]);
        assert!(resumed.iter().any(|(t, k)| t == "U2" && *k == 4));
        assert!(resumed.iter().any(|(t, k)| t == "D2" && *k == 0));
    }
}

#[cfg(test)]
mod best_review_navigation_tests {
    use super::*;
    #[test]
    fn sidebar_and_history_best_open_the_scored_session_record_not_the_latest() {
        for full in [false, true] {
            let ctx = egui::Context::default();
            let mut app = App::new(ctx.clone(), true);
            app.model.settings.session = "test".into();
            app.model.records = vec![
                json!({"id":"best","kind":"solve","session":"test","source":"manual","status":"complete","duration":10000.,"createdAt":"2026-10-27T00:00:00Z"}),
                json!({"id":"dnf","kind":"solve","session":"test","source":"manual","status":"complete","duration":3000.,"penalty":"DNF","createdAt":"2026-10-27T00:01:00Z"}),
                json!({"id":"plus-two","kind":"solve","session":"test","source":"manual","status":"complete","duration":9000.,"penalty":"+2","createdAt":"2026-10-27T00:02:00Z"}),
                json!({"id":"other-session","kind":"solve","session":"other","source":"manual","status":"complete","duration":4000.,"createdAt":"2026-10-27T00:03:00Z"}),
                json!({"id":"latest","kind":"solve","session":"test","source":"manual","status":"complete","duration":12000.,"createdAt":"2026-10-27T00:04:00Z"}),
            ];
            app.model.latest = app.model.records.last().cloned();
            let rect =
                Rect::from_min_size(Pos2::ZERO, Vec2::new(if full { 1100. } else { 256. }, 800.));
            let pos = if full {
                Pos2::new(70., 110.)
            } else {
                Pos2::new(70., 70.)
            };
            for pressed in [None, Some(true), Some(false)] {
                let events = pressed
                    .map(|pressed| {
                        vec![
                            egui::Event::PointerMoved(pos),
                            egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: Default::default(),
                            },
                        ]
                    })
                    .unwrap_or_default();
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 900.))),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default()
                            .show(ctx, |ui| history(&mut app, ui, rect, full));
                    },
                );
            }
            assert_eq!(app.review.as_ref().unwrap()["id"], "best");
            assert!(app.review_reconstruction);
        }
    }
}
