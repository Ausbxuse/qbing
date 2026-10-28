use crate::{
    cube::Cube,
    design::{self, palette},
    io::{number, seconds, timed, value_str},
    render,
    ui::{App, caption, primary},
};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};
use std::collections::{HashMap, HashSet};

pub struct TrainingRun {
    pub case_id: String,
    pub variant: usize,
    pub baseline: HashSet<String>,
}
struct Candidate {
    case_id: String,
    name: String,
    stage: String,
    variant: usize,
    count: usize,
    success: usize,
    recognition: Option<f64>,
    execution: Option<f64>,
    reason: String,
    priority: f64,
}

impl App {
    pub(crate) fn training_page(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        stage_switcher(ui, &mut self.stage, true);
        ui.add_space(16.);
        if self.training.is_some() {
            self.training_progress(ui);
            ui.add_space(16.);
        }
        let candidates = self.training_candidates();
        let has_history = candidates.iter().any(|c| c.count > 0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if has_history {
                    "Recommended next"
                } else {
                    "Start with one case"
                })
                .font(design::medium(16.))
                .color(p.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("5 clean repetitions")
                        .font(design::font(12.))
                        .color(p.muted),
                );
            });
        });
        ui.add_space(8.);
        if let Some(first) = candidates.first() {
            self.training_row(ui, first, true);
        }
        ui.add_space(20.);
        ui.label(
            RichText::new("Practice queue")
                .font(design::medium(14.))
                .color(p.text),
        );
        ui.add_space(4.);
        for item in candidates.iter().skip(1).take(11) {
            self.training_row(ui, item, false);
        }
        ui.add_space(16.);
        caption(
            ui,
            "Your learning status, accuracy, and review intervals shape this queue.",
        );
    }

    fn training_row(&mut self, ui: &mut egui::Ui, item: &Candidate, featured: bool) {
        let p = palette(ui.ctx());
        let width = ui.available_width();
        let compact = width < 620.;
        let height = if featured {
            if compact { 166. } else { 140. }
        } else if compact {
            126.
        } else {
            98.
        };
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!("Learn {}: {}", item.name, item.reason),
            )
        });
        let hover = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered() || response.has_focus(),
            0.12,
        );
        let painter = ui.painter_at(rect);
        if featured || hover > 0. {
            painter.rect_filled(
                rect,
                12.,
                mix(if featured { p.surface } else { p.bg }, p.raised, hover),
            );
        }
        if featured || response.has_focus() {
            painter.rect_stroke(
                rect,
                12.,
                Stroke::new(
                    1.0f32,
                    if response.has_focus() {
                        p.accent
                    } else {
                        p.line
                    },
                ),
                StrokeKind::Inside,
            );
        } else {
            painter.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(1.0f32, p.line),
            );
        }
        let pattern_size = if featured { 78. } else { 60. };
        let pattern_rect = Rect::from_min_size(
            rect.min
                + Vec2::new(
                    if featured { 18. } else { 8. },
                    if featured { 22. } else { 17. },
                ),
            Vec2::splat(pattern_size),
        );
        let case = self
            .model
            .catalog
            .cases
            .iter()
            .find(|c| c.id == item.case_id)
            .unwrap();
        paint_pattern(
            &painter,
            pattern_rect,
            Cube::parse(&case.variants[item.variant].positions["D"].state).unwrap(),
            &item.stage,
            p.line,
        );
        let text_x = pattern_rect.right() + 18.;
        let text_width = (rect.right() - text_x - if compact { 18. } else { 158. }).max(70.);
        let text_y = rect.top() + if featured { 22. } else { 17. };
        paint_text(
            &painter,
            Pos2::new(text_x, text_y),
            &item.name,
            design::medium(if featured { 19. } else { 16. }),
            p.text,
            text_width,
        );
        paint_text(
            &painter,
            Pos2::new(text_x, text_y + 28.),
            &item.reason,
            design::font(13.),
            p.muted,
            text_width,
        );
        let evidence = if item.count == 0 {
            format!(
                "{} algorithms · learn the pattern first",
                case.variants.len()
            )
        } else {
            let mut parts = vec![format!("{} / {} clean", item.success, item.count)];
            if let Some(ms) = item.execution {
                parts.push(format!("{} s execution", seconds(ms)));
            }
            if let Some(ms) = item.recognition {
                parts.push(format!("{} s recognition", seconds(ms)));
            }
            parts.join(" · ")
        };
        if featured || item.count > 0 {
            paint_text(
                &painter,
                Pos2::new(text_x, text_y + 51.),
                &evidence,
                design::font(12.),
                p.muted,
                text_width,
            );
        }
        let button_rect = Rect::from_min_size(
            Pos2::new(
                if compact { text_x } else { rect.right() - 134. },
                if compact {
                    rect.bottom() - 47.
                } else {
                    rect.center().y - 20.
                },
            ),
            Vec2::new(116., 38.),
        );
        let can_start = self.model.active.is_none() && self.model.countdown.is_none();
        let button = egui::Button::new(
            RichText::new("Practice 5")
                .font(design::medium(13.))
                .color(if featured { p.on_accent } else { p.text }),
        )
        .fill(if featured { p.accent } else { p.raised })
        .stroke(Stroke::NONE)
        .corner_radius(8);
        let action = ui
            .add_enabled_ui(can_start, |ui| ui.put(button_rect, button))
            .inner
            .on_hover_text(if can_start {
                "Start five clean repetitions of this case"
            } else {
                "Finish or cancel the current attempt first"
            });
        if action.clicked() {
            self.start_training(&item.case_id, item.variant);
        } else if clicked(ui, &response) {
            self.selected = item.case_id.clone();
            self.variant = item.variant;
            self.page = "Case".into();
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
        ui.advance_cursor_after_rect(rect);
    }

    pub(crate) fn start_training(&mut self, id: &str, variant: usize) {
        if self.model.active.is_some() || self.model.countdown.is_some() {
            self.model.message =
                "Finish or cancel the current attempt before starting another case.".into();
            return;
        }
        self.training = Some(TrainingRun {
            case_id: id.into(),
            variant,
            baseline: self
                .model
                .records
                .iter()
                .map(|r| value_str(r, "id").to_string())
                .collect(),
        });
        self.model.prepare_case(id, variant);
        self.page = "Practice".into();
    }

    pub(crate) fn training_progress(&mut self, ui: &mut egui::Ui) {
        let Some(run) = &self.training else { return };
        let p = palette(ui.ctx());
        let id = run.case_id.clone();
        let variant = run.variant;
        let attempts = self
            .model
            .records
            .iter()
            .filter(|r| value_str(r, "caseId") == id && !run.baseline.contains(value_str(r, "id")))
            .collect::<Vec<_>>();
        let clean = attempts.iter().filter(|r| timed(r)).count();
        let count = attempts.len();
        let mean = (clean > 0).then(|| {
            attempts
                .iter()
                .filter(|r| timed(r))
                .map(|r| number(r, "duration"))
                .sum::<f64>()
                / clean as f64
        });
        let name = self
            .model
            .catalog
            .cases
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Case practice".into());
        let busy = self.model.active.is_some() || self.model.countdown.is_some();
        let waiting_for_setup = self.model.target.is_some();
        egui::Frame::new()
            .fill(p.surface)
            .corner_radius(10)
            .inner_margin(16)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(name).font(design::medium(14.)).color(p.text));
                    ui.label(
                        RichText::new(format!("{} of 5 clean", clean.min(5)))
                            .font(design::font(12.))
                            .color(if clean >= 5 { p.green } else { p.muted }),
                    );
                });
                ui.add_space(4.);
                let width = ui.available_width();
                let (bar, _) = ui.allocate_exact_size(Vec2::new(width, 4.), egui::Sense::hover());
                let segment_width = (width - 16.) / 5.;
                for i in 0..5 {
                    let segment = Rect::from_min_size(
                        bar.min + Vec2::new(i as f32 * (segment_width + 4.), 0.),
                        Vec2::new(segment_width, 4.),
                    );
                    ui.painter().rect_filled(
                        segment,
                        2.,
                        if i < clean { p.accent } else { p.line },
                    );
                }
                ui.add_space(6.);
                ui.horizontal_wrapped(|ui| {
                    let detail = if let Some(ms) = mean {
                        format!("{} s mean execution · {count} attempts", seconds(ms))
                    } else {
                        "Match the setup, then start the recognition cue.".into()
                    };
                    caption(ui, &detail);
                });
                ui.horizontal_wrapped(|ui| {
                    if clean >= 5 && !busy {
                        if primary(ui, "Finish session").clicked() {
                            self.training = None;
                            self.page = "Training".into();
                        }
                    } else if !busy && !waiting_for_setup && ui.button("Next repetition").clicked()
                    {
                        self.model.prepare_case(&id, variant);
                        self.page = "Practice".into();
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if busy { "Stop session" } else { "End session" })
                                    .font(design::font(12.))
                                    .color(p.muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.model.interrupt("user-stopped");
                        self.training = None;
                    }
                });
            });
    }

    fn training_candidates(&self) -> Vec<Candidate> {
        let mut result = vec![];
        for case in self
            .model
            .catalog
            .cases
            .iter()
            .filter(|c| self.stage == "All" || c.stage == self.stage)
        {
            let records = self
                .model
                .records
                .iter()
                .filter(|r| value_str(r, "caseId") == case.id)
                .collect::<Vec<_>>();
            let count = records.len();
            let success = records.iter().filter(|r| timed(r)).count();
            let performance = self
                .model
                .stats
                .iter()
                .find(|p| value_str(p, "caseId") == case.id);
            let algorithm = performance.map(|p| &p["algorithms"][0]);
            let recognition = algorithm.and_then(|a| a["meanRecognition"].as_f64());
            let execution = algorithm.and_then(|a| a["meanExecution"].as_f64());
            let status = self.model.preferences[&case.id]["status"]
                .as_str()
                .unwrap_or("new");
            let favorite = self.model.preferences[&case.id]["favorite"] == true;
            let streak = records.iter().rev().take_while(|r| timed(r)).count();
            let interval = match streak {
                0..=2 => 0,
                3..=4 => 1,
                5..=9 => 2,
                10..=19 => 4,
                _ => 7,
            };
            let days = records
                .last()
                .and_then(|r| chrono::DateTime::parse_from_rfc3339(value_str(r, "createdAt")).ok())
                .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days())
                .unwrap_or(i64::MAX);
            let (reason, priority) = if count >= 3 && success as f64 / (count as f64) < 0.8 {
                (
                    "Build accuracy before speed".into(),
                    1000. + (count - success) as f64 * 20.,
                )
            } else if status == "learning" {
                ("Continue where you left off".into(), 850.)
            } else if count > 0 && days >= interval {
                (
                    "Due for a review".into(),
                    600. + execution.unwrap_or(0.) / 100.,
                )
            } else if favorite && count < 5 {
                ("Build a baseline for this favorite".into(), 500.)
            } else if count < 5 {
                ("Learn, then repeat without rushing".into(), 200.)
            } else {
                (
                    format!(
                        "Next review in {} day{}",
                        (interval - days).max(1),
                        if interval - days == 1 { "" } else { "s" }
                    ),
                    50.,
                )
            };
            let variant = (self.model.preferences[&case.id]["variant"]
                .as_u64()
                .unwrap_or(0) as usize)
                .min(case.variants.len() - 1);
            result.push(Candidate {
                case_id: case.id.clone(),
                name: case.name.clone(),
                stage: case.stage.clone(),
                variant,
                count,
                success,
                recognition,
                execution,
                reason,
                priority,
            });
        }
        result.sort_by(|a, b| b.priority.total_cmp(&a.priority));
        result
    }

    pub(crate) fn case_grid(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        caption(ui, "41 F2L · 57 OLL · 21 PLL · Tao Yu / Alg-Trainer");
        ui.add_space(12.);
        let narrow = ui.available_width() < 650.;
        if narrow {
            stage_switcher(ui, &mut self.stage, true);
            ui.add_space(8.);
            search_field(ui, &mut self.search, ui.available_width());
        } else {
            ui.horizontal(|ui| {
                stage_switcher(ui, &mut self.stage, true);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    search_field(ui, &mut self.search, 250.);
                });
            });
        }
        let search = self.search.to_lowercase();
        let cases = self
            .model
            .catalog
            .cases
            .iter()
            .filter(|c| {
                (self.stage == "All" || c.stage == self.stage)
                    && (search.is_empty()
                        || c.name.to_lowercase().contains(&search)
                        || c.variants
                            .iter()
                            .any(|v| v.text.to_lowercase().contains(&search)))
            })
            .collect::<Vec<_>>();
        let rates = case_tps(&self.model.records);
        let learned = cases
            .iter()
            .filter(|c| self.model.preferences[&c.id]["status"] == "confident")
            .count();
        ui.add_space(12.);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{} cases", cases.len()))
                    .font(design::medium(13.))
                    .color(p.text),
            );
            if learned > 0 {
                caption(ui, &format!("{learned} learned"));
            } else {
                caption(
                    ui,
                    "Choose a case to learn its pattern and track your progress.",
                );
            }
        });
        ui.add_space(10.);
        if cases.is_empty() {
            egui::Frame::new()
                .fill(p.surface)
                .corner_radius(12)
                .inner_margin(24)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("No matching cases")
                            .font(design::medium(17.))
                            .color(p.text),
                    );
                    caption(
                        ui,
                        "Try a case name such as “T perm” or choose another stage.",
                    );
                    if ui.button("Clear search").clicked() {
                        self.search.clear();
                    }
                });
            return;
        }
        let columns = ((ui.available_width() + 12.) / 330.).floor().max(1.) as usize;
        let width = (ui.available_width() - (columns - 1) as f32 * 12.) / columns as f32;
        egui::Grid::new("case-grid")
            .num_columns(columns)
            .spacing(Vec2::new(12., 12.))
            .show(ui, |ui| {
                for (i, case) in cases.iter().enumerate() {
                    let selected_variant = (self.model.preferences[&case.id]["variant"]
                        .as_u64()
                        .unwrap_or(0) as usize)
                        .min(case.variants.len() - 1);
                    let status = self.model.preferences[&case.id]["status"]
                        .as_str()
                        .unwrap_or("new");
                    let performance = self
                        .model
                        .stats
                        .iter()
                        .find(|s| value_str(s, "caseId") == case.id);
                    let execution =
                        performance.and_then(|s| s["algorithms"][0]["meanExecution"].as_f64());
                    let recognition =
                        performance.and_then(|s| s["algorithms"][0]["meanRecognition"].as_f64());
                    let count = performance
                        .map(|s| number(s, "count") as usize)
                        .unwrap_or(0);
                    let tps = rates.get(&case.id).map(|(sum, count)| sum / *count as f64);
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, 128.), egui::Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!(
                                "{}: {}, {} algorithms",
                                case.name,
                                status_name(status),
                                case.variants.len()
                            ),
                        )
                    });
                    let hover = ui.ctx().animate_bool_with_time(
                        response.id.with("hover"),
                        response.hovered() || response.has_focus(),
                        0.12,
                    );
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 12., mix(p.surface, p.raised, hover));
                    painter.rect_stroke(
                        rect,
                        12.,
                        Stroke::new(
                            1.0f32,
                            if response.has_focus() {
                                p.accent
                            } else {
                                p.line
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    let pattern =
                        Rect::from_min_size(rect.min + Vec2::new(16., 25.), Vec2::splat(72.));
                    paint_pattern(
                        &painter,
                        pattern,
                        Cube::parse(&case.variants[selected_variant].positions["D"].state).unwrap(),
                        &case.stage,
                        p.line,
                    );
                    let x = rect.left() + 106.;
                    let content_width = rect.right() - x - 16.;
                    paint_text(
                        &painter,
                        Pos2::new(x, rect.top() + 17.),
                        &case.name,
                        design::medium(17.),
                        p.text,
                        content_width - 62.,
                    );
                    let status_color = match status {
                        "confident" => p.green,
                        "learning" => p.amber,
                        _ => p.muted,
                    };
                    painter.text(
                        Pos2::new(rect.right() - 16., rect.top() + 21.),
                        Align2::RIGHT_TOP,
                        status_name(status),
                        design::font(11.),
                        status_color,
                    );
                    if let Some(ms) = execution {
                        painter.text(
                            Pos2::new(x, rect.top() + 48.),
                            Align2::LEFT_TOP,
                            format!("{} s", seconds(ms)),
                            design::medium(23.),
                            p.text,
                        );
                        painter.text(
                            Pos2::new(x, rect.top() + 80.),
                            Align2::LEFT_TOP,
                            "execution",
                            design::font(11.),
                            p.muted,
                        );
                        let metric_x = x + (content_width * 0.55).max(102.);
                        if let Some(ms) = recognition {
                            painter.text(
                                Pos2::new(metric_x, rect.top() + 53.),
                                Align2::LEFT_TOP,
                                format!("{} s", seconds(ms)),
                                design::medium(17.),
                                p.text,
                            );
                            painter.text(
                                Pos2::new(metric_x, rect.top() + 80.),
                                Align2::LEFT_TOP,
                                "recognition",
                                design::font(11.),
                                p.muted,
                            );
                        }
                        let footer = format!(
                            "{count} executions{}",
                            tps.map(|v| format!(" · {v:.2} TPS")).unwrap_or_default()
                        );
                        paint_text(
                            &painter,
                            Pos2::new(x, rect.top() + 105.),
                            &footer,
                            design::font(11.),
                            p.muted,
                            content_width,
                        );
                    } else {
                        paint_text(
                            &painter,
                            Pos2::new(x, rect.top() + 50.),
                            &case.variants[selected_variant].text,
                            design::mono(13.),
                            p.muted,
                            content_width,
                        );
                        paint_text(
                            &painter,
                            Pos2::new(x, rect.top() + 85.),
                            &format!(
                                "{} algorithm{}",
                                case.variants.len(),
                                if case.variants.len() == 1 { "" } else { "s" }
                            ),
                            design::font(12.),
                            p.muted,
                            content_width - 14.,
                        );
                        let tip = Pos2::new(rect.right() - 20., rect.bottom() - 33.);
                        painter.line_segment(
                            [tip + Vec2::new(-3., -4.), tip + Vec2::new(1., 0.)],
                            Stroke::new(1.25f32, p.muted),
                        );
                        painter.line_segment(
                            [tip + Vec2::new(1., 0.), tip + Vec2::new(-3., 4.)],
                            Stroke::new(1.25f32, p.muted),
                        );
                    }
                    if clicked(ui, &response) {
                        self.selected = case.id.clone();
                        self.variant = selected_variant;
                        self.page = "Case".into();
                    }
                    response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    if (i + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }
}

fn case_tps(records: &[serde_json::Value]) -> HashMap<String, (f64, usize)> {
    let mut rates = HashMap::<String, (f64, usize)>::new();
    let mut add = |id: &str, value: Option<f64>| {
        if !id.is_empty()
            && let Some(value) = value.filter(|v| v.is_finite())
        {
            let entry = rates.entry(id.to_owned()).or_default();
            entry.0 += value;
            entry.1 += 1;
        }
    };
    for record in records.iter().filter(|record| timed(record)) {
        let id = value_str(record, "caseId");
        if !id.is_empty() {
            add(id, record["analysis"]["tps"].as_f64());
        } else if let Some(splits) = record["analysis"]["splits"].as_array() {
            for split in splits {
                add(value_str(split, "caseId"), split["tps"].as_f64());
            }
        }
    }
    rates
}

fn search_field(ui: &mut egui::Ui, value: &mut String, width: f32) {
    let p = palette(ui.ctx());
    ui.allocate_ui_with_layout(
        Vec2::new(width, 38.),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::Frame::new()
                .fill(p.surface)
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(12, 4))
                .show(ui, |ui| {
                    ui.set_width(width - 24.);
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::splat(16.), egui::Sense::hover());
                        ui.painter().circle_stroke(
                            rect.center() - Vec2::splat(1.5),
                            4.5,
                            Stroke::new(1.25f32, p.muted),
                        );
                        ui.painter().line_segment(
                            [
                                rect.center() + Vec2::splat(2.),
                                rect.center() + Vec2::splat(6.),
                            ],
                            Stroke::new(1.25f32, p.muted),
                        );
                        ui.add(
                            egui::TextEdit::singleline(value)
                                .frame(false)
                                .hint_text("Search cases or algorithms")
                                .font(design::font(13.))
                                .desired_width((width - 52.).max(1.)),
                        );
                    });
                });
        },
    );
}

