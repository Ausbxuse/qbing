mod gpu;
mod material;
mod mesh;
pub(crate) mod orientation;
mod smoke;

pub fn gpu_smoke() -> eframe::Result {
    smoke::run()
}

use crate::cube::{Cube, Move};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant},
};

pub fn face_color(face: u8) -> Color32 {
    match face {
        b'U' => Color32::from_rgb(245, 247, 252),
        b'D' => Color32::from_rgb(255, 218, 67),
        b'F' => Color32::from_rgb(40, 201, 146),
        b'B' => Color32::from_rgb(69, 153, 244),
        b'R' => Color32::from_rgb(244, 78, 101),
        _ => Color32::from_rgb(255, 152, 70),
    }
}

pub const HOME_YAW: f32 = -0.5980603;
pub const HOME_PITCH: f32 = 0.411252;

/// Register the native renderer while eframe's GL context is current. Headless
/// renders (and unsupported graphics drivers) use the same mesh with software depth.
pub fn install_glow(ctx: &egui::Context, gl: Arc<eframe::glow::Context>) {
    match gpu::Renderer::new(gl) {
        Ok(renderer) => ctx.data_mut(|d| d.insert_temp(egui::Id::new("cube.native.gl"), renderer)),
        Err(error) => eprintln!("Cube renderer uses software depth: {error}"),
    }
}
pub fn destroy_glow(ctx: &egui::Context) {
    if let Some(renderer) = ctx.data_mut(|d| {
        let id = egui::Id::new("cube.native.gl");
        let renderer = d.get_temp::<gpu::Renderer>(id);
        d.remove::<gpu::Renderer>(id);
        renderer
    }) {
        renderer.destroy();
    }
}

#[derive(Clone, Copy, Debug)]
struct TurnGroup {
    first: Move,
    second: Option<Move>,
    timestamp_ms: Option<f64>,
    partner_timestamp_ms: Option<f64>,
    partner_join: f32,
}
impl TurnGroup {
    fn new(first: Move, timestamp_ms: Option<f64>) -> Self {
        Self {
            first,
            second: None,
            timestamp_ms,
            partner_timestamp_ms: None,
            partner_join: 0.,
        }
    }
    fn same_axis_window(self, next: Self) -> bool {
        let close = |timestamp: Option<f64>| {
            timestamp
                .zip(next.timestamp_ms)
                .is_some_and(|(a, b)| (a - b).abs() <= 100.)
        };
        self.first.axis().0 == next.first.axis().0
            && close(self.timestamp_ms)
            && (self.second.is_none() || close(self.partner_timestamp_ms))
    }
    fn can_pair(self, next: Self) -> bool {
        let compatible_time = self
            .timestamp_ms
            .zip(next.timestamp_ms)
            .is_some_and(|(a, b)| (a - b).abs() <= 100.);
        let (axis, layer, _) = self.first.axis();
        let (other_axis, other_layer, _) = next.first.axis();
        self.second.is_none()
            && compatible_time
            && self.first.turns.abs() == 1
            && next.first.turns == -self.first.turns
            && axis == other_axis
            && layer == -other_layer
    }
    fn pair(&mut self, next: Self, join: f32) -> bool {
        if self.can_pair(next) {
            self.second = Some(next.first);
            self.partner_timestamp_ms = next.timestamp_ms;
            self.partner_join = join;
            true
        } else {
            false
        }
    }
    fn motion(self, fraction: f32) -> Motion {
        let ease = |t: f32| t * t * t * (t * (t * 6. - 15.) + 10.);
        let partner_fraction =
            ((fraction - self.partner_join) / (1. - self.partner_join)).clamp(0., 1.);
        Motion {
            first: self.first,
            second: self.second,
            progress: ease(fraction),
            partner_progress: ease(partner_fraction),
        }
    }
}

/// A renderer-only group. Both outer layers reach their endpoint together; it
/// never adds a synthetic slice turn or changes the gyro's independent frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Motion {
    first: Move,
    second: Option<Move>,
    progress: f32,
    partner_progress: f32,
}
impl From<(Move, f32)> for Motion {
    fn from((first, progress): (Move, f32)) -> Self {
        Self {
            first,
            second: None,
            progress,
            partner_progress: progress,
        }
    }
}

