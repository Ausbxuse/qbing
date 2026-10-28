use crate::{
    bluetooth::Command,
    cube::scramble_progress,
    design::{self, Icon, font, mono, palette},
    io::{self, adjusted, seconds, value_str},
    ui::App,
};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use serde_json::Value;
use std::time::{Duration, Instant};

mod guidance;
mod phases;

pub(crate) fn cross_color(analysis: &Value) -> Option<&'static str> {
    if analysis["moves"].as_u64().unwrap_or(0) == 0 {
        return None;
    }
    Some(match value_str(analysis, "cross") {
        "U" => "white",
        "D" => "yellow",
        "R" => "red",
        "L" => "orange",
        "F" => "green",
        "B" => "blue",
        _ => return None,
    })
}

pub(crate) fn cross_description(analysis: &Value) -> String {
    let Some(color) = cross_color(analysis) else {
        return String::new();
    };
    let detail = match (
        value_str(analysis, "crossSource"),
        value_str(analysis, "crossEvidence"),
    ) {
        ("detected", "f2l") => "detected from the first completed F2L layer",
        ("detected", "cross") => "provisional until the F2L layer is complete",
        ("ambiguous", _) => "multiple colors fit this recording; the preferred color breaks ties",
        _ => "selected color for drills and ambiguous solves",
    };
    format!("{color} cross · {detail}")
}

/// The same time-ranked, trimmed window supplies every column. Each sensor
/// metric uses its available samples, never another window or invented zeros.
pub(crate) struct AverageMetrics {
    pub time: Option<f64>,
    pub time_estimated: bool,
    pub tps: Option<f64>,
    pub moves: Option<f64>,
    pub tps_estimated: bool,
    pub tps_samples: usize,
    pub move_samples: usize,
    pub retained: usize,
}
impl AverageMetrics {
    pub fn time_label(&self) -> String {
        self.time
            .map(|time| {
                format!(
                    "{}{}",
                    if self.time_estimated && time.is_finite() {
                        "≈ "
                    } else {
                        ""
                    },
                    seconds(time)
                )
            })
            .unwrap_or("—".into())
    }
    pub fn tps_label(&self) -> String {
        self.tps
            .map(|tps| format!("{}{tps:.2}", if self.tps_estimated { "≈ " } else { "" }))
            .unwrap_or("—".into())
    }
    pub fn sample_detail(&self) -> String {
        format!(
            "TPS: {}/{} retained solves{}. Moves: {}/{} retained solves.",
            self.tps_samples,
            self.retained,
            if self.tps_estimated {
                " (includes estimated duration)"
            } else {
                ""
            },
            self.move_samples,
            self.retained
        )
    }
}
pub(crate) fn average_metrics(records: &[Value], n: usize) -> AverageMetrics {
    let empty = AverageMetrics {
        time: None,
        time_estimated: false,
        tps: None,
        moves: None,
        tps_estimated: false,
        tps_samples: 0,
        move_samples: 0,
        retained: 0,
    };
    if records.len() < n || n == 0 {
        return empty;
    }
    let mut retained = records[records.len() - n..].iter().collect::<Vec<_>>();
    retained.sort_by(|a, b| adjusted(a).total_cmp(&adjusted(b)));
    let trim = if n >= 5 {
        (n as f64 * 0.05).ceil() as usize
    } else {
        0
    };
    let retained = &retained[trim..n - trim];
    let time = Some(retained.iter().map(|r| adjusted(r)).sum::<f64>() / retained.len() as f64);
    let rates = retained
        .iter()
        .filter_map(|r| crate::recording::execution_metrics(r))
        .collect::<Vec<_>>();
    let counts = retained
        .iter()
        .filter_map(|r| crate::recording::execution_moves(r))
        .collect::<Vec<_>>();
    AverageMetrics {
        time,
        time_estimated: retained
            .iter()
            .any(|r| crate::recording::execution_duration_estimated(r)),
        tps: (!rates.is_empty())
            .then(|| rates.iter().map(|metric| metric.tps).sum::<f64>() / rates.len() as f64),
        moves: (!counts.is_empty())
            .then(|| counts.iter().sum::<usize>() as f64 / counts.len() as f64),
        tps_estimated: rates.iter().any(|metric| metric.estimated),
        tps_samples: rates.len(),
        move_samples: counts.len(),
        retained: retained.len(),
    }
}
fn at(ui: &mut egui::Ui, rect: Rect, f: impl FnOnce(&mut egui::Ui)) {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
        |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(rect));
            f(ui);
        },
    );
}
fn centered_row(ui: &mut egui::Ui, width: f32, f: impl FnOnce(&mut egui::Ui)) {
    let rect = Rect::from_center_size(
        ui.max_rect().center(),
        Vec2::new(width.min(ui.max_rect().width()), ui.max_rect().height()),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        f,
    );
}
fn label(
    ui: &egui::Ui,
    pos: Pos2,
    text: impl Into<String>,
    size: f32,
    color: Color32,
    align: Align2,
) {
    ui.painter()
        .text(pos, align, text.into(), font(size), color);
}
fn rate(value: &Value) -> String {
    value["tps"]
        .as_f64()
        .filter(|n| n.is_finite())
        .map(|n| {
            format!(
                "{}{n:0.2}",
                if value["estimated"] == true {
                    "≈ "
                } else {
                    ""
                }
            )
        })
        .unwrap_or("—".into())
}

