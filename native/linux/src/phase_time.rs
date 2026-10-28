//! Shared visual hierarchy for CFOP totals and their secondary decomposition.
use crate::{
    design::{mono, mono_bold, palette},
    io,
};
use eframe::egui::{self, Align2, Color32, Pos2};

fn rounded_parts(total: f64, pause: f64) -> (f64, f64, f64) {
    let total = (total.max(0.) / 10.).round() * 10.;
    let pause = if pause.is_finite() {
        (pause.clamp(0., total) / 10.).round() * 10.
    } else {
        0.
    };
    (total, pause, (total - pause).max(0.))
}
pub(crate) fn width(
    ui: &egui::Ui,
    total: Option<f64>,
    pause: f64,
    cross: bool,
    estimated: bool,
    size: f32,
) -> f32 {
    let (total, pause) = total
        .map(|t| {
            let (t, p, _) = rounded_parts(t, pause);
            (Some(t), p)
        })
        .unwrap_or((None, 0.));
    let prefix = if estimated { "≈" } else { "" };
    let primary = total
        .map(|t| format!("{prefix}{}", io::seconds(t)))
        .unwrap_or("—".into());
    let mut width = ui
        .painter()
        .layout_no_wrap(primary, mono_bold(size), palette(ui.ctx()).text)
        .size()
        .x;
    if let Some(total) = total.filter(|_| !cross) {
        let detail = format!(
            "{}+{}",
            io::seconds(pause),
            io::seconds((total - pause).max(0.))
        );
        width = width.max(
            ui.painter()
                .layout_no_wrap(detail, mono(12.), Color32::WHITE)
                .size()
                .x,
        );
    }
    width
}
pub(crate) fn paint(
    ui: &egui::Ui,
    right_top: Pos2,
    total: Option<f64>,
    pause: f64,
    cross: bool,
    estimated: bool,
    size: f32,
) {
    let p = palette(ui.ctx());
    let (total, pause) = total
        .map(|t| {
            let (t, p, _) = rounded_parts(t, pause);
            (Some(t), p)
        })
        .unwrap_or((None, 0.));
    let prefix = if estimated { "≈" } else { "" };
    ui.painter().text(
        right_top,
        Align2::RIGHT_TOP,
        total
            .map(|t| format!("{prefix}{}", io::seconds(t)))
            .unwrap_or("—".into()),
        mono_bold(size),
        if total.is_some() { p.text } else { p.faint },
    );
    if let Some(total) = total.filter(|_| !cross) {
        let color = if p.bg.r() > 128 {
            Color32::from_rgb(72, 123, 198)
        } else {
            Color32::from_rgb(134, 185, 239)
        };
        let mut job = egui::text::LayoutJob::default();
        for (value, color) in [
            (io::seconds(pause), color),
            ("+".into(), p.faint),
            (io::seconds((total - pause).max(0.)), p.muted),
        ] {
            job.append(
                &value,
                0.,
                egui::TextFormat {
                    font_id: mono(12.),
                    color,
                    ..Default::default()
                },
            );
        }
        let galley = ui.painter().layout_job(job);
        ui.painter().galley(
            right_top + egui::vec2(-galley.size().x, size + 2.),
            galley,
            Color32::WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_addends_equal_the_displayed_total_including_minute_carry() {
        for (total, pause, expected) in [
            (3845., 555., (3850., 560., 3290.)),
            (1995., 997., (2000., 1000., 1000.)),
            (59995., 1205., (60000., 1210., 58790.)),
            (10., 100., (10., 10., 0.)),
        ] {
            let parts = rounded_parts(total, pause);
            assert_eq!(parts, expected);
            assert_eq!(parts.0, parts.1 + parts.2);
        }
    }
}