pub struct Animator {
    pub cube: Cube,
    mark_turns: i8,
    queue: VecDeque<TurnGroup>,
    active: Option<(TurnGroup, Instant, f32)>,
    pub yaw: f32,
    pub pitch: f32,
    physical: Option<mesh::Quaternion>,
    basis: Option<mesh::Quaternion>,
    gyro_snap: orientation::Snapper,
    orientation: mesh::Quaternion,
    target_orientation: mesh::Quaternion,
    orientation_time: Instant,
}
impl Default for Animator {
    fn default() -> Self {
        Self {
            cube: Cube::default(),
            mark_turns: 0,
            queue: VecDeque::new(),
            active: None,
            yaw: HOME_YAW,
            pitch: HOME_PITCH,
            physical: None,
            basis: None,
            gyro_snap: orientation::Snapper::default(),
            orientation: mesh::IDENTITY,
            target_orientation: mesh::IDENTITY,
            orientation_time: Instant::now(),
        }
    }
}
impl Animator {
    pub fn snap(&mut self, cube: Cube) {
        self.cube = cube;
        self.mark_turns = 0;
        self.queue.clear();
        self.active = None;
    }
    pub fn push(&mut self, mv: Move) {
        self.queue.push_back(TurnGroup::new(mv, None));
    }
    /// Pair trustworthy events within one contiguous, commuting same-axis block.
    /// Timestamp units are milliseconds; missing/recovered timing is a barrier.
    pub fn push_timed(&mut self, mv: Move, timestamp_ms: Option<f64>, recovered: bool) {
        self.push_timed_at(mv, timestamp_ms, recovered, Instant::now());
    }
    fn push_timed_at(
        &mut self,
        mv: Move,
        timestamp_ms: Option<f64>,
        recovered: bool,
        now: Instant,
    ) {
        if self.queue.len() >= 8 {
            let mut final_state = self.cube;
            if let Some((group, _, _)) = self.active {
                final_state = final_state.turn(group.first);
                if let Some(m) = group.second {
                    final_state = final_state.turn(m);
                }
            }
            for group in &self.queue {
                final_state = final_state.turn(group.first);
                if let Some(m) = group.second {
                    final_state = final_state.turn(m);
                }
            }
            self.snap(final_state.turn(mv));
            return;
        }
        let timestamp_ms = timestamp_ms.filter(|t| t.is_finite() && !recovered);
        let next = TurnGroup::new(mv, timestamp_ms);
        let mut candidate = None;
        let mut reaches_active = true;
        for (index, queued) in self.queue.iter().enumerate().rev() {
            if !queued.same_axis_window(next) {
                reaches_active = false;
                break;
            }
            if queued.can_pair(next) {
                candidate = Some(index);
            }
        }
        // Prefer the earliest still-unpaired turn. GAN may report a double
        // slice as R R L' L'; same-axis outer turns commute, so this only
        // changes presentation into two paired beats, never the raw end state.
        if reaches_active
            && let Some((active, started, duration)) = &mut self.active
            && now.saturating_duration_since(*started).as_secs_f32() < *duration
            && active.pair(
                next,
                now.saturating_duration_since(*started).as_secs_f32() / *duration,
            )
        {
            // Keep the first layer's original angle. The partner eases from zero
            // over the remaining duration, and both commit at the same endpoint.
            return;
        }
        if let Some(index) = candidate {
            self.queue[index].pair(next, 0.);
            return;
        }
        self.queue.push_back(next);
    }
    pub fn live_view_frame(&self) -> Option<([f32; 4], [f32; 4])> {
        self.basis.map(|basis| (basis, self.target_orientation))
    }
    pub fn set_orientation(&mut self, orientation: Option<[f32; 4]>) {
        self.physical = orientation.and_then(mesh::normalize);
        let target = if let Some(q) = self.physical {
            let basis = *self.basis.get_or_insert(q);
            // GAN reports body-to-world orientation. Express the current pose in
            // the initial cube frame before the home camera is applied. Putting
            // the inverse on the right instead makes axes depend on the hold at
            // connection/recentering (the two orders agree only at identity).
            self.gyro_snap
                .select(mesh::product(mesh::conjugate(basis), q))
        } else {
            self.basis = None;
            self.gyro_snap.reset();
            mesh::IDENTITY
        };
        if target != self.target_orientation {
            // A new discrete pose should animate briefly even after a long
            // stationary period; repeated samples must not restart the easing.
            self.orientation_time = Instant::now();
        }
        self.target_orientation = target;
    }
    pub fn set_replay_cross(&mut self, face: u8) {
        self.recenter_orientation();
        let pose = orientation::cross_pose(face);
        self.orientation = pose;
        self.target_orientation = pose;
    }
    pub fn set_recorded_orientation(
        &mut self,
        record: &serde_json::Value,
        time: f64,
        animate: bool,
    ) {
        let pose = orientation::replay_pose(record, time);
        if pose != self.target_orientation {
            self.orientation_time = Instant::now();
        }
        self.target_orientation = pose;
        if !animate {
            self.orientation = pose;
        }
    }
    pub fn recenter_orientation(&mut self) {
        self.basis = self.physical;
        self.gyro_snap.reset();
        self.orientation = mesh::IDENTITY;
        self.target_orientation = mesh::IDENTITY;
        self.orientation_time = Instant::now();
        self.yaw = HOME_YAW;
        self.pitch = HOME_PITCH;
    }
    fn recenter_view(&mut self) {
        // Recorded poses are authoritative. Double-click resets the orbit,
        // not the recorded body orientation at the current playhead.
        let recorded = self
            .physical
            .is_none()
            .then_some((self.orientation, self.target_orientation));
        self.recenter_orientation();
        if let Some((pose, target)) = recorded {
            self.orientation = pose;
            self.target_orientation = target;
        }
    }
    pub fn animating(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty() || self.orientation_error() > 1e-5
    }
    fn orientation_error(&self) -> f32 {
        1. - self
            .orientation
            .iter()
            .zip(self.target_orientation)
            .map(|(a, b)| a * b)
            .sum::<f32>()
            .abs()
            .min(1.)
    }
    fn advance_at(&mut self, now: Instant) -> Option<Motion> {
        let dt = now
            .saturating_duration_since(self.orientation_time)
            .as_secs_f32();
        self.orientation_time = now;
        self.orientation = mesh::slerp(
            self.orientation,
            self.target_orientation,
            1. - (-dt / 0.035).exp(),
        );
        if self.orientation_error() < 1e-6 {
            self.orientation = self.target_orientation;
        }
        let mut start = now;
        loop {
            if self.active.is_none()
                && let Some(group) = self.queue.pop_front()
            {
                let duration = (0.115 / (1. + self.queue.len() as f32 * 0.40)).max(0.022);
                self.active = Some((group, start, duration));
            }
            let Some((group, started, duration)) = self.active else {
                break;
            };
            if now.saturating_duration_since(started).as_secs_f32() < duration {
                break;
            }
            if group.first.face == b'U' {
                self.mark_turns = (self.mark_turns + group.first.turns).rem_euclid(4);
            }
            self.cube = self.cube.turn(group.first);
            if let Some(second) = group.second {
                if second.face == b'U' {
                    self.mark_turns = (self.mark_turns + second.turns).rem_euclid(4);
                }
                self.cube = self.cube.turn(second);
            }
            self.active = None;
            // Keep elapsed overshoot across turns after a slow frame. Starting at
            // `now` would accumulate lag and make rapid physical turns feel sticky.
            start = started + Duration::from_secs_f32(duration);
        }
        self.active.map(|(group, started, duration)| {
            let t = (now.saturating_duration_since(started).as_secs_f32() / duration).clamp(0., 1.);
            group.motion(t)
        })
    }
    pub fn draw(&mut self, ui: &mut egui::Ui, size: Vec2) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
        if response.dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            self.yaw += delta.x * 0.008;
            self.pitch = (self.pitch + delta.y * 0.008).clamp(-1.5, 1.5);
        }
        if response.double_clicked() {
            self.recenter_view();
        }
        let turn = self.advance_at(Instant::now());
        paint_oriented(
            ui.painter_at(rect),
            rect,
            self.cube,
            self.yaw,
            self.pitch,
            turn,
            self.orientation,
            self.mark_turns,
        );
        if self.animating() {
            ui.ctx().request_repaint();
        }
        response.on_hover_text(if self.physical.is_some() {
            "Drag to orbit. To align the gyro, hold white up and green toward you, then double-click."
        } else {
            "Drag to orbit · double-click to recenter"
        })
    }
}

