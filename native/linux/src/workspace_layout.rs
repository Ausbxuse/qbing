//! Responsive timer geometry in logical pixels; side rails require useful space.
use eframe::egui::{Pos2, Rect, Vec2};
pub struct Layout {
    pub center: Rect,
    pub rail: f32,
    pub clock: Rect,
    pub cube: Rect,
    pub scramble: Rect,
    pub phase: Rect,
    pub averages: Option<Rect>,
    pub compact: bool,
    pub dense: bool,
    pub collapsed: bool,
    pub actions: bool,
    pub clock_font: f32,
}
pub fn timer(r: Rect, mobile: bool) -> Layout {
    let rail = if !mobile && r.width() >= 1180. && r.height() >= 580. {
        (r.width() * 0.16).clamp(256., 272.)
    } else {
        0.
    };
    let center = Rect::from_min_max(r.min + Vec2::new(rail, 0.), r.max - Vec2::new(rail, 0.));
    let compact = center.width() < 600.;
    let dense = r.height() < 640.;
    let collapsed = r.height() < 400.;
    let actions = r.height() >= 300.;
    let phase_h = if collapsed {
        32.
    } else if dense {
        if compact { 112. } else { 84. }
    } else if compact {
        172.
    } else {
        130.
    };
    let phase = Rect::from_min_size(
        Pos2::new(
            center.center().x - (center.width() - 32.).min(704.) / 2.,
            r.bottom()
                - if dense {
                    8.
                } else if mobile {
                    4.
                } else {
                    24.
                }
                - phase_h,
        ),
        Vec2::new((center.width() - 32.).min(704.), phase_h),
    );
    let clock_h = if collapsed {
        56.
    } else if dense {
        64.
    } else if mobile {
        88.
    } else {
        100.
    };
    let cube_h = if collapsed {
        40.
    } else if dense {
        if compact {
            if r.height() >= 424. { 64. } else { 52. }
        } else if r.height() >= 460. {
            96.
        } else {
            64.
        }
    } else if mobile {
        128.
    } else if r.height() < 684. {
        104.
    } else {
        164.
    };
    let cube_w = if dense {
        cube_h * 1.25
    } else if mobile {
        160.
    } else if r.height() < 684. {
        144.
    } else {
        200.
    };
    let desired = r.top() + r.height() * if mobile { 0.457 } else { 0.47 };
    let scramble_h = if collapsed {
        44.
    } else if dense && compact {
        72.
    } else {
        60.
    };
    let scramble_top = if dense || r.height() < 684. || mobile {
        r.top() + if collapsed { 4. } else { 8. }
    } else {
        desired - 303.
    };
    let scramble = Rect::from_min_size(
        Pos2::new(
            center.center().x - (center.width() - 32.).min(620.) / 2.,
            scramble_top,
        ),
        Vec2::new((center.width() - 32.).min(620.), scramble_h),
    );
    let lower = scramble.bottom()
        + if dense { 8. } else { 12. }
        + cube_h
        + if dense { 12. } else { 20. }
        + clock_h / 2.;
    let upper = phase.top() - if actions { 76. } else { 34. } - clock_h / 2.;
    let cy = if dense {
        desired.clamp(lower.min(upper), upper)
    } else {
        desired
    };
    let clock = Rect::from_center_size(
        Pos2::new(center.center().x, cy),
        Vec2::new(center.width(), clock_h),
    );
    let cube_top = if dense {
        clock.top() - 12. - cube_h
    } else {
        cy - if mobile {
            186.
        } else if r.height() <= 684. {
            174.
        } else {
            235.
        }
    };
    let cube = Rect::from_min_size(
        Pos2::new(center.center().x - cube_w / 2., cube_top),
        Vec2::new(cube_w, cube_h),
    );
    let average_top = if dense {
        clock.bottom() + 80.
    } else {
        clock.top() + if mobile { 173. } else { 186. }
    };
    let average_h = if mobile { 70. } else { 74. };
    let average_w = if mobile { 320. } else { 336. };
    let averages = (average_top + average_h + 8. <= phase.top()).then(|| {
        Rect::from_min_size(
            Pos2::new(center.center().x - average_w / 2., average_top),
            Vec2::new(average_w, average_h),
        )
    });
    Layout {
        center,
        rail,
        clock,
        cube,
        scramble,
        phase,
        averages,
        compact,
        dense,
        collapsed,
        actions,
        clock_font: if collapsed {
            44.
        } else if dense {
            56.
        } else if mobile {
            64.
        } else {
            88.
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiled_windows_keep_the_timer_regions_separate() {
        for width in [
            360., 390., 450., 640., 760., 800., 960., 1080., 1180., 1280., 1440., 1920.,
        ] {
            for height in [400., 480., 540., 600., 720., 800., 1000., 1200.] {
                let mobile = width <= 760.;
                let header = if mobile { 88. } else { 56. };
                let footer = if mobile { 48. } else { 0. };
                let r =
                    Rect::from_min_max(Pos2::new(0., header), Pos2::new(width, height - footer));
                let l = timer(r, mobile);
                assert!(
                    l.cube.top() >= l.scramble.bottom(),
                    "cube/scramble {width}×{height}"
                );
                assert!(
                    l.clock.top() >= l.cube.bottom(),
                    "cube/clock {width}×{height}"
                );
                assert!(
                    l.clock.bottom() + if l.actions { 66. } else { 26. } <= l.phase.top(),
                    "clock/dock {width}×{height}"
                );
                assert!(l.phase.bottom() <= r.bottom());
                assert!(l.center.width() >= 360.);
                if let Some(a) = l.averages {
                    assert!(a.bottom() < l.phase.top());
                }
            }
        }
    }
}
