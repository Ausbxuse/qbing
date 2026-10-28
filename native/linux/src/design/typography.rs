//! Timer-specific coverage rendering. Egui's ordinary atlas is retained for UI
//! text; the large timer uses exact physical-pixel masks with two-dimensional
//! subpixel phases and coverage blended in linear light. This avoids applying
//! a dark-mode alpha boost before another texture filtering step.
use ab_glyph::{Font, FontArc, Glyph, OutlinedGlyph, PxScale, PxScaleFactor, point};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, OnceLock},
};

const SUPERSAMPLE: i32 = 4;
const PHASES: f32 = 4.;
const MAX_GLYPHS: usize = 128;
const TIMER_BYTES: &[u8] = include_bytes!("../../assets/CubeLabTimer-Regular.ttf");

fn face() -> &'static FontArc {
    static FACE: OnceLock<FontArc> = OnceLock::new();
    FACE.get_or_init(|| FontArc::try_from_slice(TIMER_BYTES).expect("Bundled timer font"))
}
#[derive(Clone)]
struct TimerRun {
    text: String,
    size: f32,
    x: f32,
}
struct TimerLayout {
    runs: Vec<TimerRun>,
    width: f32,
    ink_top: f32,
    ink_bottom: f32,
}
fn advance(text: &str, size: f32) -> f32 {
    let face = face();
    let scale = size / face.units_per_em().unwrap();
    text.chars()
        .map(|c| face.h_advance_unscaled(face.glyph_id(c)) * scale)
        .sum::<f32>()
        - text.chars().count().saturating_sub(1) as f32 * size * 0.018
}
fn layout(text: &str, size: f32) -> TimerLayout {
    let (seconds, fraction) = text
        .split_once('.')
        .map_or((text, None), |(whole, part)| (whole, Some(part)));
    let mut width = advance(seconds, size);
    let mut runs = vec![TimerRun {
        text: seconds.into(),
        size,
        x: 0.,
    }];
    if let Some(fraction) = fraction {
        let text = format!(".{fraction}");
        runs.push(TimerRun {
            text: text.clone(),
            size: size * 0.82,
            x: width,
        });
        width += advance(&text, size * 0.82);
    }
    // Reference outline bounds, rather than a changing digit's pixel box, keep
    // the baseline fixed. ab_glyph stores Y bounds in font coordinates.
    let reference = if text.chars().any(|c| c.is_ascii_digit()) {
        "08"
    } else {
        text
    };
    let scale = size / face().units_per_em().unwrap();
    let mut ink_top = f32::INFINITY;
    let mut ink_bottom = f32::NEG_INFINITY;
    for c in reference.chars() {
        if let Some(outline) = face().outline(face().glyph_id(c)) {
            ink_top = ink_top.min(-outline.bounds.min.y * scale);
            ink_bottom = ink_bottom.max(-outline.bounds.max.y * scale);
        }
    }
    if !ink_top.is_finite() {
        ink_top = -size * 0.7;
        ink_bottom = 0.;
    }
    TimerLayout {
        runs,
        width,
        ink_top,
        ink_bottom,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GlyphKey {
    character: char,
    size: u32,
    dpi: u32,
    phase_x: u8,
    phase_y: u8,
}
struct Mask {
    size: [usize; 2],
    offset: [i32; 2],
    coverage: Vec<f32>,
}
struct CachedGlyph {
    key: GlyphKey,
    mask: Mask,
    tone: [Color32; 2],
    texture: egui::TextureHandle,
}
#[derive(Default)]
struct GlyphCache {
    entries: VecDeque<CachedGlyph>,
    rasterizations: usize,
}

fn pixel_phase(position: f32) -> (i32, u8) {
    let quantized = (position * PHASES).round() as i32;
    (
        quantized.div_euclid(PHASES as i32),
        quantized.rem_euclid(PHASES as i32) as u8,
    )
}
fn rasterize(key: GlyphKey) -> Mask {
    let size = f32::from_bits(key.size);
    let dpi = f32::from_bits(key.dpi);
    let face = face();
    let id = face.glyph_id(key.character);
    let Some(outline) = face.outline(id) else {
        return Mask {
            size: [1, 1],
            offset: [0, 0],
            coverage: vec![0.],
        };
    };
    let scale = size * dpi * SUPERSAMPLE as f32 / face.units_per_em().unwrap();
    let glyph = Glyph {
        id,
        scale: PxScale::from(0.),
        position: point(key.phase_x as f32, key.phase_y as f32),
    };
    let outlined = OutlinedGlyph::new(
        glyph,
        outline,
        PxScaleFactor {
            horizontal: scale,
            vertical: scale,
        },
    );
    let bounds = outlined.px_bounds();
    let min_x = (bounds.min.x as i32).div_euclid(SUPERSAMPLE);
    let min_y = (bounds.min.y as i32).div_euclid(SUPERSAMPLE);
    let max_x = ((bounds.max.x as i32) + SUPERSAMPLE - 1).div_euclid(SUPERSAMPLE);
    let max_y = ((bounds.max.y as i32) + SUPERSAMPLE - 1).div_euclid(SUPERSAMPLE);
    let width = (max_x - min_x).max(1) as usize;
    let height = (max_y - min_y).max(1) as usize;
    let mut coverage = vec![0.; width * height];
    // Area-average raw coverage BEFORE alpha/gamma encoding. This is a pixel
    // footprint filter, not minification via one bilinear texture sample.
    outlined.draw(|x, y, covered| {
        let px = (bounds.min.x as i32 + x as i32).div_euclid(SUPERSAMPLE) - min_x;
        let py = (bounds.min.y as i32 + y as i32).div_euclid(SUPERSAMPLE) - min_y;
        coverage[py as usize * width + px as usize] +=
            covered.clamp(0., 1.) / (SUPERSAMPLE * SUPERSAMPLE) as f32;
    });
    Mask {
        size: [width, height],
        offset: [min_x, min_y],
        coverage,
    }
}
fn linear(srgb: f32) -> f32 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}
fn srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1. / 2.4) - 0.055
    }
}
fn coverage_alpha(coverage: f32, foreground: Color32, background: Color32) -> f32 {
    let coverage = coverage.clamp(0., 1.);
    if coverage <= 0. || coverage >= 1. {
        return coverage;
    }
    // Egui blends premultiplied sRGB colors. Find the common alpha whose
    // blended RGB most closely matches linear-light coverage for this theme.
    // This is grayscale AA: no assumption about RGB/BGR panel orientation.
    let mut numerator = 0.;
    let mut denominator = 0.;
    for channel in 0..3 {
        let fg = foreground[channel] as f32 / 255.;
        let bg = background[channel] as f32 / 255.;
        let target = srgb(linear(bg) + coverage * (linear(fg) - linear(bg)));
        let delta = fg - bg;
        numerator += delta * (target - bg);
        denominator += delta * delta;
    }
    if denominator < 1e-6 {
        coverage
    } else {
        (numerator / denominator).clamp(0., 1.)
    }
}
fn image(mask: &Mask, tone: [Color32; 2]) -> egui::ColorImage {
    let mut table = [0u8; 256];
    for (i, alpha) in table.iter_mut().enumerate() {
        *alpha = (coverage_alpha(i as f32 / 255., tone[0], tone[1]) * 255.).round() as u8;
    }
    egui::ColorImage::new(
        mask.size,
        mask.coverage
            .iter()
            .map(|c| Color32::from_white_alpha(table[(c.clamp(0., 1.) * 255.).round() as usize]))
            .collect(),
    )
}
impl GlyphCache {
    fn glyph(
        &mut self,
        ctx: &egui::Context,
        key: GlyphKey,
        tone: [Color32; 2],
    ) -> (egui::TextureId, [usize; 2], [i32; 2]) {
        let mut entry = if let Some(index) = self.entries.iter().position(|entry| entry.key == key)
        {
            self.entries.remove(index).unwrap()
        } else {
            let mask = rasterize(key);
            let texture = ctx.load_texture(
                "timer-coverage",
                image(&mask, tone),
                egui::TextureOptions::NEAREST,
            );
            self.rasterizations += 1;
            CachedGlyph {
                key,
                mask,
                tone,
                texture,
            }
        };
        if entry.tone != tone {
            // Keep the raw mask: status color/theme changes never rerasterize outlines.
            entry
                .texture
                .set(image(&entry.mask, tone), egui::TextureOptions::NEAREST);
            entry.tone = tone;
        }
        let result = (entry.texture.id(), entry.mask.size, entry.mask.offset);
        self.entries.push_back(entry);
        while self.entries.len() > MAX_GLYPHS {
            self.entries.pop_front();
        }
        result
    }
}