pub fn paint_cube(
    p: egui::Painter,
    rect: Rect,
    cube: Cube,
    yaw: f32,
    pitch: f32,
    turn: Option<(Move, f32)>,
) {
    paint_oriented(
        p,
        rect,
        cube,
        yaw,
        pitch,
        turn.map(Motion::from),
        mesh::IDENTITY,
        0,
    );
}

/// Deterministic close-ups use the same closed mesh, lighting and perspective as
/// native GL, with a perspective-correct software depth buffer for headless tests.
pub fn cube_image(
    cube: Cube,
    yaw: f32,
    pitch: f32,
    turn: Option<(Move, f32)>,
    orientation: Option<[f32; 4]>,
    width: usize,
    height: usize,
) -> egui::ColorImage {
    mesh::rasterize(
        &mesh::scene(
            cube,
            yaw,
            pitch,
            turn,
            orientation
                .and_then(mesh::normalize)
                .unwrap_or(mesh::IDENTITY),
        ),
        width,
        height,
    )
}

#[derive(Clone, Default)]
struct CpuCache(VecDeque<(u64, egui::TextureHandle)>);
fn paint_oriented(
    p: egui::Painter,
    rect: Rect,
    cube: Cube,
    yaw: f32,
    pitch: f32,
    turn: Option<Motion>,
    orientation: mesh::Quaternion,
    mark_turns: i8,
) {
    // The surrounding layout may be wide (replay, previews), but the object is
    // square-framed. Keep GPU allocation proportional to its visible footprint.

    if !p.is_visible() || rect.width() < 1. || rect.height() < 1. {
        return;
    }
    let renderer = p
        .ctx()
        .data(|d| d.get_temp::<gpu::Renderer>(egui::Id::new("cube.native.gl")));
    if let Some(renderer) = renderer {
        p.add(renderer.callback(
            rect,
            gpu::Snapshot::new(cube, yaw, pitch, turn, orientation, mark_turns),
        ));
    } else {
        let pixels = p.ctx().pixels_per_point() * 2.;
        let width = (rect.width() * pixels).ceil().clamp(1., 2048.) as usize;
        let height = (rect.height() * pixels).ceil().clamp(1., 2048.) as usize;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (
            cube.0,
            yaw.to_bits(),
            pitch.to_bits(),
            turn.map(|m| {
                (
                    m.first.face,
                    m.first.turns,
                    m.second.map(|n| (n.face, n.turns)),
                    m.progress.to_bits(),
                    m.partner_progress.to_bits(),
                )
            }),
            orientation.map(f32::to_bits),
            width,
            height,
        )
            .hash(&mut hasher);
        let signature = hasher.finish();
        let id = egui::Id::new("cube.native.cpu-textures");
        let cached = p.ctx().data_mut(|d| {
            let cache = d.get_temp_mut_or_default::<CpuCache>(id);
            cache
                .0
                .iter()
                .find(|(key, _)| *key == signature)
                .map(|(_, texture)| texture.clone())
        });
        let texture = cached.unwrap_or_else(|| {
            let image = mesh::rasterize(
                &mesh::scene_motion(cube, yaw, pitch, turn, orientation),
                width,
                height,
            );
            let texture = p
                .ctx()
                .load_texture("cube.depth", image, egui::TextureOptions::LINEAR);
            p.ctx().data_mut(|d| {
                let cache = d.get_temp_mut_or_default::<CpuCache>(id);
                cache.0.push_back((signature, texture.clone()));
                while cache.0.len() > 32 {
                    cache.0.pop_front();
                }
            });
            texture
        });
        p.image(
            texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
            Color32::WHITE,
        );
    }
}

