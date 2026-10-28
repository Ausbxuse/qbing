//! Compact solve review: persistent playback with progressively disclosed detail.
use crate::{
    cube::Move,
    design::{self, font, medium, mono, palette},
    io::{self, number, value_str},
    notation, recording,
    ui::App,
};
use eframe::egui::{self, Align2, Color32, RichText, Sense, Stroke, Vec2};
use serde_json::{Value, json};

fn label(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(
        RichText::new(text)
            .font(font(12.))
            .color(palette(ui.ctx()).muted),
    );
}
pub(crate) fn phase_color(ui: &egui::Ui, stage: &str) -> Color32 {
    let light = palette(ui.ctx()).bg.r() > 128;
    let colors = if light {
        [0x16bda2, 0x5a8fff, 0xedc94b, 0xa282ee]
    } else {
        [0x3bd6b6, 0x79a6ff, 0xf2d255, 0xba9bff]
    };
    let Some(index) = ["Cross", "F2L", "OLL", "PLL"]
        .iter()
        .position(|s| *s == stage)
    else {
        return palette(ui.ctx()).faint;
    };
    let n = colors[index];
    Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8)
}
pub(crate) fn draw(app: &mut App, ctx: &egui::Context, r: &Value) {
    let p = palette(ctx);
    let width = (ctx.content_rect().width() - 56.).clamp(240., 780.);
    let short = ctx.content_rect().height() < 560.;
    let detail_height = (ctx.content_rect().height() - 532.).clamp(64., 360.);
    let mut close = false;
    let id = egui::Id::new(("solve-review", value_str(r, "id")));
    let response = egui::Modal::new(id)
        .frame(
            egui::Frame::new()
                .fill(p.bg)
                .stroke(Stroke::new(1., p.line))
                .corner_radius(8)
                .inner_margin(16),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            let mut content = |ui: &mut egui::Ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8., 8.);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Solve review")
                            .font(medium(14.))
                            .color(p.text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("×").font(font(20.))).frame(false))
                            .on_hover_text("Close · Esc")
                            .clicked()
                        {
                            close = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Delete").font(font(12.)).color(p.muted),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            app.delete_confirm = true;
                        }
                    });
                });
                let execution = recording::execution_metrics(r);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(io::seconds(io::adjusted(r)))
                            .font(design::mono_bold(32.))
                            .color(p.text),
                    );
                    ui.add_space(8.);
                    for (value, unit) in [
                        (execution.map(|m| m.label()).unwrap_or("—".into()), "tps"),
                        (
                            recording::execution_moves(r)
                                .map(|n| n.to_string())
                                .unwrap_or("—".into()),
                            "moves",
                        ),
                    ] {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(value).font(mono(17.)).color(p.text));
                            label(ui, unit);
                        });
                        ui.add_space(8.);
                    }
                });
                let date = chrono::DateTime::parse_from_rfc3339(value_str(r, "createdAt"))
                    .map(|d| {
                        d.with_timezone(&chrono::Local)
                            .format("%b %-d · %H:%M")
                            .to_string()
                    })
                    .unwrap_or_default();
                ui.horizontal_wrapped(|ui| {
                    label(
                        ui,
                        format!(
                            "{}{}{}",
                            value_str(r, "session"),
                            if date.is_empty() { "" } else { " · " },
                            date
                        ),
                    );
                    if let Some(color) = crate::practice_ui::cross_color(&r["analysis"]) {
                        label(ui, format!("· {color} cross"));
                    }
                });
                if app.delete_confirm {
                    ui.horizontal_wrapped(|ui| {
                        label(ui, "Delete this solve?");
                        if ui.button("Cancel").clicked() {
                            app.delete_confirm = false;
                        }
                        if ui
                            .button(RichText::new("Delete permanently").color(p.error))
                            .clicked()
                        {
                            app.model.delete(value_str(r, "id"));
                            close = true;
                        }
                    });
                }
                ui.separator();
                let moves = r["moves"].as_array().map(Vec::as_slice).unwrap_or(&[]);
                if !moves.is_empty() {
                    advance(app, ctx, r, moves);
                    ui.horizontal(|ui| {
                        let w = if width < 500. { 110. } else { 210. };
                        app.replay
                            .draw(ui, Vec2::new(w, if short { 96. } else { 116. }));
                        ui.allocate_ui_with_layout(
                            Vec2::new((ui.available_width() - 4.).max(80.), 148.),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                phases(app, ui, r, width < 500.);
                            },
                        );
                    });
                    ui.horizontal(|ui| {
                        if step_button(ui, false).clicked() {
                            app.step_review(false);
                        }
                        if playback_button(ui, app.playing.is_some()).clicked() {
                            app.toggle_review_playback();
                        }
                        if step_button(ui, true).clicked() {
                            app.step_review(true);
                        }
                        ui.spacing_mut().slider_width = (ui.available_width() - 122.).max(40.);
                        let mut index = app.replay_index;
                        if ui
                            .scope(|ui| {
                                ui.visuals_mut().widgets.inactive.bg_fill = p.line;
                                ui.visuals_mut().widgets.hovered.bg_fill =
                                    p.accent.gamma_multiply(0.4);
                                ui.add(
                                    egui::Slider::new(&mut index, 0..=moves.len())
                                        .show_value(false)
                                        .trailing_fill(true),
                                )
                            })
                            .inner
                            .on_hover_text("Seek recorded turns")
                            .changed()
                        {
                            app.seek_review_turn(index);
                        }
                        let time = app
                            .playing
                            .map(|s| s.elapsed().as_secs_f64() * 1000.)
                            .unwrap_or(app.review_playhead)
                            .min(number(r, "duration"));
                        ui.label(
                            RichText::new(format!(
                                "{} / {}",
                                io::seconds(time),
                                io::seconds(number(r, "duration"))
                            ))
                            .font(mono(12.))
                            .color(p.muted),
                        );
                    });
                }
                ui.separator();
                if app.review_reconstruction {
                    app.review_tab = 0;
                    app.review_reconstruction = false;
                }
                ui.horizontal(|ui| {
                    for (i, name) in ["Moves", "Analysis"].iter().enumerate() {
                        ui.selectable_value(
                            &mut app.review_tab,
                            i,
                            RichText::new(*name).font(medium(12.)),
                        );
                    }
                });
                egui::ScrollArea::vertical()
                    .id_salt(("review-details", app.review_tab))
                    .max_height(detail_height)
                    .min_scrolled_height(80.)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_height(detail_height);
                        match app.review_tab {
                            0 => turns(app, ui, r, &mut close),
                            _ => insights(app, ui, r, &mut close),
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    for penalty in ["", "+2", "DNF"] {
                        if ui
                            .selectable_label(
                                value_str(r, "penalty") == penalty,
                                RichText::new(if penalty.is_empty() { "OK" } else { penalty })
                                    .font(medium(12.)),
                            )
                            .clicked()
                        {
                            let mut edit = r.clone();
                            edit["penalty"] = json!(penalty);
                            app.model.save(edit.clone());
                            app.review = Some(edit);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        label(
                            ui,
                            if value_str(r, "source") == "manual" {
                                "Manual"
                            } else if execution.is_none() {
                                "Unverified timing"
                            } else if execution.is_some_and(|m| m.estimated) {
                                "≈ Estimated timing"
                            } else {
                                "Bluetooth"
                            },
                        );
                    });
                });
            };
            if short {
                egui::ScrollArea::vertical()
                    .id_salt("review-short-window")
                    .max_height((ctx.content_rect().height() - 64.).max(160.))
                    .show(ui, content);
            } else {
                content(ui);
            }
        });
    if close || response.should_close() {
        app.review = None;
        app.playing = None;
        app.delete_confirm = false;
    }
}
fn advance(app: &mut App, ctx: &egui::Context, r: &Value, moves: &[Value]) {
    if let Some(start) = app.playing {
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        while app.replay_index < moves.len() && number(&moves[app.replay_index], "t") <= elapsed {
            let e = &moves[app.replay_index];
            if let Ok(m) = Move::parse(value_str(e, "move")) {
                let uncertain = e["recovered"] == true
                    || r["quality"].as_array().is_some_and(|f| {
                        f.iter().any(|v| v == "clock-reset" || v == "missing-moves")
                    });
                app.replay.push_timed(m, e["t"].as_f64(), uncertain);
            }
            app.replay_index += 1;
        }
        if app.replay_index == moves.len() && !app.replay.animating() {
            app.review_playhead = number(r, "duration");
            app.playing = None;
        }
        ctx.request_repaint();
    }
    let time = app
        .playing
        .map(|s| s.elapsed().as_secs_f64() * 1000.)
        .unwrap_or(app.review_playhead)
        .min(number(r, "duration"));
    app.replay
        .set_recorded_orientation(r, time, app.playing.is_some());
}
fn phases(app: &mut App, ui: &mut egui::Ui, r: &Value, compact: bool) {
    ui.spacing_mut().item_spacing.y = 2.;
    let p = palette(ui.ctx());
    let measured = recording::execution_metrics(r).is_some();
    for stage in ["Cross", "F2L", "OLL", "PLL"] {
        let split = r["analysis"]["splits"]
            .as_array()
            .and_then(|s| s.iter().find(|s| s["stage"] == stage));
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.), Sense::click());
        if app.review_phase.as_deref() == Some(stage) || response.hovered() {
            ui.painter()
                .rect_filled(rect, 3, p.accent.gamma_multiply(0.07));
        }
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min + Vec2::new(0., 4.), Vec2::new(2., 14.)),
            1,
            phase_color(ui, stage),
        );
        ui.painter().text(
            rect.min + Vec2::new(10., 10.),
            Align2::LEFT_CENTER,
            stage,
            medium(12.),
            p.muted,
        );
        if let Some(s) = split {
            let duration = number(s, "time");
            let before = r["analysis"]["splits"]
                .as_array()
                .unwrap()
                .iter()
                .take_while(|s| s["stage"] != stage)
                .map(|s| number(s, "rawMoves") as usize)
                .sum::<usize>();
            let pause = if stage == "Cross" {
                0.
            } else {
                r["moves"]
                    .as_array()
                    .and_then(|m| m.get(before))
                    .map(|m| (number(m, "t") - number(s, "start")).clamp(0., duration.max(0.)))
                    .unwrap_or(0.)
            };
            let timed = measured && s["time"].as_f64().is_some_and(|t| t.is_finite() && t >= 0.);
            let total = if timed { Some(duration) } else { None };
            let estimated = recording::split_timing_estimated(r, stage);
            let metrics = format!(
                "{} tps  ·  {} moves",
                if measured {
                    s["tps"]
                        .as_f64()
                        .map(|n| {
                            format!(
                                "{}{n:.2}",
                                if recording::split_timing_estimated(r, stage) {
                                    "≈"
                                } else {
                                    ""
                                }
                            )
                        })
                        .unwrap_or("—".into())
                } else {
                    "—".into()
                },
                number(s, "moves") as usize
            );
            let metric_width = ui
                .painter()
                .layout_no_wrap(metrics.clone(), mono(12.), p.muted)
                .size()
                .x;
            let time_width =
                crate::phase_time::width(ui, total, pause, stage == "Cross", estimated, 14.);
            let show_metrics =
                !compact && rect.width() >= 62. + time_width + 16. + metric_width + 10.;
            let x = if show_metrics {
                rect.right() - 10. - metric_width - 16.
            } else {
                rect.right() - 10.
            };
            crate::phase_time::paint(
                ui,
                egui::pos2(x, rect.top() + 1.),
                total,
                pause,
                stage == "Cross",
                estimated,
                14.,
            );
            if show_metrics {
                ui.painter().text(
                    egui::pos2(rect.right() - 10., rect.top() + 10.),
                    Align2::RIGHT_CENTER,
                    metrics,
                    mono(12.),
                    p.muted,
                );
            }
            if response.clicked() {
                app.seek_review_phase(stage);
            }
            response.on_hover_text(format!(
                "{stage} · {} s · {} moves\nSelect to replay",
                io::seconds(duration),
                number(s, "moves")
            ));
        } else {
            ui.painter().text(
                egui::pos2(rect.right() - 8., rect.center().y),
                Align2::RIGHT_CENTER,
                "—",
                mono(12.),
                p.faint,
            );
        }
    }
}
fn turns(app: &mut App, ui: &mut egui::Ui, r: &Value, close: &mut bool) {
    let p = palette(ui.ctx());
    let moves = r["moves"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    if moves.is_empty() {
        label(ui, "No recorded turns for this solve.");
        return;
    }
    let data = crate::native_ui::cached_tool(
        app,
        ui,
        app.review_solver_key.clone(),
        "native-review",
        json!({"record":r}),
    );
    ui.horizontal(|ui| {
        for (raw, label) in [(false, "Notation"), (true, "Raw")] {
            if ui
                .selectable_value(
                    &mut app.review_raw,
                    raw,
                    RichText::new(label).font(font(12.)),
                )
                .changed()
            {
                app.review_step_cursor = None;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Copy raw").clicked() {
                ui.ctx().copy_text(
                    moves
                        .iter()
                        .map(|m| value_str(m, "move"))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
            if data.is_none() {
                ui.spinner();
            }
        });
    });
    let turns = notation::with_rotations(
        if app.review_raw {
            notation::raw_turns(r)
        } else {
            notation::reconstruct_for_view(r)
        },
        r,
    );
    for (stage, begin, end) in recording::reconstruction_ranges(r) {
        ui.add_space(6.);
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(2., 14.), Sense::hover());
            ui.painter().rect_filled(rect, 1, phase_color(ui, &stage));
            if ui
                .add(egui::Button::new(RichText::new(&stage).font(medium(12.))).frame(false))
                .on_hover_text("Replay this step")
                .clicked()
            {
                app.seek_review_turn(begin);
            }
            label(
                ui,
                format!("{} moves", recording::count_moves(&moves[begin..end])),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4., 4.);
            for turn in turns.iter().filter(|t| {
                let i = t.raw_start.min(moves.len().saturating_sub(1));
                i >= begin && i < end
            }) {
                let selected = if turn.rotation {
                    turn.elapsed
                        .is_some_and(|t| (t - app.review_playhead).abs() < 0.5)
                        && app.playing.is_none()
                } else {
                    app.replay_index > turn.raw_start && app.replay_index <= turn.raw_end
                };
                let (rect, response) = ui.allocate_exact_size(Vec2::new(56., 40.), Sense::click());
                if selected || response.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        4,
                        if selected {
                            p.accent.gamma_multiply(0.12)
                        } else {
                            p.surface
                        },
                    );
                }
                let text = format!(
                    "{}{}{}",
                    if turn.rotation { "◈ " } else { "" },
                    turn.move_text,
                    if turn.inferred { "*" } else { "" }
                );
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.top() + 10.),
                    Align2::CENTER_CENTER,
                    text,
                    mono(14.),
                    if selected || turn.rotation {
                        p.accent
                    } else {
                        p.text
                    },
                );
                let uncertain = turn.rotation
                    || turn.recovered
                    || recording::turn_timing_estimated(r, turn.raw_end.saturating_sub(1));
                let elapsed = turn
                    .elapsed
                    .map(|t| format!("{}{:.3}", if uncertain { "≈" } else { "" }, t / 1000.))
                    .unwrap_or("—".into());
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.top() + 29.),
                    Align2::CENTER_CENTER,
                    &elapsed,
                    mono(12.),
                    p.faint,
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("{} at {} seconds", turn.move_text, elapsed),
                    )
                });
                if response.clicked() {
                    if turn.rotation {
                        app.seek_review_time(turn.elapsed.unwrap_or(0.));
                    } else {
                        app.seek_review_turn(turn.raw_end);
                    }
                }
                response.on_hover_text(if turn.rotation {
                    "Recorded snapped rotation · local gyro timing"
                } else if turn.inferred {
                    "Inferred slice · raw sensor turns are preserved"
                } else {
                    "Select to seek · elapsed seconds"
                });
            }
        });
        if let Some(Ok(data)) = &data {
            for group in data["groups"].as_array().into_iter().flatten().filter(|g| {
                value_str(g, "stage") == stage && number(g, "rawEnd") > number(g, "rawStart")
            }) {
                let title = if stage == "F2L" {
                    format!(
                        "{} · {}",
                        value_str(group, "label"),
                        value_str(group, "caseName")
                    )
                } else if stage == "Cross" {
                    "Cross solutions".into()
                } else {
                    value_str(group, "caseName").into()
                };
                egui::CollapsingHeader::new(RichText::new(title).font(font(12.)))
                    .id_salt(("alternatives", &stage, number(group, "rawStart") as usize))
                    .show(ui, |ui| {
                        for variant in group["variants"].as_array().into_iter().flatten() {
                            ui.horizontal_wrapped(|ui| {
                                label(
                                    ui,
                                    format!(
                                        "{} · {} moves",
                                        value_str(variant, "name"),
                                        number(variant, "moves")
                                    ),
                                );
                                if ui.link("Copy").clicked() {
                                    ui.ctx()
                                        .copy_text(value_str(variant, "viewAlgorithm").into());
                                }
                                if !value_str(variant, "caseId").is_empty()
                                    && ui.link("Practice").clicked()
                                {
                                    app.model.prepare_case(
                                        value_str(variant, "caseId"),
                                        variant["variantIndex"].as_u64().unwrap_or(0) as usize,
                                    );
                                    app.page = "Practice".into();
                                    *close = true;
                                }
                            });
                            ui.label(
                                RichText::new(value_str(variant, "viewAlgorithm"))
                                    .font(mono(14.))
                                    .color(p.text),
                            );
                        }
                    });
            }
        }
    }
}
fn insights(app: &mut App, ui: &mut egui::Ui, r: &Value, close: &mut bool) {
    crate::review_analysis::draw(app, ui, r, close);
}
fn step_button(ui: &mut egui::Ui, forward: bool) -> egui::Response {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::new(28., 28.), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4, p.raised);
    }
    let center = rect.center();
    let sign = if forward { 1. } else { -1. };
    let point = |x: f32, y: f32| center + Vec2::new(x * sign, y);
    ui.painter().rect_filled(
        egui::Rect::from_center_size(point(5., 0.), Vec2::new(2., 12.)),
        0,
        p.text,
    );
    ui.painter().add(egui::Shape::convex_polygon(
        vec![point(-5., -6.), point(-5., 6.), point(3., 0.)],
        p.text,
        Stroke::NONE,
    ));
    let title = if forward {
        "Next turn"
    } else {
        "Previous turn"
    };
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), title)
    });
    response.on_hover_text(if forward {
        "Next turn · →"
    } else {
        "Previous turn · ←"
    })
}

fn playback_button(ui: &mut egui::Ui, playing: bool) -> egui::Response {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::new(38., 28.), Sense::click());
    ui.painter().rect_filled(
        rect,
        4,
        if response.hovered() {
            p.raised
        } else {
            p.surface
        },
    );
    ui.painter()
        .rect_stroke(rect, 4, Stroke::new(1., p.line), egui::StrokeKind::Inside);
    let c = rect.center();
    if playing {
        for x in [-3., 3.] {
            ui.painter().rect_filled(
                egui::Rect::from_center_size(c + Vec2::new(x, 0.), Vec2::new(2., 10.)),
                0,
                p.text,
            );
        }
    } else {
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                c + Vec2::new(-3., -5.),
                c + Vec2::new(-3., 5.),
                c + Vec2::new(5., 0.),
            ],
            p.text,
            Stroke::NONE,
        ));
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            if playing { "Pause" } else { "Play" },
        )
    });
    response.on_hover_text("Space · play / pause")
}