impl App {
    fn is_solving(&self) -> bool {
        self.keyboard.running()
            || self
                .model
                .active
                .as_ref()
                .is_some_and(|a| a.started.is_some())
    }
    fn practice_analysis(&self) -> Value {
        self.practice_metrics.clone()
    }
    fn compute_practice_analysis(&self) -> Value {
        if self.keyboard.running()
            || self.keyboard.holding()
            || self.model.countdown.is_some()
            || self.model.cue_ready
        {
            Value::Null
        } else if self
            .model
            .active
            .as_ref()
            .is_some_and(|a| a.started.is_some())
        {
            self.model.live_analysis()
        } else {
            self.model
                .latest
                .as_ref()
                .and_then(|r| {
                    let metric = crate::recording::execution_metrics(r)?;
                    let mut analysis = if r["analysis"].is_object() {
                        r["analysis"].clone()
                    } else {
                        crate::model::analysis(r)
                    };
                    analysis["tps"] = metric.tps.into();
                    analysis["moves"] =
                        crate::recording::count_moves(r["moves"].as_array()?).into();
                    analysis["estimated"] = metric.estimated.into();
                    if let Some(splits) = analysis["splits"].as_array_mut() {
                        for split in splits {
                            split["estimated"] = crate::recording::split_timing_estimated(
                                r,
                                value_str(split, "stage"),
                            )
                            .into();
                        }
                    }
                    Some(analysis)
                })
                .unwrap_or(Value::Null)
        }
    }
    fn session_records(&self) -> Vec<Value> {
        self.model
            .records
            .iter()
            .filter(|r| {
                value_str(r, "kind") == "solve"
                    && value_str(r, "session") == self.model.settings.session
            })
            .cloned()
            .collect()
    }
    pub(crate) fn practice(&mut self, ui: &mut egui::Ui) {
        // The timer belongs to the visible window, never to an oversized scroll canvas.
        let rect = ui.available_rect_before_wrap();
        self.practice_layout(ui, rect);
    }
    fn practice_layout(&mut self, ui: &mut egui::Ui, rect: Rect) {
        // Every value on this screen uses the same device-clock snapshot.
        self.practice_metrics = self.compute_practice_analysis();
        let p = palette(ui.ctx());
        let viewport = ui.ctx().content_rect();
        let compact = viewport.width() < 760.;
        let short = viewport.height() < 680.;
        let solving = self.is_solving();
        let column = Rect::from_center_size(
            Pos2::new(viewport.center().x, rect.center().y),
            Vec2::new(rect.width().min(1040.), rect.height()),
        );
        let activity_top = if short {
            rect.top() + 8.
        } else if compact {
            rect.top() + 22.
        } else {
            rect.top() + 34.
        };
        let cube_size = if compact {
            84.
        } else if short {
            100.
        } else {
            130.
        };
        let gap = if compact { 14. } else { 32. };
        let cube_rect = Rect::from_min_size(
            Pos2::new(
                column.right() - cube_size,
                activity_top + if compact { 18. } else { 0. },
            ),
            Vec2::splat(cube_size),
        );
        let notation_rect = Rect::from_min_max(
            column.left_top() + Vec2::new(0., activity_top - column.top()),
            Pos2::new(cube_rect.left() - gap, viewport.center().y - 60.),
        );
        if !solving {
            at(ui, notation_rect, |ui| {
                self.scramble_band(ui);
            });
        }
        at(ui, cube_rect, |ui| {
            if solving {
                ui.set_opacity(0.65);
            }
            self.animator.draw(ui, cube_rect.size());
        });
        if !solving {
            let controls = Rect::from_min_size(
                Pos2::new(cube_rect.left(), cube_rect.bottom() + 2.),
                Vec2::new(cube_size, 28.),
            );
            at(ui, controls, |ui| self.cube_reset_controls(ui));
        }
        let timer_size = if compact {
            if short { 92. } else { 106. }
        } else if short {
            112.
        } else {
            140.
        };
        self.timer_center(ui, viewport.center(), timer_size);

        let result_width = column.width().min(900.);
        let result_height = if compact {
            if short { 132. } else { 155. }
        } else {
            170.
        };
        let metrics_bottom = viewport.center().y + timer_size * 0.50 + 31.;
        let result_top = (rect.bottom() - result_height - if short { 0. } else { 35. })
            .max(metrics_bottom + if short { 12. } else { 24. });
        let result = Rect::from_min_size(
            Pos2::new(viewport.center().x - result_width / 2., result_top),
            Vec2::new(result_width, result_height),
        );
        ui.painter().line_segment(
            [result.left_top(), result.right_top()],
            Stroke::new(0.75_f32, p.line),
        );
        let averages_height = if compact {
            if short { 50. } else { 58. }
        } else {
            63.
        };
        if !solving {
            let records = self.session_records();
            self.practice_averages(
                ui,
                Rect::from_min_size(result.min, Vec2::new(result.width(), averages_height)),
                &records,
                compact,
            );
        }
        // Keep phases anchored when the solve starts: only unrelated controls disappear.
        self.phase_strip(
            ui,
            Rect::from_min_max(result.min + Vec2::new(0., averages_height), result.max),
        );
        if self.isolated {
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("practice-secondary-bounds"),
                    [result, cube_rect],
                )
            });
        }
        if !solving && !short {
            let message = if !self.model.message.is_empty() {
                self.model.message.clone()
            } else if !self.model.connected {
                "Hold Space · release to start · Space to stop".into()
            } else {
                String::new()
            };
            label(
                ui,
                Pos2::new(column.left(), rect.bottom() - 8.),
                message,
                11.,
                p.faint,
                Align2::LEFT_BOTTOM,
            );
        }
    }
    fn cube_reset_controls(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        let center = ui.max_rect().center();
        for (i, tooltip) in [
            "Reset gyro — hold white up and green toward you",
            "Reset cube — confirm the physical cube is solved",
        ]
        .into_iter()
        .enumerate()
        {
            let rect = Rect::from_center_size(
                center + Vec2::new(if i == 0 { -18. } else { 18. }, 0.),
                Vec2::splat(28.),
            );
            let response = ui.interact(rect, ui.id().with(("cube-reset", i)), egui::Sense::click());
            let enabled = i == 0 || self.model.connected;
            let color = if enabled && (response.hovered() || response.has_focus()) {
                p.text
            } else {
                p.faint
            };
            if i == 0 {
                let c = rect.center();
                ui.painter()
                    .circle_stroke(c, 4.5, Stroke::new(1_f32, color));
                for delta in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                    ui.painter()
                        .line_segment([c + delta * 6., c + delta * 8.], Stroke::new(1_f32, color));
                }
            } else {
                design::icon(ui.painter(), rect.shrink(7.), Icon::Cube, color);
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tooltip)
            });
            if enabled
                && (response.clicked()
                    || response.has_focus()
                        && ui.input(|i| {
                            i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)
                        }))
            {
                if i == 0 {
                    self.animator.recenter_orientation();
                } else {
                    self.reset_confirm = true;
                }
            }
            response.on_hover_text(tooltip);
        }
    }
    fn practice_averages(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        records: &[Value],
        compact: bool,
    ) {
        let p = palette(ui.ctx());
        let actions_width = if compact { 150. } else { 220. };
        let averages_width = (rect.width() - actions_width - 12.).min(480.);
        for (i, n) in [5, 12].into_iter().enumerate() {
            let average = average_metrics(records, n);
            let x = rect.left() + i as f32 * averages_width / 2.;
            label(
                ui,
                Pos2::new(x, rect.top() + 21.),
                format!("ao{n}"),
                11.,
                p.muted,
                Align2::LEFT_CENTER,
            );
            label(
                ui,
                Pos2::new(x + 30., rect.top() + 21.),
                average.time_label(),
                if compact { 16. } else { 18. },
                p.text,
                Align2::LEFT_CENTER,
            );
            let details = format!(
                "{} tps · {} moves",
                average.tps_label(),
                average
                    .moves
                    .map(|n| format!("{n:.1}"))
                    .unwrap_or("—".into())
            );
            label(
                ui,
                Pos2::new(x, rect.top() + 43.),
                details,
                if compact { 11. } else { 12. },
                p.muted,
                Align2::LEFT_CENTER,
            );
            ui.interact(
                Rect::from_min_size(
                    Pos2::new(x, rect.top()),
                    Vec2::new(averages_width / 2., rect.height()),
                ),
                ui.id().with(("average", n)),
                egui::Sense::hover(),
            )
            .on_hover_text(format!(
                "Same time-ranked retained solves for all metrics. {}",
                average.sample_detail()
            ));
        }
        at(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.right() - actions_width, rect.top() + 9.),
                Vec2::new(actions_width, 40.),
            ),
            |ui| self.practice_actions(ui, self.training.is_some()),
        );
    }
    fn scramble_band(&mut self, ui: &mut egui::Ui) -> f32 {
        let p = palette(ui.ctx());
        let rect = ui.max_rect();
        let target = self.model.target.clone();
        let solving = self.is_solving();
        let top = rect.top();
        let controls =
            Rect::from_min_size(Pos2::new(rect.right() - 66., top - 4.), Vec2::new(66., 32.));
        at(ui, controls, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                if design::icon_button(ui, Icon::Next, "Generate another scramble").clicked() {
                    self.training = None;
                    self.model.new_scramble();
                }
                if let Some(target) = &target {
                    let copied = self.copied_until.is_some_and(|t| Instant::now() < t);
                    if design::icon_button(
                        ui,
                        if copied { Icon::Check } else { Icon::Copy },
                        "Copy original scramble",
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
                        self.copied_until = Some(Instant::now() + Duration::from_secs(2));
                    }
                }
            });
        });
        at(
            ui,
            Rect::from_min_size(rect.min, Vec2::new((rect.width() - 72.).max(0.), 28.)),
            |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.;
                    ui.spacing_mut().button_padding = Vec2::new(0., 4.);
                    ui.label(RichText::new("3 × 3").font(font(12.)).color(p.muted));
                    ui.label(RichText::new("/").font(font(12.)).color(p.faint));
                    if let Some(run) = &self.training {
                        let clean = self
                            .model
                            .records
                            .iter()
                            .filter(|r| {
                                value_str(r, "caseId") == run.case_id
                                    && !run.baseline.contains(value_str(r, "id"))
                                    && io::timed(r)
                            })
                            .count();
                        let name = self
                            .model
                            .catalog
                            .cases
                            .iter()
                            .find(|c| c.id == run.case_id)
                            .map(|c| c.name.as_str())
                            .unwrap_or("Case");
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{name} · {}/5", clean.min(5)))
                                    .font(font(12.))
                                    .color(p.muted),
                            )
                            .truncate(),
                        );
                        return;
                    }
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
                    let mut session = self.model.settings.session.clone();
                    let mut sessions = self
                        .model
                        .records
                        .iter()
                        .map(|r| value_str(r, "session").to_owned())
                        .collect::<Vec<_>>();
                    sessions.push(session.clone());
                    sessions.sort();
                    sessions.dedup();
                    egui::ComboBox::from_id_salt("practice-session")
                        .width((rect.width() - 140.).clamp(90., 180.))
                        .selected_text(RichText::new(&session).font(font(12.)).color(p.muted))
                        .show_ui(ui, |ui| {
                            for name in sessions {
                                ui.selectable_value(&mut session, name.clone(), name);
                            }
                            if ui.button("Manage sessions").clicked() {
                                self.page = "Settings".into();
                            }
                            if ui.button("Enter a time manually").clicked() {
                                self.manual = true;
                            }
                        });
                    if session != self.model.settings.session {
                        self.model.set_session(&session);
                    }
                });
            },
        );
        let Some(target) = target else {
            if self.model.scramble_loading {
                at(
                    ui,
                    Rect::from_center_size(Pos2::new(rect.center().x, top + 52.), Vec2::splat(20.)),
                    |ui| {
                        ui.spinner().on_hover_text("Preparing the next scramble");
                    },
                );
            }
            return top + 68.;
        };
        let actual = if solving {
            Some(target.cube)
        } else {
            self.model.state
        };
        let tokens = guidance::tokens(
            &target.moves,
            actual,
            self.model.guidance_checkpoint,
            &self.model.recovery,
        );
        let size = if rect.width() < 400. {
            16.
        } else if rect.width() < 600. {
            18.
        } else {
            22.
        };
        let stats = if solving {
            self.model
                .active
                .as_ref()
                .map(|active| {
                    crate::scramble::stats(&active.scramble_moves, &active.scramble_quality)
                })
                .unwrap_or(Value::Null)
        } else if self.model.active.is_none() {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
            self.model.scramble.live_stats()
        } else {
            self.model.scramble.stats()
        };
        let tps_text = format!("{} tps", rate(&stats));
        // The rate follows the final notation row, rather than stealing a column.
        let base = Pos2::new(rect.left() + rect.width() / 2., top + 39.);
        let rate_width = ui
            .painter()
            .layout_no_wrap(tps_text.clone(), font(12.), p.muted)
            .size()
            .x;
        let (bottom, edge) = paint_moves(ui, &tokens, base, rect.width(), size, rate_width + 18.);
        let rate_pos = if edge.x + rate_width + 18. < rect.right() {
            Pos2::new(edge.x + 12., edge.y + size * 0.65)
        } else {
            Pos2::new(rect.left(), bottom + 5.)
        };
        label(ui, rate_pos, tps_text, 12., p.muted, Align2::LEFT_CENTER);
        ui.interact(
            Rect::from_min_size(rate_pos - Vec2::new(0., 12.), Vec2::new(90., 24.)),
            ui.id().with("scramble-live-rate"),
            egui::Sense::hover(),
        )
        .on_hover_text("Preparation speed, including corrections. A half turn counts once.");
        at(
            ui,
            Rect::from_min_size(
                Pos2::new(rate_pos.x + rate_width + 12., rate_pos.y - 13.),
                Vec2::new(26., 26.),
            ),
            |ui| self.scramble_guide(ui),
        );
        bottom + 4.
    }
    fn scramble_guide(&self, ui: &mut egui::Ui) {
        if self.is_solving() {
            return;
        }
        let p = palette(ui.ctx());
        let rect = ui.max_rect();
        if self.model.recovery_busy {
            at(
                ui,
                Rect::from_center_size(rect.center(), Vec2::splat(20.)),
                |ui| {
                    ui.spinner()
                        .on_hover_text("Finding a correction back to the scramble");
                },
            );
        } else if let Some(error) = &self.model.recovery_error {
            label(
                ui,
                rect.center(),
                error,
                12.,
                p.error,
                Align2::CENTER_CENTER,
            );
        } else if self.model.recovery.is_empty()
            && let (Some(state), Some(target)) = (self.model.state, self.model.target.as_ref())
            && state == target.cube
        {
            let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(18.));
            design::icon(ui.painter(), icon_rect, Icon::Check, p.green);
            let response = ui.interact(
                icon_rect.expand(6.),
                ui.id().with("scramble-ready"),
                egui::Sense::hover(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Label,
                    true,
                    "Scramble matched; turn to start",
                )
            });
            response.on_hover_text("Scramble matched. Your first turn starts the timer.");
        }
    }
    fn timer_center(&mut self, ui: &mut egui::Ui, center: Pos2, size: f32) {
        let p = palette(ui.ctx());
        let solving = self.is_solving();
        let analysis = self.practice_analysis();
        if self.isolated {
            ui.ctx().data_mut(|data| {
                data.insert_temp(egui::Id::new("practice-metrics"), analysis.clone())
            });
        }
        let status = if self.keyboard.ready() {
            "release to start"
        } else if self.keyboard.holding() {
            "hold…"
        } else if self.model.countdown.is_some() {
            "get ready"
        } else if solving {
            "solving"
        } else if self.model.active.is_some() {
            "turn to start"
        } else if self.model.cue_ready {
            "case ready"
        } else if self.model.latest.is_some() {
            "last solve"
        } else {
            "ready"
        };
        let partial_guidance = !solving
            && self
                .model
                .state
                .zip(self.model.target.as_ref())
                .is_some_and(|(state, target)| scramble_progress(state, &target.moves).partial);
        if (self.keyboard.holding() || self.keyboard.ready() || self.model.countdown.is_some())
            && self.model.recovery.is_empty()
            && !self.model.recovery_busy
            && !partial_guidance
        {
            label(
                ui,
                center - Vec2::new(0., size * 0.71),
                status,
                12.,
                if self.keyboard.ready() {
                    p.green
                } else {
                    p.muted
                },
                Align2::CENTER_CENTER,
            );
        }
        let text = self
            .model
            .countdown
            .map(|t| (3 - t.elapsed().as_secs().min(2)).to_string())
            .unwrap_or_else(|| {
                seconds(self.keyboard.elapsed().unwrap_or_else(|| {
                    if solving {
                        analysis["duration"]
                            .as_f64()
                            .unwrap_or_else(|| self.model.elapsed())
                    } else {
                        self.model.latest.as_ref().map(adjusted).unwrap_or(0.)
                    }
                }))
            });
        let color = if self.keyboard.ready() {
            p.green
        } else {
            p.text
        };
        let number_rect = design::paint_timer(ui, center, &text, size, color);
        if !solving
            && self.model.countdown.is_none()
            && !self.keyboard.holding()
            && self
                .model
                .latest
                .as_ref()
                .is_some_and(crate::recording::execution_duration_estimated)
        {
            label(
                ui,
                Pos2::new(number_rect.left() - 13., center.y),
                "≈",
                20.,
                p.muted,
                Align2::RIGHT_CENTER,
            );
        }
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new("practice-timer-bounds"), number_rect));
        let response = ui.interact(
            number_rect.expand(8.),
            ui.id().with("timer-number"),
            egui::Sense::click(),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
        if response.clicked() && !solving {
            if let Some(last) = self.model.latest.clone() {
                self.open_review(last);
            }
        }
        let moves = analysis["moves"]
            .as_u64()
            .map(|n| n.to_string())
            .unwrap_or("—".into());
        let metrics_y = center.y + size * 0.50 + 18.;
        let groups = [(rate(&analysis), "tps"), (moves, "moves")].map(|(value, unit)| {
            let number = ui.painter().layout_no_wrap(value, font(18.), p.text);
            let label = ui.painter().layout_no_wrap(unit.into(), font(12.), p.muted);
            let width = number.size().x + 7. + label.size().x;
            (number, label, width)
        });
        let total = groups.iter().map(|g| g.2).sum::<f32>() + 28.;
        let mut x = center.x - total / 2.;
        for (value, unit, width) in groups {
            let value_y = metrics_y - value.mesh_bounds.center().y;
            let baseline = value_y + value.rows[0].glyphs[0].pos.y;
            let unit_y = baseline - unit.rows[0].glyphs[0].pos.y;
            let value_width = value.size().x;
            ui.painter().galley(Pos2::new(x, value_y), value, p.text);
            ui.painter()
                .galley(Pos2::new(x + value_width + 7., unit_y), unit, p.muted);
            x += width + 28.;
        }
        ui.interact(Rect::from_center_size(Pos2::new(center.x, metrics_y), Vec2::new(total, 30.)), ui.id().with("solve-turn-count"), egui::Sense::hover()).on_hover_text("Half-turn metric: R, R' and R2 each count as one move. TPS uses execution time, excluding penalties. A dash means the measurement is unavailable or unreliable.");
    }
    fn practice_actions(&mut self, ui: &mut egui::Ui, compact_training: bool) {
        let p = palette(ui.ctx());
        if self.model.cue_ready {
            let mut action = None;
            let can_cancel = self.model.active.is_some() || self.keyboard.holding();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.;
                for (kind, help, visible) in [
                    (0, "Start recognition cue", true),
                    (1, "End training session", compact_training),
                    (2, "Cancel attempt", can_cancel),
                ] {
                    if !visible {
                        continue;
                    }
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(30.), egui::Sense::click());
                    let color = if kind == 0 { p.accent } else { p.muted };
                    if response.hovered() || response.has_focus() {
                        ui.painter().rect_filled(rect, 3., p.surface);
                    }
                    let center = rect.center();
                    if kind == 0 {
                        ui.painter().add(egui::Shape::convex_polygon(
                            vec![
                                center + Vec2::new(-4., -6.),
                                center + Vec2::new(6., 0.),
                                center + Vec2::new(-4., 6.),
                            ],
                            color,
                            Stroke::NONE,
                        ));
                    } else if kind == 1 {
                        ui.painter().rect_stroke(
                            Rect::from_center_size(center, Vec2::splat(10.)),
                            1.,
                            Stroke::new(1.5_f32, color),
                            egui::StrokeKind::Inside,
                        );
                    } else {
                        for dy in [-5., 5.] {
                            ui.painter().line_segment(
                                [center + Vec2::new(-5., dy), center + Vec2::new(5., -dy)],
                                Stroke::new(1.5_f32, color),
                            );
                        }
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, help)
                    });
                    if response.clicked()
                        || response.has_focus()
                            && ui.input(|i| {
                                i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)
                            })
                    {
                        action = Some(kind);
                    }
                    response
                        .on_hover_text(help)
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            });
            match action {
                Some(0) => self.model.start_cue(),
                Some(1) => {
                    self.model.interrupt("user-stopped");
                    self.training = None;
                    self.page = "Training".into();
                }
                Some(2) => {
                    self.model.interrupt("user-stopped");
                    self.keyboard.cancel();
                }
                _ => {}
            }
            return;
        }
        if !self.keyboard.running()
            && !self.model.cue_ready
            && let Some(last) = self.model.latest.clone()
        {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = if ui.available_width() < 160. {
                    11.
                } else {
                    18.
                };
                ui.spacing_mut().button_padding = Vec2::new(0., 4.);
                ui.spacing_mut().interact_size = Vec2::new(0., 28.);
                let button = |ui: &mut egui::Ui, text: &str, accent: bool| {
                    ui.add(
                        egui::Button::new(RichText::new(text).font(font(12.)).color(if accent {
                            p.accent
                        } else {
                            p.muted
                        }))
                        .frame(false),
                    )
                };
                if button(ui, "replay", true).clicked() {
                    self.open_review(last.clone());
                }
                for penalty in ["+2", "DNF"] {
                    if button(ui, penalty, value_str(&last, "penalty") == penalty).clicked() {
                        let mut updated = last.clone();
                        updated["penalty"] = if value_str(&last, "penalty") == penalty {
                            "".into()
                        } else {
                            penalty.into()
                        };
                        self.model.save(updated);
                    }
                }
                let (more, _) = egui::containers::menu::MenuButton::from_button(
                    egui::Button::new("")
                        .min_size(Vec2::splat(28.))
                        .frame(false),
                )
                .ui(ui, |ui| {
                    if ui.button("Add note").clicked() {
                        self.open_review(last.clone());
                        ui.close();
                    }
                    if let Some(id) = last["caseId"].as_str()
                        && ui.button("Repeat case").clicked()
                    {
                        self.model.prepare_case(id, self.variant);
                        ui.close();
                    }
                    if compact_training && ui.button("End training session").clicked() {
                        self.model.interrupt("user-stopped");
                        self.training = None;
                        self.page = "Training".into();
                        ui.close();
                    }
                    if self.model.active.is_some() && ui.button("Cancel attempt").clicked() {
                        self.model.interrupt("user-stopped");
                        ui.close();
                    }
                    if ui.button("Enter a time manually").clicked() {
                        self.manual = true;
                        ui.close();
                    }
                });
                if more.hovered() || more.has_focus() {
                    ui.painter().rect_filled(more.rect, 3., p.surface);
                }
                for dx in [-4., 0., 4.] {
                    ui.painter()
                        .circle_filled(more.rect.center() + Vec2::new(dx, 0.), 1., p.muted);
                }
                more.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        "Notes and more actions",
                    )
                });
                more.on_hover_text("Notes and more actions");
            });
            return;
        }
        let mut labels = if self.keyboard.running() {
            vec!["stop timer"]
        } else if self.model.cue_ready {
            vec!["start recognition cue"]
        } else if let Some(last) = &self.model.latest {
            let mut v = vec!["replay", "+2", "DNF", "note"];
            if last["caseId"].is_string() {
                v.push("repeat case");
            }
            v
        } else if !self.model.connected {
            let mut v = vec![if !self.model.powered {
                "enable Bluetooth"
            } else if self.model.connecting {
                "connecting…"
            } else {
                "connect cube"
            }];
            if self.model.target.is_none() {
                v.push("new scramble");
            }
            v
        } else if self.model.target.is_none() && self.model.active.is_none() {
            vec!["new scramble", "use current state"]
        } else {
            vec![]
        };
        if compact_training {
            labels.push("end session");
        }
        if self.model.active.is_some() || self.keyboard.running() || self.keyboard.holding() {
            labels.push("cancel");
        }
        let width = labels
            .iter()
            .map(|s| {
                ui.painter()
                    .layout_no_wrap((*s).into(), font(12.), p.text)
                    .size()
                    .x
            })
            .sum::<f32>()
            + labels.len().saturating_sub(1) as f32 * 16.;
        centered_row(ui, width, |ui| {
            ui.spacing_mut().item_spacing.x = 16.;
            ui.spacing_mut().button_padding = Vec2::new(0., 4.);
            ui.spacing_mut().interact_size = Vec2::new(0., 26.);
            let button = |ui: &mut egui::Ui, text: &str, accent: bool| {
                ui.add(
                    egui::Button::new(RichText::new(text).font(font(12.)).color(if accent {
                        p.accent
                    } else {
                        p.faint
                    }))
                    .frame(false),
                )
            };
            if self.keyboard.running() {
                if button(ui, "stop timer", true).clicked() {
                    if let Some(ms) = self.keyboard.elapsed() {
                        self.model.manual(ms / 1000.);
                    }
                    self.keyboard.cancel();
                }
            } else if self.model.cue_ready {
                if button(ui, "start recognition cue", true).clicked() {
                    self.model.start_cue();
                }
            } else if let Some(last) = self.model.latest.clone() {
                if button(ui, "replay", true).clicked() {
                    self.open_review(last.clone());
                }
                for penalty in ["+2", "DNF"] {
                    if button(ui, penalty, value_str(&last, "penalty") == penalty).clicked() {
                        let mut updated = last.clone();
                        updated["penalty"] = if value_str(&last, "penalty") == penalty {
                            "".into()
                        } else {
                            penalty.into()
                        };
                        self.model.save(updated);
                    }
                }
                if button(ui, "note", false).clicked() {
                    self.open_review(last.clone());
                }
                if let Some(id) = last["caseId"].as_str() {
                    if button(ui, "repeat case", false).clicked() {
                        self.model.prepare_case(id, self.variant);
                    }
                }
            } else if !self.model.connected {
                if !self.model.powered {
                    if button(ui, "enable Bluetooth", true).clicked() {
                        self.model.command(Command::PowerOn);
                    }
                } else if button(
                    ui,
                    if self.model.connecting {
                        "connecting…"
                    } else {
                        "connect cube"
                    },
                    true,
                )
                .clicked()
                {
                    self.connect_dialog();
                }
                if self.model.target.is_none() && button(ui, "new scramble", false).clicked() {
                    self.model.new_scramble();
                }
            } else if self.model.target.is_none() && self.model.active.is_none() {
                if button(ui, "new scramble", true).clicked() {
                    self.model.new_scramble();
                }
                if button(ui, "use current state", false).clicked() {
                    self.model.arm_current();
                }
            }
            if compact_training && button(ui, "end session", false).clicked() {
                self.model.interrupt("user-stopped");
                self.training = None;
                self.page = "Training".into();
            }
            if self.model.active.is_some() || self.keyboard.running() || self.keyboard.holding() {
                if button(ui, "cancel", false).clicked() {
                    self.model.interrupt("user-stopped");
                    self.keyboard.cancel();
                }
            }
        });
    }
}