#[cfg(test)]
mod animation_tests {
    use super::*;

    fn assert_direction(actual: mesh::V3, expected: mesh::V3) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 0.00001,
                "{actual} != {expected}"
            );
        }
    }

    #[test]
    fn paired_outer_turns_share_progress_and_commit_raw_state_in_either_order() {
        for notation in [
            "R L'", "L' R", "U D'", "D' U", "F B'", "B' F", "R' L", "U' D", "F' B",
        ] {
            let moves = crate::cube::sequence(notation).unwrap();
            let mut animator = Animator::default();
            let start = Instant::now();
            animator.push_timed_at(moves[0], Some(500.), false, start);
            animator.push_timed_at(moves[1], Some(560.), false, start);
            assert_eq!(animator.queue.len(), 1);
            animator.advance_at(start);
            let motion = animator
                .advance_at(start + Duration::from_millis(50))
                .unwrap();
            assert_eq!(motion.first, moves[0]);
            assert_eq!(motion.second, Some(moves[1]));
            assert_eq!(motion.progress, motion.partner_progress);
            assert!(motion.progress > 0. && motion.progress < 1.);
            assert_eq!(animator.cube, Cube::default());
            animator.advance_at(start + Duration::from_secs(1));
            assert_eq!(animator.cube, Cube::default().apply(&moves));
            assert!(!animator.animating());
        }
    }

    #[test]
    fn delayed_partner_never_rewinds_first_layer_or_jumps_second_layer() {
        let moves = crate::cube::sequence("R L'").unwrap();
        let mut animator = Animator::default();
        let start = Instant::now();
        animator.push_timed_at(moves[0], Some(0.), false, start);
        animator.advance_at(start);
        let joined_at = start + Duration::from_millis(65);
        let before = animator.advance_at(joined_at).unwrap();
        animator.push_timed_at(moves[1], Some(65.), false, joined_at);
        let after = animator.advance_at(joined_at).unwrap();
        assert_eq!(after.progress, before.progress);
        assert_eq!(after.partner_progress, 0.);
        assert_eq!(after.second, Some(moves[1]));
        assert!(animator.queue.is_empty());
        let later = animator
            .advance_at(start + Duration::from_millis(95))
            .unwrap();
        assert!(later.progress >= after.progress);
        assert!(later.partner_progress > 0. && later.partner_progress < 1.);
        assert_eq!(animator.cube, Cube::default());
        animator.advance_at(start + Duration::from_millis(116));
        assert_eq!(animator.cube, Cube::default().apply(&moves));
        assert!(!animator.animating());
    }

    #[test]
    fn noninterleaved_double_slices_form_two_coherent_paired_beats() {
        for notation in ["R R L' L'", "L' L' R R", "U U D' D'", "F F B' B'"] {
            let moves = crate::cube::sequence(notation).unwrap();
            let mut animator = Animator::default();
            let start = Instant::now();
            for (i, mv) in moves.iter().copied().enumerate() {
                animator.push_timed_at(mv, Some(i as f64 * 20.), false, start);
            }
            assert_eq!(animator.queue.len(), 2, "{notation}");
            assert!(animator.queue.iter().all(|group| group.second.is_some()));
            animator.advance_at(start);
            let halfway = animator
                .advance_at(start + Duration::from_millis(40))
                .unwrap();
            assert_eq!(halfway.progress, halfway.partner_progress);
            animator.advance_at(start + Duration::from_secs(1));
            assert_eq!(animator.cube, Cube::default().apply(&moves));
            assert!(!animator.animating());
        }
    }

    #[test]
    fn double_slice_partner_can_join_active_across_queued_same_axis_continuation() {
        let moves = crate::cube::sequence("R R L' L'").unwrap();
        let mut animator = Animator::default();
        let start = Instant::now();
        animator.push_timed_at(moves[0], Some(0.), false, start);
        animator.advance_at(start);
        animator.push_timed_at(
            moves[1],
            Some(20.),
            false,
            start + Duration::from_millis(20),
        );
        let joined_at = start + Duration::from_millis(50);
        let before = animator.advance_at(joined_at).unwrap();
        animator.push_timed_at(moves[2], Some(50.), false, joined_at);
        let after = animator.advance_at(joined_at).unwrap();
        assert_eq!(after.progress, before.progress);
        assert_eq!(after.partner_progress, 0.);
        assert_eq!(after.second, Some(moves[2]));
        assert_eq!(animator.queue.len(), 1);
        animator.push_timed_at(
            moves[3],
            Some(80.),
            false,
            start + Duration::from_millis(80),
        );
        assert_eq!(animator.queue.len(), 1);
        assert_eq!(animator.queue[0].second, Some(moves[3]));
        animator.advance_at(start + Duration::from_secs(1));
        assert_eq!(animator.cube, Cube::default().apply(&moves));
        assert!(!animator.animating());
    }

    #[test]
    fn commuting_search_never_crosses_an_axis_timing_or_recovery_barrier() {
        for active in [false, true] {
            for (middle, timestamp, recovered) in [
                ("U", Some(20.), false),
                ("R", None, false),
                ("R", Some(20.), true),
                ("R", Some(1000.), false),
            ] {
                let mut animator = Animator::default();
                let start = Instant::now();
                animator.push_timed_at(Move::parse("R").unwrap(), Some(0.), false, start);
                if active {
                    animator.advance_at(start);
                }
                animator.push_timed_at(Move::parse(middle).unwrap(), timestamp, recovered, start);
                animator.push_timed_at(
                    Move::parse("L'").unwrap(),
                    Some(50.),
                    false,
                    start + Duration::from_millis(50),
                );
                assert!(animator.queue.iter().all(|group| group.second.is_none()));
                if active {
                    assert!(animator.active.unwrap().0.second.is_none());
                }
                animator.advance_at(start);
                animator.advance_at(start + Duration::from_secs(1));
                let expected = Cube::default()
                    .apply(&crate::cube::sequence(&format!("R {middle} L'")).unwrap());
                assert_eq!(animator.cube, expected);
            }
        }
    }

    #[test]
    fn pairing_requires_opposite_quarters_and_trusted_close_timestamps() {
        for (notation, first_time, second_time, first_recovered, second_recovered) in [
            ("R L'", Some(0.), Some(100.01), false, false),
            ("R L'", None, Some(20.), false, false),
            ("R L'", Some(0.), None, false, false),
            ("R L'", Some(f64::NAN), Some(20.), false, false),
            ("R L'", Some(0.), Some(20.), true, false),
            ("R L'", Some(0.), Some(20.), false, true),
            ("R U'", Some(0.), Some(20.), false, false),
            ("R L", Some(0.), Some(20.), false, false),
            ("R2 L2", Some(0.), Some(20.), false, false),
            ("R R'", Some(0.), Some(20.), false, false),
        ] {
            let moves = crate::cube::sequence(notation).unwrap();
            let mut animator = Animator::default();
            let start = Instant::now();
            animator.push_timed_at(moves[0], first_time, first_recovered, start);
            animator.push_timed_at(moves[1], second_time, second_recovered, start);
            assert_eq!(animator.queue.len(), 2, "Unexpected pairing: {notation}");
        }
        let mut animator = Animator::default();
        let moves = crate::cube::sequence("R U L'").unwrap();
        let start = Instant::now();
        for (i, mv) in moves.into_iter().enumerate() {
            animator.push_timed_at(mv, Some(i as f64 * 10.), false, start);
        }
        assert_eq!(
            animator.queue.len(),
            3,
            "Pairing skipped an intervening turn"
        );
        animator.snap(Cube::default());
        for mv in crate::cube::sequence("R L'").unwrap() {
            animator.push(mv);
        }
        assert_eq!(
            animator.queue.len(),
            2,
            "Untimed pushes must stay sequential"
        );
    }

    #[test]
    fn a_completed_first_layer_is_never_reopened_for_late_delivery() {
        let moves = crate::cube::sequence("R L'").unwrap();
        let mut animator = Animator::default();
        let start = Instant::now();
        animator.push_timed_at(moves[0], Some(0.), false, start);
        animator.advance_at(start);
        animator.push_timed_at(
            moves[1],
            Some(50.),
            false,
            start + Duration::from_millis(120),
        );
        assert_eq!(animator.queue.len(), 1);
        assert!(animator.active.unwrap().0.second.is_none());
        animator.advance_at(start + Duration::from_secs(1));
        assert_eq!(animator.cube, Cube::default().apply(&moves));
    }

    #[test]
    fn paired_groups_preserve_overshoot_and_do_not_inject_gyro_rotation() {
        let mut animator = Animator::default();
        let s = std::f32::consts::FRAC_1_SQRT_2;
        animator.set_orientation(Some([s, 0., 0., s]));
        animator.set_orientation(Some([0.5, 0.5, 0.5, 0.5]));
        let physical = animator.physical;
        let basis = animator.basis;
        let target = animator.target_orientation;
        let moves = crate::cube::sequence("R L' R L' U D' F B'").unwrap();
        let start = Instant::now();
        for (i, mv) in moves.iter().copied().enumerate() {
            animator.push_timed_at(mv, Some(i as f64 * 20.), false, start);
        }
        assert_eq!(animator.queue.len(), 4);
        animator.advance_at(start);
        animator.advance_at(start + Duration::from_secs(1));
        assert_eq!(animator.cube, Cube::default().apply(&moves));
        assert_eq!(animator.physical, physical);
        assert_eq!(animator.basis, basis);
        assert_eq!(animator.target_orientation, target);
        assert!(!animator.animating());
    }

    #[test]
    fn a_delayed_frame_consumes_turn_overshoot() {
        let mut animator = Animator::default();
        let moves = crate::cube::sequence("R U R' U'").unwrap();
        for m in &moves {
            animator.push(*m);
        }
        let start = Instant::now();
        animator.advance_at(start);
        animator.advance_at(start + Duration::from_secs(1));
        assert_eq!(animator.cube, Cube::default().apply(&moves));
        assert!(!animator.animating());
    }
    #[test]
    fn gyro_tracks_local_axes_from_a_nonidentity_starting_hold() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        // Initial body-to-world pose: +90 degrees around Y. Each following
        // quaternion is that pose followed by a +90 degree local X, Y, or Z turn.
        // These fixed vector oracles catch reversed reference multiplication.
        for (current, source, expected) in [
            ([0.5, 0.5, -0.5, 0.5], [0., 1., 0.], [0., 0., 1.]),
            ([0., 1., 0., 0.], [0., 0., 1.], [1., 0., 0.]),
            ([0.5, 0.5, 0.5, 0.5], [1., 0., 0.], [0., 1., 0.]),
        ] {
            let mut animator = Animator::default();
            animator.set_orientation(Some([0., s, 0., s]));
            animator.set_orientation(Some(current));
            animator.advance_at(Instant::now() + Duration::from_secs(1));
            assert_direction(mesh::orient(source, animator.orientation), expected);
            assert_eq!(animator.cube, Cube::default());
        }
    }

    #[test]
    fn gyro_recenters_a_rotated_pose_and_accepts_antipodal_samples() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let mut animator = Animator::default();
        animator.set_orientation(Some([0., s, 0., s]));
        animator.set_orientation(Some([0.5, 0.5, -0.5, 0.5]));
        animator.yaw = 1.2;
        animator.pitch = -0.4;
        animator.recenter_orientation();
        assert_eq!(animator.orientation, mesh::IDENTITY);
        assert_eq!((animator.yaw, animator.pitch), (HOME_YAW, HOME_PITCH));
        // The new reference followed by local +90 degrees around Y is
        // [0, s, 0, s] * X90 * Y90 = [s, s, 0, 0]. Use its antipode,
        // which represents exactly the same pose, as real sensors may do.
        animator.set_orientation(Some([-s, -s, 0., 0.]));
        animator.advance_at(Instant::now() + Duration::from_secs(1));
        assert_direction(
            mesh::orient([0., 0., 1.], animator.orientation),
            [1., 0., 0.],
        );
    }

    #[test]
    fn gyro_snap_transition_eases_once_without_tracking_hand_tilt() {
        let mut animator = Animator::default();
        animator.set_orientation(Some(mesh::IDENTITY));
        animator.orientation_time = Instant::now() - Duration::from_secs(30);
        let half = 60f32.to_radians() * 0.5;
        animator.set_orientation(Some([0., half.sin(), 0., half.cos()]));
        let start = animator.orientation_time;
        assert!(animator.animating());
        animator.advance_at(start + Duration::from_millis(10));
        let partially_turned = mesh::orient([0., 0., 1.], animator.orientation);
        assert!(partially_turned[0] > 0.1 && partially_turned[0] < 0.9);
        assert!(partially_turned[2] > 0.1 && partially_turned[2] < 1.);
        // New raw samples within this target's cell must neither change its
        // exact quarter-turn target nor restart the in-flight transition.
        let last_frame = animator.orientation_time;
        let half = 75f32.to_radians() * 0.5;
        animator.set_orientation(Some([0., half.sin(), 0., half.cos()]));
        assert_eq!(animator.orientation_time, last_frame);
        animator.advance_at(start + Duration::from_secs(1));
        assert_direction(
            mesh::orient([0., 0., 1.], animator.orientation),
            [1., 0., 0.],
        );
        assert!(!animator.animating());
    }

    #[test]
    fn missing_gyro_data_clears_calibration_and_snap_without_clearing_moves() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let mut animator = Animator::default();
        animator.set_orientation(Some(mesh::IDENTITY));
        animator.set_orientation(Some([0., s, 0., s]));
        animator.advance_at(Instant::now() + Duration::from_secs(1));
        let mv = Move::parse("R").unwrap();
        animator.push(mv);
        animator.set_orientation(None);
        assert!(animator.basis.is_none());
        assert!(animator.physical.is_none());
        assert_eq!(animator.target_orientation, mesh::IDENTITY);
        let start = animator.orientation_time;
        animator.advance_at(start);
        animator.advance_at(start + Duration::from_secs(1));
        assert_eq!(animator.orientation, mesh::IDENTITY);
        assert_eq!(animator.cube, Cube::default().turn(mv));
        // The first pose after reconnect becomes the new calibration. A 46°
        // local turn must remain at identity, proving old selection was cleared.
        animator.set_orientation(Some([s, 0., 0., s]));
        let half = 46f32.to_radians() * 0.5;
        animator.set_orientation(Some(mesh::product(
            [s, 0., 0., s],
            [0., half.sin(), 0., half.cos()],
        )));
        assert_eq!(animator.target_orientation, mesh::IDENTITY);
    }

    #[test]
    fn recenter_clears_snap_hysteresis_at_the_current_nonidentity_hold() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let mut animator = Animator::default();
        animator.set_orientation(Some([s, 0., 0., s]));
        let current = [0.5, 0.5, 0.5, 0.5];
        animator.set_orientation(Some(current));
        animator.advance_at(Instant::now() + Duration::from_secs(1));
        animator.recenter_orientation();
        let half = 46f32.to_radians() * 0.5;
        animator.set_orientation(Some(mesh::product(
            current,
            [0., half.sin(), 0., half.cos()],
        )));
        assert_eq!(animator.target_orientation, mesh::IDENTITY);
        assert_eq!(animator.orientation, mesh::IDENTITY);
        assert_eq!(animator.cube, Cube::default());
    }

    #[test]
    fn replay_view_reset_keeps_the_recorded_pose() {
        let mut replay = Animator::default();
        let record = serde_json::json!({"rotationFrame":"live-v1","initialOrientation":[0.,1.,0.,0.],"rotations":[]});
        replay.set_recorded_orientation(&record, 0., false);
        replay.yaw = 1.2;
        replay.pitch = 0.8;
        replay.recenter_view();
        assert_eq!(replay.orientation, [0., 1., 0., 0.]);
        assert_eq!(replay.target_orientation, [0., 1., 0., 0.]);
        assert_eq!(replay.yaw, HOME_YAW);
        assert_eq!(replay.pitch, HOME_PITCH);
    }
    #[test]
    fn gyro_and_queued_face_turns_keep_separate_coordinate_frames() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let mut animator = Animator::default();
        animator.set_orientation(Some([0., s, 0., s]));
        let moves = crate::cube::sequence("R U F' R2").unwrap();
        for mv in &moves {
            animator.push(*mv);
        }
        let start = Instant::now();
        animator.advance_at(start);
        // A face turn is still in flight while the body rotates around local X.
        animator.set_orientation(Some([0.5, 0.5, -0.5, 0.5]));
        animator.advance_at(start + Duration::from_millis(10));
        assert_eq!(animator.cube, Cube::default());
        animator.advance_at(start + Duration::from_secs(1));
        assert_eq!(animator.cube, Cube::default().apply(&moves));
        assert_direction(
            mesh::orient([0., 1., 0.], animator.orientation),
            [0., 0., 1.],
        );
        animator.recenter_orientation();
        assert_eq!(animator.orientation, mesh::IDENTITY);
        assert_eq!(animator.cube, Cube::default().apply(&moves));
    }
}

