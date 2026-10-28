use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, RichText, Stroke, Vec2};

mod typography;
pub use typography::paint_timer;

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub line: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub error: Color32,
    pub cyan: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub green: Color32,
    pub amber: Color32,
}
const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}
pub fn colors(name: &str) -> Palette {
    let light = name == "paper";
    Palette {
        bg: rgb(if light { 0xfcfcfd } else { 0x17191d }),
        surface: rgb(if light { 0xf4f5f7 } else { 0x1d2026 }),
        raised: rgb(if light { 0xffffff } else { 0x242831 }),
        line: if light {
            Color32::from_black_alpha(23)
        } else {
            Color32::from_white_alpha(23)
        },
        text: rgb(if light { 0x202633 } else { 0xf0f2f7 }),
        muted: rgb(if light { 0x566174 } else { 0xbac2cf }),
        faint: rgb(if light { 0x636f80 } else { 0x939fb1 }),
        error: rgb(if light { 0xb44e45 } else { 0xf0a59d }),
        cyan: rgb(if light { 0x087f82 } else { 0x74ccbf }),
        accent: rgb(if light { 0x2b5de5 } else { 0x86a9ff }),
        on_accent: rgb(if light { 0xffffff } else { 0x111827 }),
        green: rgb(if light { 0x25764d } else { 0x9ed2b2 }),
        amber: rgb(if light { 0x966414 } else { 0xdbb978 }),
    }
}
pub fn palette(ctx: &egui::Context) -> Palette {
    colors(&ctx.data(|d| {
        d.get_temp::<String>(egui::Id::new("theme"))
            .unwrap_or_default()
    }))
}
pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Medium".into()))
}
pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Display".into()))
}
pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}
pub fn app_icon() -> std::sync::Arc<egui::IconData> {
    static ICON: std::sync::OnceLock<std::sync::Arc<egui::IconData>> = std::sync::OnceLock::new();
    ICON.get_or_init(|| {
        let image = image::load_from_memory(include_bytes!("../../assets/app-icon/qbing-256.png"))
            .expect("Valid bundled app icon")
            .into_rgba8();
        std::sync::Arc::new(egui::IconData {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        })
    })
    .clone()
}
pub fn install_fonts(ctx: &egui::Context) {
    let mut f = egui::FontDefinitions::default();
    for (name, bytes) in [
        (
            "Regular",
            include_bytes!("../assets/Geist-Regular.ttf").as_slice(),
        ),
        (
            "Medium",
            include_bytes!("../assets/Geist-Medium.ttf").as_slice(),
        ),
        (
            "Semibold",
            include_bytes!("../assets/Geist-SemiBold.ttf").as_slice(),
        ),
        (
            "Display",
            include_bytes!("../assets/MapleMono-Regular.ttf").as_slice(),
        ),
        (
            "Timer",
            include_bytes!("../assets/MapleMono-Regular.ttf").as_slice(),
        ),
        (
            "MonoSemibold",
            include_bytes!("../assets/MapleMono-SemiBold.ttf").as_slice(),
        ),
        (
            "Notation",
            include_bytes!("../assets/MapleMono-Regular.ttf").as_slice(),
        ),
    ] {
        f.font_data
            .insert(name.into(), egui::FontData::from_static(bytes).into());
        f.families
            .insert(FontFamily::Name(name.into()), vec![name.into()]);
    }
    f.families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "Regular".into());
    f.families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "Notation".into());
    ctx.set_fonts(f);
}
pub fn apply_theme(ctx: &egui::Context, name: &str) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("theme"), name.to_string()));
    let p = colors(name);
    let mut s = egui::Style {
        visuals: if name == "paper" {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        },
        ..Default::default()
    };
    s.animation_time = 0.14;
    s.visuals.panel_fill = p.bg;
    s.visuals.window_fill = p.surface;
    s.visuals.extreme_bg_color = p.bg;
    s.visuals.faint_bg_color = p.surface;
    s.visuals.override_text_color = Some(p.text);
    s.visuals.selection.bg_fill = p.raised;
    s.visuals.selection.stroke = Stroke::new(1.0f32, p.accent);
    s.visuals.widgets.noninteractive.bg_fill = p.surface;
    s.visuals.widgets.noninteractive.weak_bg_fill = p.surface;
    s.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, p.line);
    s.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, p.muted);
    for w in [
        &mut s.visuals.widgets.inactive,
        &mut s.visuals.widgets.hovered,
        &mut s.visuals.widgets.active,
        &mut s.visuals.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(3);
        w.bg_stroke = Stroke::NONE;
        w.fg_stroke = Stroke::new(1.0f32, p.text);
    }
    s.visuals.widgets.inactive.bg_fill = p.bg;
    s.visuals.widgets.inactive.bg_stroke = Stroke::new(1., p.line);
    s.visuals.widgets.inactive.weak_bg_fill = p.raised;
    s.visuals.widgets.hovered.bg_fill = p.line;
    s.visuals.widgets.hovered.weak_bg_fill = p.line;
    s.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, p.line);
    s.visuals.widgets.active.bg_fill = p.line;
    s.visuals.widgets.active.weak_bg_fill = p.line;
    s.visuals.widgets.active.bg_stroke = Stroke::new(1.0f32, p.accent);
    s.visuals.widgets.open.bg_fill = p.raised;
    s.visuals.widgets.open.weak_bg_fill = p.raised;
    s.visuals.window_corner_radius = egui::CornerRadius::same(4);
    s.visuals.window_stroke = Stroke::new(1.0f32, p.line);
    s.visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(75),
    };
    s.spacing.item_spacing = Vec2::new(10., 8.);
    s.spacing.button_padding = Vec2::new(10., 5.);
    s.spacing.interact_size = Vec2::new(28., 32.);
    s.spacing.slider_width = 180.;
    s.text_styles.insert(egui::TextStyle::Body, font(14.));
    s.text_styles.insert(egui::TextStyle::Button, medium(14.));
    s.text_styles.insert(egui::TextStyle::Small, font(12.));
    s.text_styles.insert(egui::TextStyle::Heading, medium(23.));
    s.text_styles.insert(egui::TextStyle::Monospace, mono(15.));
    ctx.set_style(s);
}

