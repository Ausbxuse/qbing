//! Measured solve costs and case-level F2L review. No inferred recognition times.
use crate::{
    design::{font, medium, mono, palette},
    io::{self, number, value_str},
    recording,
    ui::App,
};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use serde_json::{Value, json};

pub(crate) struct SegmentMetrics {
    pub start: usize,
    pub duration: f64,
    pub gap: f64,
    pub execution: f64,
    pub moves: usize,
    pub tps: Option<f64>,
    pub estimated: bool,
}
pub(crate) fn segment_metrics(r: &Value, start: usize, end: usize) -> Option<SegmentMetrics> {
    let events = r["moves"].as_array()?;
    if end <= start || end > events.len() {
        return None;
    }
    let before = if start == 0 {
        0.
    } else {
        events[start - 1]["t"].as_f64()?
    };
    let first = events[start]["t"].as_f64()?;
    let last = events[end - 1]["t"].as_f64()?;
    if ![before, first, last].iter().all(|n| n.is_finite())
        || before < 0.
        || first < before
        || last < first
    {
        return None;
    }
    let duration = last - before;
    let moves = recording::move_weights(events)[start..end]
        .iter()
        .sum::<usize>();
    Some(SegmentMetrics {
        start,
        duration,
        gap: first - before,
        execution: last - first,
        moves,
        tps: recording::execution_metrics(r)
            .filter(|_| duration > 0.)
            .map(|_| moves as f64 * 1000. / duration),
        estimated: [start.saturating_sub(1), start, end - 1]
            .into_iter()
            .any(|i| recording::turn_timing_estimated(r, i)),
    })
}
fn text(ui: &mut egui::Ui, s: impl Into<String>) {
    ui.label(
        RichText::new(s)
            .font(font(12.))
            .color(palette(ui.ctx()).muted),
    );
}
pub(crate) fn pair_color(light: bool, i: usize) -> Color32 {
    let c = if light {
        [0x4c87f2, 0x27a4b5, 0x7e82e8, 0xa171d4]
    } else {
        [0x79a6ff, 0x61c9d0, 0xa4a8ff, 0xc29aec]
    }[i % 4];
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}
fn panel() -> egui::Frame {
    egui::Frame::new().inner_margin(10).corner_radius(5)
}
pub(crate) fn draw(app: &mut App, ui: &mut egui::Ui, r: &Value, close: &mut bool) {
    let p = palette(ui.ctx());
    ui.spacing_mut().interact_size.y = 24.;
    panel()
        .fill(p.surface)
        .stroke(Stroke::new(1., p.line))
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - 2.).max(0.));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Scramble").font(medium(12.)));
                if ui.small_button("Copy scramble").clicked() {
                    ui.ctx().copy_text(value_str(r, "scramble").into());
                }
                if let Some(speed) = r["scrambleStats"]["tps"]
                    .as_f64()
                    .filter(|v| v.is_finite() && *v > 0.)
                {
                    text(ui, format!("{speed:.2} tps"));
                }
            });
            let scramble = value_str(r, "scramble");
            ui.label(
                RichText::new(if scramble.is_empty() {
                    "No scramble saved."
                } else {
                    scramble
                })
                .font(mono(14.)),
            );
        });
    ui.add_space(8.);
    let splits = r["analysis"]["splits"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let largest = splits
        .iter()
        .filter(|s| s["time"].as_f64().is_some_and(|v| v.is_finite() && v > 0.))
        .max_by(|a, b| number(a, "time").total_cmp(&number(b, "time")));
    let gap = r["analysis"]["pauses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| number(p, "duration") > 0.)
        .max_by(|a, b| number(a, "duration").total_cmp(&number(b, "duration")));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.;
        if let Some(s) = largest {
            text(
                ui,
                format!(
                    "Largest phase · {} · {:.0}%",
                    value_str(s, "stage"),
                    number(s, "time") / number(r, "duration").max(1.) * 100.
                ),
            );
        }
        if let Some(g) = gap {
            ui.label(
                RichText::new(format!(
                    "Longest gap · {}{} s",
                    if g["estimated"] == true { "≈" } else { "" },
                    io::seconds(number(g, "duration"))
                ))
                .font(font(12.))
                .color(p.accent),
            );
            if ui.small_button("Replay gap").clicked() {
                app.seek_review_turn(number(g, "index") as usize);
                app.toggle_review_playback();
            }
        }
    });
    let data = if r["moves"].as_array().is_some_and(|m| !m.is_empty()) {
        crate::native_ui::cached_tool(
            app,
            ui,
            app.review_solver_key.clone(),
            "native-review",
            json!({"record":r}),
        )
    } else {
        None
    };
    ui.add_space(10.);
    ui.horizontal(|ui| {
        ui.label(RichText::new("F2L pairs").font(medium(14.)));
        if data.is_none() && r["moves"].as_array().is_some_and(|m| !m.is_empty()) {
            ui.spinner();
        }
    });
    let own = crate::notation::reconstruct_for_view(r);
    let groups = data
        .as_ref()
        .and_then(|d| d.as_ref().ok())
        .and_then(|d| d["groups"].as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let pairs = groups
        .iter()
        .filter(|g| g["stage"] == "F2L" && number(g, "rawEnd") > number(g, "rawStart"))
        .collect::<Vec<_>>();
    if pairs.is_empty() && data.is_some() {
        text(ui, "No verified pair breakdown for this solve.");
    }
    let pair_total = pairs
        .iter()
        .filter_map(|g| {
            segment_metrics(
                r,
                number(g, "rawStart") as usize,
                number(g, "rawEnd") as usize,
            )
        })
        .map(|m| m.duration)
        .sum::<f64>();
    if pair_total > 0. {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            let available = ui.available_width() - (pairs.len().saturating_sub(1) as f32 * 4.);
            for (i, g) in pairs.iter().enumerate() {
                if let Some(m) = segment_metrics(
                    r,
                    number(g, "rawStart") as usize,
                    number(g, "rawEnd") as usize,
                ) {
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new((available * (m.duration / pair_total) as f32).max(1.), 14.),
                        egui::Sense::click(),
                    );
                    ui.painter()
                        .rect_filled(rect, 2, pair_color(p.bg.r() > 128, i));
                    if response
                        .on_hover_text(format!(
                            "{} · {} s",
                            value_str(g, "label"),
                            io::seconds(m.duration)
                        ))
                        .clicked()
                    {
                        app.seek_review_turn(m.start);
                    }
                }
            }
        });
    }
    let columns = if ui.available_width() >= 600. { 2 } else { 1 };
    for (row, chunk) in pairs.chunks(columns).enumerate() {
        ui.columns(columns, |uis| {
            for (column, g) in chunk.iter().enumerate() {
                pair_card(
                    app,
                    &mut uis[column],
                    r,
                    g,
                    row * columns + column,
                    &own,
                    close,
                );
            }
        });
    }
    for stage in ["Cross", "OLL", "PLL"] {
        for g in groups
            .iter()
            .filter(|g| g["stage"] == stage && number(g, "rawEnd") > number(g, "rawStart"))
        {
            ui.add_space(8.);
            egui::CollapsingHeader::new(
                RichText::new(if stage == "Cross" {
                    "Cross solutions".into()
                } else {
                    format!("{} · {}", stage, value_str(g, "caseName"))
                })
                .font(medium(12.))
                .color(crate::review_ui::phase_color(ui, stage)),
            )
            .id_salt(("analysis-layer", stage))
            .show(ui, |ui| {
                for v in g["variants"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|v| v["verified"] == true)
                {
                    text(
                        ui,
                        format!("{} · {} moves", value_str(v, "name"), number(v, "moves")),
                    );
                    ui.label(RichText::new(value_str(v, "viewAlgorithm")).font(mono(14.)));
                    ui.horizontal(|ui| {
                        if ui.link("Copy").clicked() {
                            ui.ctx().copy_text(value_str(v, "viewAlgorithm").into());
                        }
                        if !value_str(v, "caseId").is_empty() && ui.link("Practice").clicked() {
                            app.model.prepare_case(
                                value_str(v, "caseId"),
                                v["variantIndex"].as_u64().unwrap_or(0) as usize,
                            );
                            app.page = "Practice".into();
                            *close = true;
                        }
                    });
                }
            });
        }
    }
    egui::CollapsingHeader::new(RichText::new("Timing details").font(font(12.)))
        .id_salt("analysis-timing")
        .show(ui, |ui| {
            if r["rotations"].as_array().is_some_and(|v| !v.is_empty())
                && crate::render::orientation::initial_live_pose(r).is_none()
            {
                text(
                    ui,
                    "Starting calibration was not recorded; gyro poses are relative.",
                );
            }
            for key in ["quality", "scrambleQuality"] {
                if let Some(f) = r[key].as_array().filter(|f| !f.is_empty()) {
                    text(
                        ui,
                        f.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" · "),
                    );
                }
            }
            for s in r["analysis"]["signals"]
                .as_array()
                .into_iter()
                .flatten()
                .take(5)
            {
                text(
                    ui,
                    format!("Possible correction · {}", value_str(s, "text")),
                );
            }
            text(
                ui,
                "Colored time is the entry gap. ≈ marks approximate event timing.",
            );
        });
}