pub fn chart(ui: &mut egui::Ui, values: &[f64], height: f32) {
    let colors = crate::design::palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let p = ui.painter_at(rect);
    let valid = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .collect::<Vec<_>>();
    if valid.is_empty() {
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Your trend appears after the first attempt",
            egui::FontId::proportional(13.),
            colors.muted,
        );
        return;
    }
    let low = valid.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min) * 0.85;
    let high = (valid.iter().map(|(_, v)| *v).fold(0., f64::max) * 1.1).max(low + 500.);
    for n in 0..3 {
        let y = rect.top() + 12. + (height - 30.) * n as f32 / 2.;
        p.line_segment(
            [Pos2::new(rect.left() + 38., y), Pos2::new(rect.right(), y)],
            Stroke::new(0.6f32, colors.line),
        );
        let value = high - (high - low) * n as f64 / 2.;
        p.text(
            Pos2::new(rect.left(), y),
            egui::Align2::LEFT_CENTER,
            format!("{:0.1}", value / 1000.),
            egui::FontId::proportional(11.),
            colors.muted,
        );
    }
    let points = valid
        .iter()
        .map(|(i, v)| {
            Pos2::new(
                rect.left()
                    + 40.
                    + *i as f32 / (values.len().saturating_sub(1).max(1)) as f32
                        * (rect.width() - 44.),
                rect.top() + 12. + (1. - ((v - low) / (high - low)) as f32) * (height - 30.),
            )
        })
        .collect::<Vec<_>>();
    for pair in points.windows(2) {
        p.line_segment([pair[0], pair[1]], Stroke::new(1.8f32, colors.accent));
    }
    if let Some(cursor) = response.hover_pos()
        && let Some((index, point)) = points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (a.x - cursor.x).abs().total_cmp(&(b.x - cursor.x).abs()))
    {
        p.line_segment(
            [
                Pos2::new(point.x, rect.top() + 4.),
                Pos2::new(point.x, rect.bottom() - 5.),
            ],
            Stroke::new(1.0f32, colors.muted.gamma_multiply(0.4)),
        );
        p.circle_filled(*point, 4., colors.accent);
        response.on_hover_text(format!(
            "Attempt {} · {} s",
            valid[index].0 + 1,
            crate::io::seconds(valid[index].1)
        ));
    }
    for point in points {
        p.circle_filled(point, 2.8, colors.accent);
    }
}

pub fn native_profile() -> serde_json::Value {
    gpu::profile()
}
