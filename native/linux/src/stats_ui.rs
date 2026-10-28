use crate::{
    design::{self, display, font, medium, palette},
    io::{adjusted, number, seconds, timed, value_str},
    ui::App,
};
use eframe::egui::{self, Align2, Color32, Margin, Pos2, Rect, RichText, Stroke, Vec2};
use serde_json::Value;

struct Phase {
    name: &'static str,
    time: f64,
    pause: f64,
    rate: Option<f64>,
    count: usize,
}

impl App {
    pub(crate) fn insights_view(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        if self.model.records.is_empty() {
            self.stats_empty(
                ui,
                "A baseline starts with one solve",
                "Record a solve to see your pace, phase times, and where to focus next.",
                false,
            );
            return;
        }
        let solves = self
            .model
            .records
            .iter()
            .filter(|r| value_str(r, "kind") == "solve")
            .cloned()
            .collect::<Vec<_>>();
        let drills = self
            .model
            .records
            .iter()
            .filter(|r| value_str(r, "kind") == "drill")
            .cloned()
            .collect::<Vec<_>>();
        let mode_id = egui::Id::new("insights-kind");
        let mut mode = ui
            .ctx()
            .data(|d| d.get_temp::<String>(mode_id))
            .unwrap_or_else(|| {
                if solves.is_empty() {
                    "Drills"
                } else {
                    "Solves"
                }
                .into()
            });
        ui.horizontal_wrapped(|ui| {
            for label in ["Solves", "Drills"] {
                ui.selectable_value(&mut mode, label.into(), label);
            }
            ui.add_space(8.);
            ui.label(RichText::new("All sessions").font(font(12.)).color(p.muted));
        });
        ui.ctx().data_mut(|d| d.insert_temp(mode_id, mode.clone()));
        ui.add_space(20.);
        if mode == "Drills" {
            self.drill_insights(ui, &drills);
            return;
        }
        if solves.is_empty() {
            self.stats_empty(ui, "Your first full solve belongs here", "Case drills have their own view. Record a full solve to start tracking your overall pace.", false);
            return;
        }
        let clean = solves.iter().filter(|r| timed(r)).collect::<Vec<_>>();
        let best = clean.iter().map(|r| adjusted(r)).min_by(f64::total_cmp);
        summary_strip(
            ui,
            &[
                (
                    "Solves",
                    solves.len().to_string(),
                    "all recorded attempts".into(),
                ),
                (
                    "Best",
                    best.map(seconds).unwrap_or("—".into()),
                    "clean completed solve".into(),
                ),
                (
                    "Average of 5",
                    crate::practice_ui::average_metrics(&solves, 5).time_label(),
                    average_detail(&solves, 5),
                ),
                (
                    "Average of 12",
                    crate::practice_ui::average_metrics(&solves, 12).time_label(),
                    average_detail(&solves, 12),
                ),
            ],
        );
        ui.add_space(20.);
        let recent = solves
            .iter()
            .rev()
            .take(60)
            .rev()
            .cloned()
            .collect::<Vec<_>>();
        panel(ui, |ui| {
            section_line(
                ui,
                "Your pace",
                &format!("Last {} solves · seconds", recent.len()),
            );
            ui.add_space(10.);
            trend(ui, &recent, 175.);
        });
        let phases = phases(&clean);
        ui.add_space(20.);
        if phases.is_empty() {
            panel(ui, |ui| {
                ui.label(
                    RichText::new("Add turn-by-turn detail")
                        .font(medium(16.))
                        .color(p.text),
                );
                ui.label(RichText::new("Connect your cube for CFOP times and TPS. Manually entered times still count toward your session averages.").font(font(13.)).color(p.muted));
                ui.add_space(6.);
                if ui.button("Go to practice").clicked() {
                    self.page = "Practice".into();
                }
            });
        } else {
            panel(ui, |ui| {
                section_line(
                    ui,
                    "Where your time goes",
                    &format!("Last {} clean solves", clean.len().min(30)),
                );
                ui.add_space(16.);
                phase_summary(ui, &phases);
            });
            ui.add_space(16.);
            if let Some(focus) = phases
                .iter()
                .max_by(|a, b| (a.time * 0.4 + a.pause).total_cmp(&(b.time * 0.4 + b.pause)))
            {
                panel(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("NEXT PRACTICE")
                                .font(medium(11.))
                                .color(p.accent),
                        );
                        ui.label(
                            RichText::new(format!("Based on {} recent solves", focus.count))
                                .font(font(11.))
                                .color(p.muted),
                        );
                    });
                    ui.add_space(8.);
                    ui.label(
                        RichText::new(format!("Make {0} more consistent", focus.name))
                            .font(medium(19.))
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} s on average, including {} s in pauses of at least 0.5 s.",
                            seconds(focus.time),
                            seconds(focus.pause)
                        ))
                        .font(font(13.))
                        .color(p.muted),
                    );
                    ui.label(RichText::new(match focus.name {
                        "Cross" => "Plan the whole cross before turning, then compare your route with the optimal solution.",
                        "F2L" => "Use slow, continuous turns. Look for the next pair before finishing the current insertion.",
                        _ => "Practice five clean repetitions. Compare recognition and execution separately.",
                    }).font(font(13.)).color(p.muted));
                    ui.add_space(10.);
                    if design::primary(ui, &format!("Practice {}", focus.name)).clicked() {
                        self.stage = focus.name.into();
                        self.page = "Training".into();
                    }
                });
            }
        }
        ui.add_space(20.);
        self.case_performance(ui);
    }

    fn drill_insights(&mut self, ui: &mut egui::Ui, drills: &[Value]) {
        if drills.is_empty() {
            self.stats_empty(ui, "Learn a case, then measure it", "Start with five clean repetitions. Recognition and execution are tracked separately for every case.", true);
            return;
        }
        let p = palette(ui.ctx());
        let clean = drills.iter().filter(|r| timed(r)).count();
        let cases = drills
            .iter()
            .map(|r| value_str(r, "caseId"))
            .collect::<std::collections::HashSet<_>>()
            .len();
        let cues = drills
            .iter()
            .filter(|r| timed(r) && r["recognitionMs"].as_f64().is_some())
            .count();
        summary_strip(
            ui,
            &[
                (
                    "Repetitions",
                    drills.len().to_string(),
                    "all recorded attempts".into(),
                ),
                ("Cases", cases.to_string(), "practiced so far".into()),
                (
                    "Clean",
                    format!("{:.0}%", clean as f64 / drills.len() as f64 * 100.),
                    format!("{clean} completed reliably"),
                ),
                (
                    "Recognition",
                    cues.to_string(),
                    "cue-to-first-turn samples".into(),
                ),
            ],
        );
        ui.add_space(20.);
        panel(ui, |ui| {
            section_line(ui, "Build consistency", "Your next session");
            ui.label(RichText::new("Revisit unfinished cases and compare your next five repetitions with your baseline.").font(font(13.)).color(p.muted));
            ui.add_space(10.);
            if design::primary(ui, "Open training").clicked() {
                self.page = "Training".into();
            }
        });
        ui.add_space(20.);
        self.case_performance(ui);
    }

    fn case_performance(&mut self, ui: &mut egui::Ui) {
        if self.model.stats.is_empty() {
            return;
        }
        let p = palette(ui.ctx());
        section_line(ui, "Case performance", "Recognition / execution");
        ui.add_space(8.);
        let mut stats = self.model.stats.clone();
        stats.sort_by(|a, b| {
            number(&b["algorithms"][0], "meanExecution")
                .total_cmp(&number(&a["algorithms"][0], "meanExecution"))
        });
        for stat in stats {
            let name = value_str(&stat, "name");
            let algorithm = stat["algorithms"].as_array().and_then(|a| {
                a.iter()
                    .max_by(|a, b| number(a, "count").total_cmp(&number(b, "count")))
            });
            let execution = algorithm
                .and_then(|a| a["meanExecution"].as_f64())
                .map(seconds)
                .unwrap_or("—".into());
            let recognition = algorithm
                .and_then(|a| a["meanRecognition"].as_f64())
                .map(seconds)
                .unwrap_or("—".into());
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 64.), egui::Sense::click());
            let painter = ui.painter_at(rect);
            hover_row(ui, &response, rect);
            let narrow = rect.width() < 520.;
            let text_right = rect.right() - if narrow { 150. } else { 255. };
            clipped_text(
                &painter,
                Rect::from_min_max(rect.min, Pos2::new(text_right, rect.bottom())),
                Pos2::new(rect.left() + 12., rect.top() + 18.),
                name,
                medium(14.),
                p.text,
            );
            painter.text(
                Pos2::new(rect.left() + 12., rect.top() + 43.),
                Align2::LEFT_CENTER,
                format!(
                    "{} · {} executions",
                    value_str(&stat, "stage"),
                    number(&stat, "count") as usize
                ),
                font(11.),
                p.muted,
            );
            painter.text(
                Pos2::new(rect.right() - 18., rect.top() + 20.),
                Align2::RIGHT_CENTER,
                format!("{recognition} / {execution} s"),
                medium(14.),
                p.text,
            );
            painter.text(
                Pos2::new(rect.right() - 18., rect.top() + 43.),
                Align2::RIGHT_CENTER,
                "most-used algorithm",
                font(11.),
                p.muted,
            );
            painter.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(1.0f32, p.line),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Review {name}"))
            });
            if activated(ui, &response) {
                self.selected = value_str(&stat, "caseId").into();
                self.variant = algorithm
                    .and_then(|a| a["variant"].as_u64())
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                self.page = "Case".into();
            }
        }
    }

    pub(crate) fn history_view(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width().min(1100.);
        let inset = (ui.available_width() - width) * 0.5;
        let height = ui.available_height();
        ui.horizontal(|ui| {
            ui.add_space(inset);
            ui.allocate_ui_with_layout(
                Vec2::new(width, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.history_table(ui),
            );
        });
    }

    fn history_table(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        let compact = ui.available_width() < 700.;
        let row_height = if compact { 40. } else { 36. };
        ui.add_space(if compact { 16. } else { 30. });
        let count = self
            .model
            .records
            .iter()
            .filter(|r| history_matches(r, &self.history_session, &self.history_kind))
            .count();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Solves").font(font(25.)).color(p.text));
            ui.add_space(6.);
            ui.label(
                RichText::new(count.to_string())
                    .font(font(13.))
                    .color(p.faint),
            );
        });
        ui.add_space(if compact { 12. } else { 18. });
        let mut sessions = self
            .model
            .records
            .iter()
            .map(|r| value_str(r, "session").to_owned())
            .collect::<Vec<_>>();
        sessions.sort();
        sessions.dedup();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().interact_size.y = if compact { 34. } else { 30. };
            ui.spacing_mut().button_padding = Vec2::new(8., 5.);
            ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
            ui.visuals_mut().widgets.hovered.bg_fill = p.surface;
            ui.visuals_mut().widgets.hovered.weak_bg_fill = p.surface;
            egui::ComboBox::from_id_salt("scorebook-session")
                .width(if compact { 145. } else { 175. })
                .selected_text(
                    RichText::new(&self.history_session)
                        .font(font(12.))
                        .color(p.muted),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.history_session,
                        "All sessions".into(),
                        "All sessions",
                    );
                    for session in sessions {
                        ui.selectable_value(&mut self.history_session, session.clone(), session);
                    }
                });
            egui::ComboBox::from_id_salt("scorebook-kind")
                .width(110.)
                .selected_text(
                    RichText::new(&self.history_kind)
                        .font(font(12.))
                        .color(p.muted),
                )
                .show_ui(ui, |ui| {
                    for kind in ["All attempts", "Solves", "Drills"] {
                        ui.selectable_value(&mut self.history_kind, kind.into(), kind);
                    }
                });
            if !compact {
                let mut recent = self
                    .model
                    .records
                    .iter()
                    .rev()
                    .filter(|r| {
                        value_str(r, "kind") == "solve"
                            && history_matches(r, &self.history_session, &self.history_kind)
                    })
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>();
                if recent.len() == 5 {
                    recent.reverse();
                    let average = crate::practice_ui::average_metrics(&recent, 5);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "ao5  {} · {} TPS · {} moves",
                                average.time_label(),
                                average.tps_label(),
                                average
                                    .moves
                                    .map(|m| format!("{m:.1}"))
                                    .unwrap_or("—".into()),
                            ))
                            .font(font(11.))
                            .color(p.faint),
                        )
                        .on_hover_text(average.sample_detail());
                    });
                }
            }
        });
        if !self.model.message.is_empty() {
            ui.add_space(4.);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(self.model.message.clone())
                        .font(font(11.))
                        .color(p.muted),
                );
                if ui.small_button("Dismiss").clicked() {
                    self.model.message.clear();
                }
            });
        }
        // Keep only indices here. Long scorebooks should not clone every raw
        // reconstruction on each frame; only visible rows need their record.
        let records = self
            .model
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| history_matches(r, &self.history_session, &self.history_kind))
            .map(|(i, _)| i)
            .rev()
            .collect::<Vec<_>>();
        ui.add_space(if compact { 12. } else { 18. });
        if records.is_empty() {
            ui.separator();
            ui.add_space(16.);
            ui.label(
                RichText::new(if self.model.records.is_empty() {
                    "No solves yet"
                } else {
                    "No attempts match these filters"
                })
                .font(font(14.))
                .color(p.muted),
            );
            if self.model.records.is_empty() {
                if ui.button("Go to timer").clicked() {
                    self.page = "Practice".into();
                }
            } else if ui.button("Clear filters").clicked() {
                self.history_session = "All sessions".into();
                self.history_kind = "All attempts".into();
            }
            return;
        }
        ui.spacing_mut().item_spacing.y = 0.;
        // Floating scroll bars keep fixed headers and the visible rows at the
        // same width. The final column leaves room for the narrow scroll thumb.
        ui.spacing_mut().scroll.floating = true;
        ui.spacing_mut().scroll.floating_allocated_width = 0.;
        let (header, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.), egui::Sense::hover());
        let columns = HistoryColumns::new(header);
        let painter = ui.painter_at(header);
        let mut headings = vec![
            (columns.index, "#", Align2::LEFT_CENTER),
            (
                columns.context,
                if columns.date.is_some() {
                    "Session / case"
                } else {
                    "Attempt"
                },
                Align2::LEFT_CENTER,
            ),
            (columns.time, "Time", Align2::RIGHT_CENTER),
            (columns.tps, "TPS", Align2::RIGHT_CENTER),
            (columns.moves, "Moves", Align2::RIGHT_CENTER),
        ];
        if let Some(date) = columns.date {
            headings.push((date, "Recorded", Align2::LEFT_CENTER));
        }
        if let Some(scramble) = columns.scramble {
            headings.push((scramble, "Scramble TPS", Align2::RIGHT_CENTER));
        }
        for (x, label, align) in headings {
            painter.text(
                Pos2::new(x, header.center().y),
                align,
                label,
                font(11.),
                p.faint,
            );
        }
        painter.line_segment(
            [
                header.left_bottom() - Vec2::new(0., 0.5),
                header.right_bottom() - Vec2::new(0., 0.5),
            ],
            Stroke::new(1.0f32, p.line),
        );
        let scroll_id = (
            "scorebook-rows",
            self.history_session.clone(),
            self.history_kind.clone(),
        );
        egui::ScrollArea::vertical()
            .id_salt(scroll_id)
            .auto_shrink([false, false])
            .min_scrolled_height(0.)
            .max_height(ui.available_height())
            .show_rows(ui, row_height, records.len(), |ui, visible| {
                for row in visible {
                    let record = self.model.records[records[row]].clone();
                    self.scorebook_row(ui, &record, records.len() - row, row_height);
                }
            });
    }

    fn scorebook_row(&mut self, ui: &mut egui::Ui, record: &Value, index: usize, height: f32) {
        let p = palette(ui.ctx());
        let case = record["caseId"]
            .as_str()
            .and_then(|id| self.model.catalog.cases.iter().find(|c| c.id == id))
            .map(|c| c.name.as_str());
        let session = value_str(record, "session");
        let date = chrono::DateTime::parse_from_rfc3339(value_str(record, "createdAt"))
            .map(|d| {
                d.with_timezone(&chrono::Local)
                    .format("%b %-d · %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        let quality = record["quality"]
            .as_array()
            .filter(|flags| !flags.is_empty());
        let interrupted = value_str(record, "status") != "complete";
        let manual = value_str(record, "source") == "manual";
        let estimated =
            crate::recording::execution_metrics(record).is_some_and(|metric| metric.estimated);
        let mut duration = format!(
            "{}{}",
            if crate::recording::execution_duration_estimated(record)
                && adjusted(record).is_finite()
            {
                "≈ "
            } else {
                ""
            },
            seconds(adjusted(record))
        );
        if value_str(record, "penalty") == "+2" && !interrupted {
            duration.push_str(" +2");
        }
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            egui::Sense::click(),
        );
        let columns = HistoryColumns::new(rect);
        let painter = ui.painter_at(rect);
        if response.hovered() || response.has_focus() {
            painter.rect_filled(rect, 0, p.surface);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                0,
                Stroke::new(1.0f32, p.accent),
                egui::StrokeKind::Inside,
            );
        }
        let y = rect.center().y;
        painter.text(
            Pos2::new(columns.index, y),
            Align2::LEFT_CENTER,
            index.to_string(),
            font(11.),
            p.faint,
        );
        if interrupted || quality.is_some() || manual {
            painter.circle_filled(
                Pos2::new(rect.left() + 38., y),
                2.,
                if interrupted || quality.is_some() {
                    p.amber
                } else {
                    p.faint
                },
            );
        }
        if let Some(x) = columns.date {
            painter.text(
                Pos2::new(x, y),
                Align2::LEFT_CENTER,
                &date,
                font(11.),
                p.muted,
            );
        }
        let label = if columns.date.is_some() {
            case.map(|case| format!("{session} · {case}"))
                .unwrap_or_else(|| session.to_owned())
        } else {
            case.unwrap_or("3 × 3 solve").to_owned()
        };
        clipped_text(
            &painter,
            Rect::from_min_max(
                Pos2::new(columns.context, rect.top()),
                Pos2::new(columns.time - 94., rect.bottom()),
            ),
            Pos2::new(columns.context, y),
            &label,
            font(12.),
            p.muted,
        );
        painter
            .with_clip_rect(Rect::from_min_max(
                Pos2::new(columns.time - 88., rect.top()),
                Pos2::new(columns.time + 1., rect.bottom()),
            ))
            .text(
                Pos2::new(columns.time, y),
                Align2::RIGHT_CENTER,
                &duration,
                font(15.),
                if interrupted || value_str(record, "penalty") == "DNF" {
                    p.amber
                } else {
                    p.text
                },
            );
        painter.text(
            Pos2::new(columns.tps, y),
            Align2::RIGHT_CENTER,
            solve_rate(record),
            font(12.),
            p.muted,
        );
        if let Some(x) = columns.scramble {
            painter.text(
                Pos2::new(x, y),
                Align2::RIGHT_CENTER,
                scramble_rate(record),
                font(12.),
                p.muted,
            );
        }
        painter.text(
            Pos2::new(columns.moves, y),
            Align2::RIGHT_CENTER,
            crate::recording::execution_moves(record)
                .map(|n| n.to_string())
                .unwrap_or("—".into()),
            font(12.),
            p.muted,
        );
        painter.line_segment(
            [
                rect.left_bottom() - Vec2::new(0., 0.5),
                rect.right_bottom() - Vec2::new(0., 0.5),
            ],
            Stroke::new(1.0f32, p.line),
        );
        let full_name = case.unwrap_or("3 × 3 solve").to_owned();
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!(
                    "Review {full_name}, {duration} seconds, {} TPS",
                    solve_rate(record)
                ),
            )
        });
        let clicked = activated(ui, &response);
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_ui(|ui| {
                ui.label(RichText::new(&full_name).font(medium(13.)));
                ui.label(format!("{date} · {session}"));
                if interrupted {
                    ui.label("Interrupted attempt · retained for review");
                }
                if manual {
                    ui.label("Manual time · no recorded turns");
                }
                if value_str(record, "penalty") == "+2" {
                    ui.label("Includes a +2 penalty; TPS uses execution time");
                }
                if estimated {
                    ui.label("≈ TPS uses an estimated start or finish time");
                }
                if let Some(flags) = quality {
                    ui.label(format!(
                        "Recording: {}",
                        flags
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if columns.scramble.is_none() {
                    ui.label(format!("Scramble: {} TPS", scramble_rate(record)));
                }
            });
        if clicked {
            self.open_review(record.clone());
        }
    }

    fn stats_empty(&mut self, ui: &mut egui::Ui, title: &str, detail: &str, training: bool) {
        let p = palette(ui.ctx());
        panel(ui, |ui| {
            ui.add_space(8.);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.), egui::Sense::hover());
            design::icon(
                ui.painter(),
                rect,
                if training {
                    design::Icon::Train
                } else {
                    design::Icon::Chart
                },
                p.accent,
            );
            ui.add_space(12.);
            ui.label(RichText::new(title).font(medium(21.)).color(p.text));
            ui.label(RichText::new(detail).font(font(14.)).color(p.muted));
            ui.add_space(14.);
            if design::primary(
                ui,
                if training {
                    "Choose a case"
                } else {
                    "Go to practice"
                },
            )
            .clicked()
            {
                self.page = if training { "Training" } else { "Practice" }.into();
            }
            ui.add_space(8.);
        });
    }
}