/// Stable typesetting plus timer-only, DPI-aware coverage masks. Every mask pixel
/// maps to exactly one physical display pixel; nearest sampling is intentional
/// because coverage has already been filtered, avoiding a second blur pass.
pub fn paint_timer(ui: &egui::Ui, center: Pos2, text: &str, size: f32, color: Color32) -> Rect {
    let mut laid = layout(text, size);
    let available = (ui.clip_rect().width() - 32.).max(180.);
    if laid.width > available {
        laid = layout(text, size * available / laid.width);
    }
    let left = center.x - laid.width / 2.;
    let baseline = center.y - (laid.ink_top + laid.ink_bottom) / 2.;
    let dpi = ui.ctx().pixels_per_point();
    let cache = ui.ctx().data_mut(|data| {
        data.get_temp_mut_or_default::<Arc<Mutex<GlyphCache>>>(egui::Id::new(
            "timer-coverage-cache",
        ))
        .clone()
    });
    let mut cache = cache.lock().unwrap();
    let tone = [color, super::palette(ui.ctx()).bg];
    let scale = 1. / face().units_per_em().unwrap();
    for part in laid.runs {
        let mut x = left + part.x;
        for c in part.text.chars() {
            let (base_x, phase_x) = pixel_phase(x * dpi);
            let (base_y, phase_y) = pixel_phase(baseline * dpi);
            let key = GlyphKey {
                character: c,
                size: part.size.to_bits(),
                dpi: dpi.to_bits(),
                phase_x,
                phase_y,
            };
            let (texture, size, offset) = cache.glyph(ui.ctx(), key, tone);
            let rect = Rect::from_min_size(
                Pos2::new(
                    (base_x + offset[0]) as f32 / dpi,
                    (base_y + offset[1]) as f32 / dpi,
                ),
                Vec2::new(size[0] as f32 / dpi, size[1] as f32 / dpi),
            );
            ui.painter().image(
                texture,
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                color,
            );
            x += face().h_advance_unscaled(face().glyph_id(c)) * part.size * scale
                - part.size * 0.018;
        }
    }
    Rect::from_center_size(
        center,
        Vec2::new(laid.width, laid.ink_bottom - laid.ink_top),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn with_ui(f: impl FnOnce(&egui::Ui)) {
        let context = egui::Context::default();
        super::super::install_fonts(&context);
        let mut callback = Some(f);
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900., 700.))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| callback.take().unwrap()(ui));
            },
        );
    }
    #[test]
    fn actual_tabular_glyphs_keep_decimal_and_baseline_still() {
        let ones = layout("11.11", 112.);
        let eights = layout("88.88", 112.);
        assert!((ones.width - eights.width).abs() < 0.01);
        assert!((ones.runs[1].x - eights.runs[1].x).abs() < 0.01);
        assert!((ones.ink_top - eights.ink_top).abs() < 0.01);
        assert!((ones.ink_bottom - eights.ink_bottom).abs() < 0.01);
        let widths = ('0'..='9')
            .map(|c| face().h_advance_unscaled(face().glyph_id(c)))
            .collect::<Vec<_>>();
        assert!(widths.iter().all(|w| (*w - widths[0]).abs() < 0.01));
    }
    #[test]
    fn numeric_minutes_and_dnf_hit_rectangles_keep_the_requested_center() {
        with_ui(|ui| {
            let center = Pos2::new(450., 350.);
            for text in [
                "0.00",
                "13.32",
                "1:11.11",
                "8:88.88",
                "123:45.67",
                "DNF",
                "—",
            ] {
                let bounds = paint_timer(ui, center, text, 112., Color32::WHITE);
                assert!((bounds.center() - center).length() < 0.01, "{text}");
                assert!(bounds.is_finite() && bounds.width() > 0. && bounds.height() > 0.);
            }
        });
    }
    #[test]
    fn coverage_is_smooth_and_preserves_area_across_pixel_phases_and_dpi() {
        for dpi in [1f32, 1.25, 1.5, 2.] {
            let key = GlyphKey {
                character: '3',
                size: 112f32.to_bits(),
                dpi: dpi.to_bits(),
                phase_x: 0,
                phase_y: 0,
            };
            let first = rasterize(key);
            let shifted = rasterize(GlyphKey {
                phase_x: 2,
                phase_y: 2,
                ..key
            });
            let area = first.coverage.iter().sum::<f32>();
            let other = shifted.coverage.iter().sum::<f32>();
            assert!((area - other).abs() / area < 0.001);
            assert!(
                first
                    .coverage
                    .iter()
                    .filter(|c| **c > 0.01 && **c < 0.99)
                    .count()
                    > 80
            );
            assert!(
                first
                    .coverage
                    .iter()
                    .all(|c| c.is_finite() && *c >= 0. && *c <= 1.00001)
            );
        }
    }
    #[test]
    fn gamma_accounts_for_theme_and_raw_glyphs_are_reused() {
        let white = Color32::WHITE;
        let black = Color32::BLACK;
        let a = coverage_alpha(0.5, white, black);
        let b = coverage_alpha(0.5, black, white);
        assert!((a - 0.735357).abs() < 0.0001);
        assert!((b - 0.264643).abs() < 0.0001);
        assert_eq!(coverage_alpha(0., white, black), 0.);
        assert_eq!(coverage_alpha(1., white, black), 1.);
        with_ui(|ui| {
            let mut cache = GlyphCache::default();
            let key = GlyphKey {
                character: '8',
                size: 96f32.to_bits(),
                dpi: 1f32.to_bits(),
                phase_x: 0,
                phase_y: 0,
            };
            let first = cache.glyph(ui.ctx(), key, [white, black]).0;
            assert_eq!(first, cache.glyph(ui.ctx(), key, [black, white]).0);
            assert_eq!(cache.rasterizations, 1);
            cache.glyph(
                ui.ctx(),
                GlyphKey {
                    dpi: 2f32.to_bits(),
                    ..key
                },
                [white, black],
            );
            assert_eq!(cache.rasterizations, 2);
            for n in 0..MAX_GLYPHS + 5 {
                cache.glyph(
                    ui.ctx(),
                    GlyphKey {
                        size: (20. + n as f32).to_bits(),
                        ..key
                    },
                    [white, black],
                );
            }
            assert_eq!(cache.entries.len(), MAX_GLYPHS);
        });
    }

    #[test]
    fn changing_hundredths_reuses_warmed_masks_and_one_baseline() {
        with_ui(|ui| {
            let draw_cycle = || {
                for hundredths in 0..100 {
                    paint_timer(
                        ui,
                        Pos2::new(450., 350.),
                        &format!("13.{hundredths:02}"),
                        112.,
                        Color32::WHITE,
                    );
                }
            };
            draw_cycle();
            let cache = ui.ctx().data_mut(|data| {
                data.get_temp::<Arc<Mutex<GlyphCache>>>(egui::Id::new("timer-coverage-cache"))
                    .unwrap()
            });
            let warmed = cache.lock().unwrap().rasterizations;
            assert!(warmed < MAX_GLYPHS);
            draw_cycle();
            let cache = cache.lock().unwrap();
            assert_eq!(cache.rasterizations, warmed);
            assert!(
                cache.entries.iter().all(|entry| {
                    entry.key.phase_y == cache.entries.front().unwrap().key.phase_y
                })
            );
        });
    }
}