fn stage_switcher(ui: &mut egui::Ui, selected: &mut String, include_all: bool) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.surface)
        .corner_radius(9)
        .inner_margin(4)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.;
                for stage in ["All", "Cross", "F2L", "OLL", "PLL"] {
                    if stage == "All" && !include_all {
                        continue;
                    }
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(if stage == "Cross" { 62. } else { 48. }, 32.),
                        egui::Sense::click(),
                    );
                    let active = selected == stage;
                    let hover = ui.ctx().animate_bool_with_time(
                        response.id.with("hover"),
                        response.hovered(),
                        0.12,
                    );
                    if active || hover > 0. {
                        ui.painter().rect_filled(
                            rect,
                            6.,
                            if active {
                                p.raised
                            } else {
                                mix(p.surface, p.raised, hover)
                            },
                        );
                    }
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            6.,
                            Stroke::new(1.0f32, p.accent),
                            StrokeKind::Inside,
                        );
                    }
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        stage,
                        if active {
                            design::medium(13.)
                        } else {
                            design::font(13.)
                        },
                        if active { p.text } else { p.muted },
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("{stage} cases"),
                        )
                    });
                    if clicked(ui, &response) {
                        *selected = stage.into();
                    }
                    response.on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            });
        });
}

fn clicked(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.clicked()
        || (response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)))
}
fn status_name(value: &str) -> &'static str {
    match value {
        "confident" => "Learned",
        "learning" => "Learning",
        _ => "New",
    }
}
fn paint_text(
    painter: &egui::Painter,
    at: Pos2,
    text: &str,
    font: egui::FontId,
    color: Color32,
    width: f32,
) {
    let mut job = egui::text::LayoutJob::simple(text.into(), font, color, width.max(1.));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    painter.galley(at, painter.layout_job(job), color);
}
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0., 1.);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