fn paint_moves(
    ui: &egui::Ui,
    tokens: &[guidance::Token],
    base: Pos2,
    width: f32,
    size: f32,
    suffix_width: f32,
) -> (f32, Pos2) {
    let p = palette(ui.ctx());
    let gap = if size > 20. { 6. } else { 4. };
    let padding = 5.;
    let mut rows: Vec<Vec<(guidance::Token, std::sync::Arc<egui::Galley>, Color32)>> = vec![vec![]];
    let mut row_width = 0.;
    for (index, &token) in tokens.iter().enumerate() {
        let correction = matches!(token.origin, guidance::Origin::Correction(_));
        let color = if token.done {
            p.muted.gamma_multiply(0.22)
        } else if correction {
            p.error
        } else if token.current {
            p.accent
        } else {
            p.text.gamma_multiply(0.8)
        };
        let galley = ui
            .painter()
            .layout_no_wrap(token.mv.to_string(), mono(size), color);
        let token_width = galley.size().x + padding * 2.;
        let reserved = if index + 1 == tokens.len() {
            suffix_width
        } else {
            0.
        };
        if row_width + token_width + reserved > width && !rows.last().unwrap().is_empty() {
            rows.push(vec![]);
            row_width = 0.;
        }
        row_width += token_width + gap;
        rows.last_mut().unwrap().push((token, galley, color));
    }
    let mut y = base.y;
    let mut edge = base;
    for row in rows {
        let mut x = base.x - width / 2.;
        for (token, galley, color) in row {
            let correction = matches!(token.origin, guidance::Origin::Correction(_));
            let width = galley.size().x + padding * 2.;
            let rect = Rect::from_min_size(Pos2::new(x, y - 3.), Vec2::new(width, size * 1.55));
            if token.current {
                ui.painter()
                    .rect_filled(rect, 4, color.gamma_multiply(0.10));
            }
            if token.done {
                for offset in [
                    Vec2::new(-0.6, 0.),
                    Vec2::new(0.6, 0.),
                    Vec2::new(0., -0.6),
                    Vec2::new(0., 0.6),
                ] {
                    ui.painter()
                        .galley(Pos2::new(x + padding, y) + offset, galley.clone(), color);
                }
            } else {
                ui.painter()
                    .galley(Pos2::new(x + padding, y), galley, color);
            }
            if token.current {
                let left = x + 2.;
                let right = x + width - 2.;
                let underline_y = rect.bottom();
                ui.painter().line_segment(
                    [Pos2::new(left, underline_y), Pos2::new(right, underline_y)],
                    Stroke::new(1.0_f32, p.line),
                );
                ui.painter().line_segment(
                    [
                        Pos2::new(left, underline_y),
                        Pos2::new(
                            if token.partial {
                                (left + right) / 2.
                            } else {
                                right
                            },
                            underline_y,
                        ),
                    ],
                    Stroke::new(
                        if token.current { 1.5_f32 } else { 1.0_f32 },
                        if correction {
                            p.error
                        } else if token.current || token.partial {
                            p.accent
                        } else {
                            p.accent.gamma_multiply(0.4)
                        },
                    ),
                );
            }
            if matches!(token.origin, guidance::Origin::Correction(0)) {
                // Small undo arrow identifies inserted repairs without another caption.
                let c = Pos2::new(rect.center().x, y - 9.);
                ui.painter().add(egui::Shape::line(
                    vec![
                        c + Vec2::new(4., 2.),
                        c + Vec2::new(4., -1.),
                        c + Vec2::new(1., -3.),
                        c + Vec2::new(-3., -3.),
                        c + Vec2::new(-5., 0.),
                    ],
                    Stroke::new(1.1_f32, p.error),
                ));
                ui.painter().add(egui::Shape::line(
                    vec![
                        c + Vec2::new(-5., -3.),
                        c + Vec2::new(-5., 0.),
                        c + Vec2::new(-2., 0.),
                    ],
                    Stroke::new(1.1_f32, p.error),
                ));
            }
            let help = if correction {
                format!("Undo the misscramble: {}", token.mv)
            } else if token.partial {
                format!("{}: one quarter turn remains", token.mv)
            } else if token.done {
                format!("{} completed", token.mv)
            } else if token.allowed && !token.current {
                format!("{} can be turned next; opposite faces commute", token.mv)
            } else {
                token.mv.to_string()
            };
            ui.interact(
                rect,
                ui.id().with((
                    "scramble-token",
                    match token.origin {
                        guidance::Origin::Scramble(i) => (0, i),
                        guidance::Origin::Correction(i) => (1, i),
                    },
                )),
                egui::Sense::hover(),
            )
            .on_hover_text(help);
            x += width + gap;
        }
        edge = Pos2::new(x, y);
        y += size * 1.9;
    }
    (y, edge)
}