fn pair_card(
    app: &mut App,
    ui: &mut egui::Ui,
    r: &Value,
    g: &Value,
    i: usize,
    own: &[crate::notation::NotationTurn],
    close: &mut bool,
) {
    let p = palette(ui.ctx());
    let Some(m) = segment_metrics(
        r,
        number(g, "rawStart") as usize,
        number(g, "rawEnd") as usize,
    ) else {
        return;
    };
    let color = pair_color(p.bg.r() > 128, i);
    ui.add_space(6.);
    panel()
        .fill(color.gamma_multiply(0.055))
        .stroke(Stroke::new(1., color.gamma_multiply(0.24)))
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - 2.).max(0.));
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(value_str(g, "label"))
                        .font(medium(14.))
                        .color(color),
                );
                text(ui, value_str(g, "caseName"));
                if ui.small_button("Replay").clicked() {
                    app.seek_review_turn(m.start);
                    app.toggle_review_playback();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 16.;
                let width =
                    crate::phase_time::width(ui, Some(m.duration), m.gap, false, m.estimated, 16.);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 34.), egui::Sense::hover());
                crate::phase_time::paint(
                    ui,
                    rect.right_top(),
                    Some(m.duration),
                    m.gap,
                    false,
                    m.estimated,
                    16.,
                );
                text(ui, format!("{} moves", m.moves));
                text(
                    ui,
                    m.tps
                        .map(|v| format!("{}{v:.2} tps", if m.estimated { "≈" } else { "" }))
                        .unwrap_or("— tps".into()),
                );
            });
            let algorithm = own
                .iter()
                .filter(|t| {
                    t.raw_start >= number(g, "rawStart") as usize
                        && t.raw_end <= number(g, "rawEnd") as usize
                })
                .map(|t| t.move_text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            ui.label(RichText::new(algorithm).font(mono(14.)).color(p.muted));
            egui::CollapsingHeader::new(RichText::new("Alternatives").font(medium(12.)))
                .id_salt(("analysis-alternatives", i))
                .show(ui, |ui| {
                    for v in g["variants"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|v| v["verified"] == true)
                    {
                        ui.horizontal_wrapped(|ui| {
                            text(
                                ui,
                                format!("{} · {} moves", value_str(v, "name"), number(v, "moves")),
                            );
                            if ui.link("Copy").clicked() {
                                ui.ctx().copy_text(value_str(v, "viewAlgorithm").into());
                            }
                            if !value_str(v, "caseId").is_empty() && ui.link("Practice").clicked() {
                                app.model.prepare_case(
                                    value_str(v, "caseId"),
                                    v["variantIndex"].as_u64().unwrap_or(0) as usize,
                                );
                                app.page = "Practice".into();
                                *close = true;
                            }
                        });
                        ui.label(RichText::new(value_str(v, "viewAlgorithm")).font(mono(14.)));
                    }
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pair_windows_keep_half_turns_and_gaps_separate() {
        let r = json!({"source":"bluetooth","status":"complete","duration":200.,"quality":[],"moves":[{"move":"R","t":0.,"raw":{"cubeTimestamp":1000.}},{"move":"R","t":30.,"raw":{"cubeTimestamp":1030.}},{"move":"U","t":100.,"raw":{"cubeTimestamp":1100.}},{"move":"F","t":200.,"raw":{"cubeTimestamp":1200.}}]});
        let a = segment_metrics(&r, 0, 2).unwrap();
        assert_eq!(a.moves, 1);
        assert_eq!(a.duration, 30.);
        let b = segment_metrics(&r, 2, 4).unwrap();
        assert_eq!(b.gap, 70.);
        assert_eq!(b.execution, 100.);
        assert_eq!(b.moves, 2);
        let mut recovered = r.clone();
        recovered["moves"][1]["recovered"] = json!(true);
        recovered["quality"] = json!(["estimated-timing"]);
        assert!(!segment_metrics(&recovered, 0, 4).unwrap().estimated);
        assert!(segment_metrics(&recovered, 2, 4).unwrap().estimated);
        assert!(segment_metrics(&r, 3, 8).is_none());
    }
    #[test]
    fn synthetic_analysis_has_four_verified_pair_boundaries() {
        let r: Value = serde_json::from_str(include_str!(
            "../../androidApp/src/debug/assets/analysis-fixture.json"
        ))
        .unwrap();
        let r = recording::for_review(r);
        let data = crate::solver::request("native-review", Some(&json!({"record":r}))).unwrap();
        let pairs = data["groups"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|g| g["stage"] == "F2L" && number(g, "rawEnd") > number(g, "rawStart"))
            .collect::<Vec<_>>();
        assert_eq!(pairs.len(), 4);
        assert_eq!(
            pairs
                .iter()
                .map(|g| number(g, "rawEnd") as usize)
                .collect::<Vec<_>>(),
            vec![8, 11, 14, 17]
        );
        for g in pairs {
            let m = segment_metrics(
                &r,
                number(g, "rawStart") as usize,
                number(g, "rawEnd") as usize,
            )
            .unwrap();
            assert!(m.duration > 0.);
            assert!(m.tps.is_some());
        }
        assert!(recording::execution_metrics(&r).unwrap().tps >= 10.);
    }
}
