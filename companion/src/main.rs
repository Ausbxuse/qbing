use eframe::egui::{self, Color32, FontId, RichText, Stroke, Vec2};
use serde::Deserialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const BG: Color32 = Color32::from_rgb(30, 34, 51);
const FG: Color32 = Color32::from_rgb(228, 227, 239);
const MUTED: Color32 = Color32::from_rgb(147, 155, 181);
const ACCENT: Color32 = Color32::from_rgb(180, 190, 254);

#[derive(Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
struct Segment {
    text: String,
    kind: String,
}
#[derive(Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
struct Snapshot {
    scramble: String,
    segments: Vec<Segment>,
    connected: bool,
    device: String,
    time: String,
    tps: Option<f64>,
    online: bool,
}
struct Companion {
    data: Arc<Mutex<Snapshot>>,
    pinned: bool,
    launcher: String,
    capture: Option<String>,
    capture_requested: bool,
    started: Instant,
}

impl Companion {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "maple".into(),
            egui::FontData::from_static(include_bytes!("../assets/MapleMono-NF-Regular.ttf"))
                .into(),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.insert(family, vec!["maple".into()]);
        }
        cc.egui_ctx.set_fonts(fonts);
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(FG);
        visuals.panel_fill = BG;
        visuals.window_corner_radius = egui::CornerRadius::ZERO;
        visuals.menu_corner_radius = egui::CornerRadius::ZERO;
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::ZERO;
        }
        visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 54, 75);
        cc.egui_ctx.set_visuals(visuals);
        let data = Arc::new(Mutex::new(Snapshot::default()));
        let shared = data.clone();
        let ctx = cc.egui_ctx.clone();
        let endpoint =
            std::env::var("CSTIMER_URL").unwrap_or_else(|_| "http://localhost:3327".into());
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(750))
                .build()
                .unwrap();
            loop {
                let result = client
                    .get(format!("{endpoint}/api/companion/state"))
                    .send()
                    .and_then(|r| r.error_for_status())
                    .and_then(|r| r.json::<Snapshot>());
                let mut latest = shared.lock().unwrap();
                match result {
                    Ok(next) => {
                        if *latest != next {
                            *latest = next;
                            ctx.request_repaint();
                        }
                    }
                    Err(_) => {
                        if latest.connected || latest.online {
                            latest.connected = false;
                            latest.online = false;
                            ctx.request_repaint();
                        }
                    }
                }
                drop(latest);
                std::thread::sleep(Duration::from_millis(200));
            }
        });
        let args: Vec<String> = std::env::args().collect();
        let capture = args
            .windows(2)
            .find(|pair| pair[0] == "--screenshot")
            .map(|pair| pair[1].clone());
        Self {
            data,
            pinned: true,
            launcher: std::env::var("CSTIMER_LAUNCHER").unwrap_or_else(|_| "cstimer-local".into()),
            capture,
            capture_requested: false,
            started: Instant::now(),
        }
    }
    fn open_timer(&self) {
        let _ = std::process::Command::new(&self.launcher).spawn();
    }
}
fn icon(ui: &mut egui::Ui, pin: bool, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(16., 18.), egui::Sense::click());
    let paint = ui.painter();
    let color = if response.hovered() {
        FG
    } else if active {
        ACCENT
    } else {
        MUTED
    };
    if response.hovered() {
        paint.rect_filled(rect, 0., Color32::from_rgb(48, 54, 75));
    }
    let origin = rect.center() - Vec2::splat(5.5);
    let p = |x: f32, y: f32| origin + Vec2::new(x * 11. / 24., y * 11. / 24.);
    let stroke = Stroke::new(1.2_f32, color);
    if pin {
        let pts = [
            p(15., 3.),
            p(21., 9.),
            p(17., 10.),
            p(13., 15.),
            p(13., 19.),
            p(5., 11.),
            p(9., 11.),
            p(14., 7.),
            p(15., 3.),
        ];
        paint.add(egui::Shape::line(pts.to_vec(), stroke));
        paint.line_segment([p(8., 16.), p(3., 21.)], stroke);
    } else {
        paint.line_segment([p(14., 3.), p(21., 3.)], stroke);
        paint.line_segment([p(21., 3.), p(21., 10.)], stroke);
        paint.line_segment([p(21., 3.), p(11., 13.)], stroke);
        paint.add(egui::Shape::line(
            vec![p(11., 5.), p(4., 5.), p(4., 21.), p(20., 21.), p(20., 14.)],
            stroke,
        ));
    }
    response
}
impl eframe::App for Companion {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(output) = &self.capture {
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    image::save_buffer(
                        output,
                        &bytes,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    )
                    .expect("Save native screenshot");
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            if self.started.elapsed() > Duration::from_secs(1) && !self.capture_requested {
                self.capture_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            ctx.request_repaint_after(Duration::from_millis(40));
        }
        let data = self.data.lock().unwrap().clone();
        let frame = egui::Frame::new()
            .fill(BG)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(65, 71, 94)))
            .corner_radius(0)
            .inner_margin(egui::Margin::symmetric(6, 4));
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4., 0.);
            ui.spacing_mut().interact_size.y = 18.;
            let scramble = if data.scramble.is_empty() {
                "No scramble"
            } else {
                &data.scramble
            };
            let segments = if data.segments.is_empty() {
                vec![Segment {
                    text: scramble.into(),
                    kind: String::new(),
                }]
            } else {
                data.segments.clone()
            };
            // Balance at a move boundary, keeping each token and its live style.
            // Disable automatic wrapping so a long scramble cannot create row three.
            let text: String = segments.iter().map(|s| s.text.as_str()).collect();
            let split = text
                .char_indices()
                .filter(|(_, c)| c.is_whitespace())
                .min_by_key(|(i, c)| {
                    text[..*i]
                        .chars()
                        .count()
                        .max(text[*i + c.len_utf8()..].chars().count())
                })
                .map(|(i, _)| i);
            let layout = |font_size: f32| {
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = f32::INFINITY;
                let mut offset = 0;
                for segment in &segments {
                    let color = match segment.kind.as_str() {
                        "done" => Color32::from_rgb(94, 101, 125),
                        "current" => ACCENT,
                        "correction" => Color32::from_rgb(237, 165, 176),
                        _ => Color32::from_rgb(205, 208, 227),
                    };
                    let mut format = egui::TextFormat {
                        font_id: FontId::monospace(font_size),
                        color,
                        line_height: Some(14.),
                        ..Default::default()
                    };
                    if segment.kind == "current" {
                        format.underline = Stroke::new(1.0_f32, ACCENT);
                    }
                    let mut part = segment.text.clone();
                    if let Some(i) = split.filter(|i| *i >= offset && *i < offset + part.len()) {
                        let local = i - offset;
                        let length = part[local..].chars().next().unwrap().len_utf8();
                        part.replace_range(local..local + length, "\n");
                    }
                    offset += segment.text.len();
                    job.append(&part, 0., format);
                }
                ui.fonts_mut(|fonts| fonts.layout_job(job))
            };
            let mut galley = layout(11.);
            if galley.size().x > ui.available_width() {
                galley = layout(11. * ui.available_width() / galley.size().x * 0.98);
            }
            let needed = (galley.size().y + 30.).max(58.);
            let size = ctx.input(|i| {
                i.viewport()
                    .inner_rect
                    .map(|r| r.size())
                    .unwrap_or(Vec2::new(210., 58.))
            });
            if (size.y - needed).abs() > 2. {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(size.x, needed)));
            }
            let (area, response) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), galley.size().y.max(28.)),
                egui::Sense::drag(),
            );
            ui.painter().galley(area.min, galley, FG);
            if response.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            ui.add_space(2.);
            // A quiet, square result strip gives the two text rows a clear base
            // without spending any extra window space on labels or dividers.
            let footer = egui::Rect::from_min_max(
                ui.cursor().min - Vec2::new(5., 0.),
                egui::pos2(ui.max_rect().right() + 5., ui.max_rect().bottom() + 3.),
            );
            ui.painter()
                .rect_filled(footer, 0., Color32::from_rgb(25, 29, 44));
            ui.horizontal(|ui| {
                let (dot, response) =
                    ui.allocate_exact_size(Vec2::new(6., 18.), egui::Sense::click());
                if data.connected && data.online {
                    ui.painter()
                        .circle_filled(dot.center(), 2.5, Color32::from_rgb(168, 200, 145));
                } else {
                    ui.painter()
                        .circle_stroke(dot.center(), 2.5, Stroke::new(1.0_f32, MUTED));
                }
                if response
                    .on_hover_text(if data.connected && data.online {
                        format!("Connected: {}", data.device)
                    } else {
                        "Open csTimer to connect your cube".into()
                    })
                    .clicked()
                {
                    self.open_timer();
                }
                ui.label(
                    RichText::new(if data.time.is_empty() {
                        "—"
                    } else {
                        &data.time
                    })
                    .font(FontId::monospace(14.)),
                );
                let speed = data
                    .tps
                    .map(|x| format!("{x:.2}"))
                    .unwrap_or_else(|| "—".into());
                ui.label(
                    RichText::new(format!("{speed} tps"))
                        .font(FontId::monospace(10.))
                        .color(MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon(ui, false, false)
                        .on_hover_text("Open full timer")
                        .clicked()
                    {
                        self.open_timer();
                    }
                    let pin = icon(ui, true, self.pinned)
                        .on_hover_text("Always on top · right-click to close");
                    if pin.clicked() {
                        self.pinned = !self.pinned;
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.pinned {
                            egui::WindowLevel::AlwaysOnTop
                        } else {
                            egui::WindowLevel::Normal
                        }));
                    }
                    pin.context_menu(|ui| {
                        if ui.button("Close widget").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });
        });
    }
}
fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cube Companion")
            .with_app_id("cstimer-companion")
            .with_inner_size([210., 58.])
            .with_min_inner_size([180., 58.])
            .with_decorations(false)
            .with_transparent(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop),
        ..Default::default()
    };
    // X11/Xwayland supports the window level needed by the companion. Pure
    // Wayland is retained as a fallback when no X display is available.
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some() {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }));
    }
    eframe::run_native(
        "Cube Companion",
        options,
        Box::new(|cc| Ok(Box::new(Companion::new(cc)))),
    )
}
