use super::{cross_description, label, rate};
use crate::{
    design::{medium, palette},
    io::{seconds, value_str},
    ui::App,
};
use eframe::egui::{self, Align2, Rect, Stroke, Vec2};
use serde_json::Value;

const NAMES: [&str; 4] = ["Cross", "F2L", "OLL", "PLL"];

struct PhaseRow {
    name: &'static str,
    time: Option<f64>,
    estimated: bool,
    tps: String,
    moves: Option<u64>,
    complete: bool,
    live: bool,
}
impl PhaseRow {
    fn measured(&self) -> bool {
        self.complete && self.time.is_some_and(|t| t > 0.) && self.moves.is_some_and(|n| n > 0)
    }
    fn time_text(&self) -> String {
        self.time
            .map(|time| {
                format!(
                    "{}{}",
                    if self.estimated && time > 0. {
                        "≈ "
                    } else {
                        ""
                    },
                    seconds(time)
                )
            })
            .unwrap_or_else(|| "—".into())
    }
    fn moves_text(&self) -> String {
        self.moves
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".into())
    }
    fn tooltip(&self, duration: f64, reviewable: bool) -> String {
        let Some(time) = self.time else {
            return format!("{} · waiting for this phase", self.name);
        };
        let share = if duration > 0. {
            format!(" · {:.0}% of solve", time / duration * 100.)
        } else {
            String::new()
        };
        format!(
            "{} · {} s · {} TPS · {} moves{}{}",
            self.name,
            self.time_text(),
            self.tps,
            self.moves_text(),
            share,
            if reviewable {
                "\nReview turns and pauses · Enter or Space"
            } else {
                ""
            }
        )
    }
}
fn rows(analysis: &Value) -> [PhaseRow; 4] {
    std::array::from_fn(|i| {
        let split = analysis["splits"]
            .as_array()
            .and_then(|splits| splits.iter().find(|s| value_str(s, "stage") == NAMES[i]));
        let time = split
            .and_then(|s| s["time"].as_f64())
            .filter(|n| n.is_finite() && *n >= 0.);
        PhaseRow {
            name: NAMES[i],
            time,
            estimated: split.is_some_and(|s| s["estimated"] == true),
            tps: split.map(rate).unwrap_or_else(|| "—".into()),
            moves: split
                .and_then(|s| s["moves"].as_u64())
                .filter(|_| time.is_some()),
            complete: split.is_some_and(|s| s["end"].as_f64().is_some_and(f64::is_finite)),
            live: split.is_some_and(|s| s["live"].as_bool().unwrap_or(false)),
        }
    })
}
fn duration(analysis: &Value, rows: &[PhaseRow; 4]) -> f64 {
    analysis["duration"]
        .as_f64()
        .filter(|n| n.is_finite() && *n > 0.)
        .unwrap_or(0.)
        .max(rows.iter().filter_map(|r| r.time).sum())
}
fn activated(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.clicked()
        || response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space))
}

impl App {
    fn phase_review_record(&self) -> Option<Value> {
        if self.is_solving()
            || self.keyboard.holding()
            || self.model.active.is_some()
            || self.model.countdown.is_some()
            || self.model.cue_ready
        {
            return None;
        }
        self.model
            .latest
            .as_ref()
            .filter(|r| {
                crate::recording::execution_metrics(r).is_some()
                    && value_str(r, "source") != "manual"
                    && r["moves"].as_array().is_some_and(|moves| !moves.is_empty())
                    && r["analysis"]["splits"].is_array()
            })
            .cloned()
    }

    pub(super) fn phase_strip(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = palette(ui.ctx());
        let analysis = self.practice_analysis();
        if self.isolated {
            ui.ctx().data_mut(|data| {
                data.insert_temp(egui::Id::new("phase-metrics"), analysis.clone())
            });
        }
        let rows = rows(&analysis);
        let duration = duration(&analysis, &rows);
        let record = self.phase_review_record();
        let compact = rect.width() < 620.;
        let short = rect.height() <= 84.;
        let gap = if compact { 12. } else { 28. };
        let width = (rect.width() - gap * 3.) / 4.;
        let mut selected = None;
        for (i, row) in rows.iter().enumerate() {
            let cell = Rect::from_min_size(
                rect.min + Vec2::new(i as f32 * (width + gap), 0.),
                Vec2::new(width, rect.height()),
            );
            let reviewable = record.is_some() && row.measured();
            let response = ui.interact(
                cell,
                ui.id().with(("phase-result", row.name)),
                if reviewable {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                },
            );
            let hover = reviewable && (response.hovered() || response.has_focus());
            let name_color = if row.live || hover { p.accent } else { p.muted };
            let value_color = if row.live {
                p.accent
            } else if row.time.is_some() {
                p.text
            } else {
                p.faint
            };
            label(
                ui,
                cell.min + Vec2::new(0., 9.),
                row.name,
                12.,
                name_color,
                Align2::LEFT_CENTER,
            );
            if row.live || hover {
                ui.painter().line_segment(
                    [
                        cell.min + Vec2::new(0., 21.),
                        cell.min + Vec2::new(width.min(40.), 21.),
                    ],
                    Stroke::new(1.5_f32, p.accent),
                );
            }
            ui.painter().text(
                cell.min + Vec2::new(0., if short { 33. } else { 40. }),
                Align2::LEFT_CENTER,
                row.time_text(),
                medium(if compact { 24. } else { 28. }),
                value_color,
            );
            if compact {
                label(
                    ui,
                    cell.min + Vec2::new(0., if short { 49. } else { 66. }),
                    format!("{} tps", row.tps),
                    11.,
                    p.muted,
                    Align2::LEFT_CENTER,
                );
                label(
                    ui,
                    cell.min + Vec2::new(0., if short { 64. } else { 83. }),
                    format!("{} moves", row.moves_text()),
                    11.,
                    p.muted,
                    Align2::LEFT_CENTER,
                );
            } else {
                label(
                    ui,
                    cell.min + Vec2::new(0., 72.),
                    format!("{} tps · {} moves", row.tps, row.moves_text()),
                    12.,
                    p.muted,
                    Align2::LEFT_CENTER,
                );
            }
            if reviewable {
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!(
                            "Review {}: {} seconds, {} TPS, {} moves",
                            row.name,
                            row.time_text(),
                            row.tps,
                            row.moves_text()
                        ),
                    )
                });
                if activated(ui, &response) {
                    selected = Some(row.name);
                }
            }
            let response = if reviewable {
                response.on_hover_cursor(egui::CursorIcon::PointingHand)
            } else {
                response
            };
            let mut tooltip = row.tooltip(duration, reviewable);
            if i == 0 {
                tooltip.push_str(&format!("\n{}", cross_description(&analysis)));
            }
            response.on_hover_text(tooltip);
        }
        if let (Some(record), Some(phase)) = (record, selected) {
            self.open_phase_review(record, phase);
        }
    }
}