#[cfg(test)]
mod tests {
    use super::average_metrics;
    use serde_json::{Value, json};

    fn records() -> Vec<Value> {
        (1..=5)
            .map(|n| {
                json!({
                    "source":"bluetooth", "status":"complete", "penalty":"", "duration": n * 1000,
                    "quality":[], "moves":[
                        {"move":"R", "t":0, "raw":{"cubeTimestamp":1000}},
                        {"move":"U", "t":n * 1000, "raw":{"cubeTimestamp":1000 + n * 1000}}
                    ]
                })
            })
            .collect()
    }

    #[test]
    fn average_time_certainty_uses_only_retained_endpoints() {
        let mut records = records();
        records[0]["quality"] = json!(["estimated-timing"]);
        records[0]["moves"][1]["recovered"] = true.into();
        let trimmed = average_metrics(&records, 5);
        assert_eq!(trimmed.time_label(), "3.00");
        assert!(!trimmed.time_estimated);
        records[2]["quality"] = json!(["estimated-timing"]);
        records[2]["moves"][1]["recovered"] = true.into();
        let retained = average_metrics(&records, 5);
        assert!(retained.time_estimated);
        assert_eq!(retained.time_label(), "≈ 3.00");
        assert_eq!(retained.move_samples, 3);
    }

    #[test]
    fn recovered_interior_preserves_precise_average_time() {
        let mut records = records();
        records[2]["quality"] = json!(["estimated-timing"]);
        records[2]["moves"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!({"move":"F", "t":50, "recovered":true}));
        let average = average_metrics(&records, 5);
        assert!(!average.time_estimated);
        assert!(!average.tps_estimated);
        assert_eq!(average.time_label(), "3.00");
    }
}