fn history_matches(record: &Value, session: &str, kind: &str) -> bool {
    (session == "All sessions" || value_str(record, "session") == session)
        && (kind == "All attempts"
            || value_str(record, "kind") == if kind == "Solves" { "solve" } else { "drill" })
}

struct HistoryColumns {
    index: f32,
    date: Option<f32>,
    context: f32,
    time: f32,
    tps: f32,
    scramble: Option<f32>,
    moves: f32,
}
impl HistoryColumns {
    fn new(rect: Rect) -> Self {
        let wide = rect.width() >= 820.;
        let dated = rect.width() >= 620.;
        let right = rect.right() - 14.;
        Self {
            index: rect.left(),
            date: dated.then_some(rect.left() + 64.),
            context: rect.left() + if dated { 206. } else { 48. },
            time: right - if wide { 324. } else { 160. },
            tps: right - if wide { 206. } else { 70. },
            scramble: wide.then_some(right - 88.),
            moves: right,
        }
    }
}

fn panel<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.surface)
        .corner_radius(12)
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            content(ui)
        })
}
fn section_line(ui: &mut egui::Ui, title: &str, aside: &str) {
    let p = palette(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(title).font(medium(15.)).color(p.text));
        ui.add_space(8.);
        ui.label(RichText::new(aside).font(font(11.)).color(p.muted));
    });
}
fn summary_strip(ui: &mut egui::Ui, metrics: &[(&str, String, String)]) {
    let p = palette(ui.ctx());
    let columns = if ui.available_width() < 590. { 2 } else { 4 };
    for row in metrics.chunks(columns) {
        ui.columns(columns, |cells| {
            for (cell, (label, value, detail)) in cells.iter_mut().zip(row) {
                cell.label(RichText::new(*label).font(font(12.)).color(p.muted));
                cell.add_space(2.);
                cell.label(RichText::new(value).font(display(32.)).color(p.text));
                cell.label(RichText::new(detail).font(font(11.)).color(p.muted));
            }
        });
        ui.add_space(12.);
    }
}
fn phases(records: &[&Value]) -> Vec<Phase> {
    ["Cross", "F2L", "OLL", "PLL"]
        .into_iter()
        .filter_map(|name| {
            let splits = records
                .iter()
                .rev()
                .take(30)
                .filter_map(|r| {
                    r["analysis"]["splits"]
                        .as_array()?
                        .iter()
                        .find(|s| value_str(s, "stage") == name && s["time"].as_f64().is_some())
                })
                .collect::<Vec<_>>();
            if splits.is_empty() {
                return None;
            }
            let count = splits.len();
            let duration = splits.iter().map(|s| number(s, "time")).sum::<f64>();
            let moves = splits.iter().map(|s| number(s, "moves")).sum::<f64>();
            Some(Phase {
                name,
                time: duration / count as f64,
                pause: splits.iter().map(|s| number(s, "pause")).sum::<f64>() / count as f64,
                rate: if duration > 0. {
                    Some(moves * 1000. / duration)
                } else {
                    None
                },
                count,
            })
        })
        .collect()
}
fn phase_summary(ui: &mut egui::Ui, phases: &[Phase]) {
    let p = palette(ui.ctx());
    let total = phases.iter().map(|s| s.time).sum::<f64>();
    let columns = if ui.available_width() < 560. { 2 } else { 4 };
    for row in phases.chunks(columns) {
        ui.columns(columns, |cells| {
            for (cell, phase) in cells.iter_mut().zip(row) {
                cell.label(RichText::new(phase.name).font(medium(12.)).color(p.muted));
                cell.label(
                    RichText::new(format!("{} s", seconds(phase.time)))
                        .font(display(26.))
                        .color(p.text),
                );
                cell.label(
                    RichText::new(
                        phase
                            .rate
                            .map(|r| format!("{r:.2} TPS"))
                            .unwrap_or("— TPS".into()),
                    )
                    .font(font(12.))
                    .color(p.accent),
                );
                let (bar, _) = cell.allocate_exact_size(
                    Vec2::new(cell.available_width(), 4.),
                    egui::Sense::hover(),
                );
                cell.painter().rect_filled(bar, 2, p.raised);
                if total > 0. {
                    cell.painter().rect_filled(
                        Rect::from_min_size(
                            bar.min,
                            Vec2::new(bar.width() * (phase.time / total) as f32, 4.),
                        ),
                        2,
                        p.accent,
                    );
                }
                cell.label(
                    RichText::new(format!(
                        "{:.0}% of solve",
                        if total > 0. {
                            phase.time / total * 100.
                        } else {
                            0.
                        }
                    ))
                    .font(font(11.))
                    .color(p.muted),
                );
            }
        });
        ui.add_space(10.);
    }
}
fn trend(ui: &mut egui::Ui, records: &[Value], height: f32) {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let values = records
        .iter()
        .map(|r| if timed(r) { Some(adjusted(r)) } else { None })
        .collect::<Vec<_>>();
    let finite = values.iter().flatten().copied().collect::<Vec<_>>();
    if finite.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Reliable solve times will appear here",
            font(12.),
            p.muted,
        );
        return;
    }
    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(0., f64::max);
    let padding = ((max - min) * 0.18).max(500.);
    let low = (min - padding).max(0.);
    let high = max + padding;
    let plot = Rect::from_min_max(
        rect.min + Vec2::new(40., 12.),
        rect.max - Vec2::new(12., 27.),
    );
    for i in 0..3 {
        let fraction = i as f32 / 2.;
        let y = plot.top() + plot.height() * fraction;
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(1.0f32, p.line),
        );
        painter.text(
            Pos2::new(rect.left(), y),
            Align2::LEFT_CENTER,
            format!("{:.1}", (high - (high - low) * fraction as f64) / 1000.),
            font(10.),
            p.muted,
        );
    }
    let point = |i: usize, v: f64| {
        Pos2::new(
            plot.left() + plot.width() * i as f32 / records.len().saturating_sub(1).max(1) as f32,
            plot.bottom() - ((v - low) / (high - low)) as f32 * plot.height(),
        )
    };
    let mut previous = None;
    for (i, value) in values.iter().enumerate() {
        if let Some(value) = value {
            let pos = point(i, *value);
            if let Some(last) = previous {
                painter.line_segment([last, pos], Stroke::new(1.8f32, p.accent));
            }
            painter.circle_filled(pos, 2.5, p.accent);
            previous = Some(pos);
        } else {
            previous = None;
            let pos = point(i, high);
            painter.circle_stroke(pos, 3., Stroke::new(1.0f32, p.amber));
        }
    }
    painter.text(
        Pos2::new(plot.left(), rect.bottom() - 7.),
        Align2::LEFT_CENTER,
        "Earlier",
        font(10.),
        p.muted,
    );
    painter.text(
        Pos2::new(plot.right(), rect.bottom() - 7.),
        Align2::RIGHT_CENTER,
        "Latest",
        font(10.),
        p.muted,
    );
    if let Some(pointer) = response.hover_pos() {
        let index = (((pointer.x - plot.left()) / plot.width())
            * records.len().saturating_sub(1) as f32)
            .round()
            .clamp(0., records.len().saturating_sub(1) as f32) as usize;
        if let Some(value) = values[index] {
            let pos = point(index, value);
            painter.line_segment(
                [
                    Pos2::new(pos.x, plot.top()),
                    Pos2::new(pos.x, plot.bottom()),
                ],
                Stroke::new(1.0f32, p.line),
            );
            painter.circle_filled(pos, 4., p.accent);
        }
        response.on_hover_text(format!(
            "Solve {} · {} s · {} TPS",
            index + 1,
            format!(
                "{}{}",
                if crate::recording::execution_duration_estimated(&records[index]) {
                    "≈ "
                } else {
                    ""
                },
                seconds(adjusted(&records[index]))
            ),
            solve_rate(&records[index])
        ));
    }
}
fn solve_rate(record: &Value) -> String {
    crate::recording::execution_metrics(record)
        .map(|metric| metric.label())
        .unwrap_or_else(|| "—".into())
}
fn scramble_rate(record: &Value) -> String {
    if value_str(record, "source") == "manual"
        || record["scrambleQuality"]
            .as_array()
            .is_some_and(|q| !q.is_empty())
    {
        return "—".into();
    }
    rate_number(&record["scrambleStats"])
}
fn rate_number(value: &Value) -> String {
    value["tps"]
        .as_f64()
        .filter(|n| n.is_finite())
        .map(|n| format!("{n:.2}"))
        .unwrap_or("—".into())
}
fn clipped_text(
    painter: &egui::Painter,
    clip: Rect,
    position: Pos2,
    text: &str,
    font: egui::FontId,
    color: Color32,
) {
    painter
        .with_clip_rect(clip)
        .text(position, Align2::LEFT_CENTER, text, font, color);
}
fn hover_row(ui: &egui::Ui, response: &egui::Response, rect: Rect) {
    let p = palette(ui.ctx());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
    if hover > 0. {
        ui.painter()
            .rect_filled(rect, 6, p.surface.gamma_multiply(hover));
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            6,
            Stroke::new(1.0f32, p.accent),
            egui::StrokeKind::Inside,
        );
    }
}
fn activated(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.clicked()
        || (response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)))
}

fn average_detail(records: &[Value], n: usize) -> String {
    let m = crate::practice_ui::average_metrics(records, n);
    format!(
        "{} avg TPS ({}/{}) · {} avg moves ({}/{})",
        m.tps_label(),
        m.tps_samples,
        m.retained,
        m.moves.map(|v| format!("{v:.1}")).unwrap_or("—".into()),
        m.move_samples,
        m.retained,
    )
}