#[derive(Clone, Copy)]
pub enum Icon {
    Timer,
    Train,
    Library,
    Chart,
    History,
    Settings,
    Cube,
    Next,
    Copy,
    Check,
}
pub fn icon(p: &egui::Painter, rect: Rect, kind: Icon, color: Color32) {
    let at = |x: f32, y: f32| {
        Pos2::new(
            rect.left() + x * rect.width(),
            rect.top() + y * rect.height(),
        )
    };
    let stroke = Stroke::new(1.5f32, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], stroke);
    };
    match kind {
        Icon::Timer => {
            p.circle_stroke(at(0.5, 0.56), rect.width() * 0.34, stroke);
            line((0.39, 0.08), (0.61, 0.08));
            line((0.5, 0.22), (0.5, 0.08));
            line((0.5, 0.56), (0.65, 0.39));
        }
        Icon::Train => {
            for (y, x) in [(0.25, 0.7), (0.5, 0.9), (0.75, 0.6)] {
                line((0.2, y), (x, y));
            }
            line((0.08, 0.2), (0.08, 0.3));
            line((0.08, 0.45), (0.08, 0.55));
            line((0.08, 0.7), (0.08, 0.8));
        }
        Icon::Library => {
            for x in [0.15, 0.4, 0.65] {
                p.rect_stroke(
                    Rect::from_min_max(at(x, 0.16), at(x + 0.17, 0.84)),
                    2,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
        }
        Icon::Chart => {
            line((0.15, 0.1), (0.15, 0.85));
            line((0.15, 0.85), (0.9, 0.85));
            line((0.25, 0.66), (0.45, 0.43));
            line((0.45, 0.43), (0.62, 0.55));
            line((0.62, 0.55), (0.86, 0.2));
        }
        Icon::History => {
            p.circle_stroke(at(0.52, 0.5), rect.width() * 0.35, stroke);
            line((0.52, 0.28), (0.52, 0.52));
            line((0.52, 0.52), (0.69, 0.6));
        }
        Icon::Settings => {
            for (y, x) in [(0.25, 0.65), (0.5, 0.35), (0.75, 0.58)] {
                line((0.12, y), (0.88, y));
                p.circle_filled(at(x, y), rect.width() * 0.08, color);
            }
        }
        Icon::Cube => {
            let a = (0.5, 0.08);
            let b = (0.9, 0.3);
            let c = (0.5, 0.53);
            let d = (0.1, 0.3);
            for (x, y) in [
                (a, b),
                (b, c),
                (c, d),
                (d, a),
                (d, (0.1, 0.73)),
                (b, (0.9, 0.73)),
                (c, (0.5, 0.96)),
                ((0.1, 0.73), (0.5, 0.96)),
                ((0.5, 0.96), (0.9, 0.73)),
            ] {
                line(x, y);
            }
        }
        Icon::Next => {
            line((0.2, 0.5), (0.82, 0.5));
            line((0.58, 0.25), (0.83, 0.5));
            line((0.58, 0.75), (0.83, 0.5));
        }
        Icon::Copy => {
            p.rect_stroke(
                Rect::from_min_max(at(0.3, 0.3), at(0.85, 0.87)),
                3,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((0.15, 0.65), (0.15, 0.12));
            line((0.15, 0.12), (0.68, 0.12));
        }
        Icon::Check => {
            line((0.17, 0.5), (0.4, 0.72));
            line((0.4, 0.72), (0.84, 0.22));
        }
    }
}
pub fn nav(
    ui: &mut egui::Ui,
    label: &str,
    kind: Icon,
    selected: bool,
    compact: bool,
    width: f32,
) -> egui::Response {
    let p = palette(ui.ctx());
    let height = if compact { 56. } else { 42. };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::SelectableLabel, ui.is_enabled(), label)
    });
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.13);
    if selected || hover > 0. {
        ui.painter().rect_filled(
            rect,
            8,
            if selected {
                p.raised
            } else {
                p.raised.gamma_multiply(hover)
            },
        );
    }
    let color = if selected { p.text } else { p.muted };
    let center = if compact {
        Pos2::new(rect.center().x, rect.top() + 17.)
    } else {
        Pos2::new(rect.left() + 22., rect.center().y)
    };
    icon(
        ui.painter(),
        Rect::from_center_size(center, Vec2::splat(18.)),
        kind,
        color,
    );
    if compact {
        ui.painter().text(
            Pos2::new(rect.center().x, rect.bottom() - 13.),
            egui::Align2::CENTER_CENTER,
            label,
            medium(10.5),
            color,
        );
    } else {
        ui.painter().text(
            Pos2::new(rect.left() + 42., rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            medium(13.5),
            color,
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            8,
            Stroke::new(1.0f32, p.accent),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
pub fn icon_button(ui: &mut egui::Ui, kind: Icon, label: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 7, p.raised);
    }
    icon(ui.painter(), rect.shrink(8.), kind, p.muted);
    response
        .on_hover_text(label)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
pub fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let p = palette(ui.ctx());
    ui.add(
        egui::Button::new(RichText::new(label).font(medium(14.)).color(p.on_accent))
            .fill(p.accent)
            .stroke(Stroke::NONE)
            .corner_radius(3)
            .min_size(Vec2::new(0., 32.)),
    )
}
pub fn field(ui: &mut egui::Ui, label: &str, value: &mut String, secret: bool) -> egui::Response {
    let p = palette(ui.ctx());
    ui.label(RichText::new(label).font(font(12.)).color(p.muted));
    ui.scope(|ui| {
        ui.visuals_mut().extreme_bg_color = p.surface;
        ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(1.0f32, p.line);
        ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(1.0f32, p.muted);
        ui.add(
            egui::TextEdit::singleline(value)
                .password(secret)
                .desired_width((ui.available_width() - 20.).max(80.))
                .margin(Vec2::new(10., 7.))
                .font(font(14.)),
        )
    })
    .inner
}

pub fn mono_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("MonoSemibold".into()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Semibold".into()))
}