pub fn case_pattern(ui: &mut egui::Ui, state: Cube, stage: &str, width: f32) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(width), egui::Sense::hover());
    paint_pattern(&ui.painter_at(rect), rect, state, stage, p.line);
}
fn paint_pattern(painter: &egui::Painter, rect: Rect, state: Cube, stage: &str, neutral: Color32) {
    if stage == "Cross" || stage == "F2L" {
        render::paint_cube(painter.clone(), rect, state, -0.58, 0.48, None);
        return;
    }
    let unit = rect.width() / 5.3;
    let paint = |x: f32, y: f32, c: u8| {
        let color = if stage == "OLL" {
            if c == b'U' {
                render::face_color(b'D')
            } else {
                neutral
            }
        } else {
            render::face_color(if c == b'U' {
                b'D'
            } else if c == b'D' {
                b'U'
            } else {
                c
            })
        };
        painter.rect_filled(
            Rect::from_min_size(
                rect.min + Vec2::new(x * unit, y * unit),
                Vec2::splat(unit * 0.88),
            ),
            1.5,
            color,
        );
    };
    for row in 0..3 {
        for col in 0..3 {
            paint(1.15 + col as f32, 1.15 + row as f32, state.0[row * 3 + col]);
        }
    }
    for j in 0..3 {
        paint(1.15 + j as f32, 0.1, state.0[47 - j]);
        paint(1.15 + j as f32, 4.2, state.0[18 + j]);
        paint(0.1, 1.15 + j as f32, state.0[36 + j]);
        paint(4.2, 1.15 + j as f32, state.0[11 - j]);
    }
}
