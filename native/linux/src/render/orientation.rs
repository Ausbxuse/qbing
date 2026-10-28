//! Stable gyro targets in the cube's 24 proper, axis-aligned orientations.
use super::mesh::{self, Quaternion};

const S: f32 = std::f32::consts::FRAC_1_SQRT_2;
const HYSTERESIS_RADIANS: f32 = 8. * std::f32::consts::PI / 180.;

// Identity; face-axis quarter/half turns; body-diagonal third turns; and
// edge-axis half turns. Antipodal quaternions represent the same pose.
const POSES: [Quaternion; 24] = [
    mesh::IDENTITY,
    [S, 0., 0., S],
    [-S, 0., 0., S],
    [0., S, 0., S],
    [0., -S, 0., S],
    [0., 0., S, S],
    [0., 0., -S, S],
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0.5, 0.5, 0.5, 0.5],
    [0.5, 0.5, -0.5, 0.5],
    [0.5, -0.5, 0.5, 0.5],
    [0.5, -0.5, -0.5, 0.5],
    [-0.5, 0.5, 0.5, 0.5],
    [-0.5, 0.5, -0.5, 0.5],
    [-0.5, -0.5, 0.5, 0.5],
    [-0.5, -0.5, -0.5, 0.5],
    [S, S, 0., 0.],
    [S, -S, 0., 0.],
    [S, 0., S, 0.],
    [S, 0., -S, 0.],
    [0., S, S, 0.],
    [0., S, -S, 0.],
];

fn similarity(a: Quaternion, b: Quaternion) -> f32 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| a * b)
        .sum::<f32>()
        .abs()
        .min(1.)
}

pub(super) struct Snapper {
    selected: Quaternion,
}

impl Default for Snapper {
    fn default() -> Self {
        Self {
            selected: mesh::IDENTITY,
        }
    }
}

impl Snapper {
    pub fn reset(&mut self) {
        self.selected = mesh::IDENTITY;
    }

    pub fn select(&mut self, relative: Quaternion) -> Quaternion {
        let Some(relative) = mesh::normalize(relative) else {
            self.reset();
            return self.selected;
        };
        let mut nearest = POSES[0];
        let mut nearest_similarity = similarity(relative, nearest);
        for pose in POSES.into_iter().skip(1) {
            let candidate = similarity(relative, pose);
            if candidate > nearest_similarity {
                nearest = pose;
                nearest_similarity = candidate;
            }
        }
        let current_error = 2. * similarity(relative, self.selected).acos();
        let nearest_error = 2. * nearest_similarity.acos();
        // Along a face-axis turn this retains the old pose through 49 degrees,
        // and the new one down to 41 degrees. Sensor noise near 45 cannot flicker.
        if current_error - nearest_error > HYSTERESIS_RADIANS {
            self.selected = nearest;
        }
        self.selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn y_rotation(degrees: f32) -> Quaternion {
        let half = degrees.to_radians() * 0.5;
        [0., half.sin(), 0., half.cos()]
    }

    fn same_pose(a: Quaternion, b: Quaternion) -> bool {
        similarity(a, b) > 1. - 0.000001
    }

    #[test]
    fn all_twenty_four_targets_are_distinct_proper_cube_orientations() {
        use std::collections::HashSet;
        let mut frames = HashSet::new();
        for pose in POSES {
            let axes = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]].map(|axis| {
                mesh::orient(axis, pose).map(|v| {
                    assert!((v - v.round()).abs() < 0.000001);
                    v.round() as i8
                })
            });
            let determinant = axes[0][0] * (axes[1][1] * axes[2][2] - axes[1][2] * axes[2][1])
                - axes[0][1] * (axes[1][0] * axes[2][2] - axes[1][2] * axes[2][0])
                + axes[0][2] * (axes[1][0] * axes[2][1] - axes[1][1] * axes[2][0]);
            assert_eq!(determinant, 1);
            assert!(frames.insert(axes), "Duplicate cube orientation");
            let mut snapper = Snapper::default();
            assert!(same_pose(snapper.select(pose), pose));
            // Slight hand tilt, scaled sensor quaternion, and antipodal sample
            // all keep the same target for every cube orientation.
            let tilted = mesh::product(pose, y_rotation(12.));
            assert!(same_pose(snapper.select(tilted.map(|v| -v * 1.001)), pose));
        }
        assert_eq!(frames.len(), 24);
    }

    #[test]
    fn forty_five_degree_boundary_has_eight_degree_error_hysteresis() {
        let mut snapper = Snapper::default();
        for angle in [44., 46., 45., 48.9, 44.5, 48.9] {
            assert!(same_pose(snapper.select(y_rotation(angle)), mesh::IDENTITY));
        }
        let quarter = y_rotation(90.);
        assert!(same_pose(snapper.select(y_rotation(49.1)), quarter));
        for angle in [48., 44., 46., 41.1, 45.] {
            assert!(same_pose(snapper.select(y_rotation(angle)), quarter));
        }
        assert!(same_pose(snapper.select(y_rotation(40.9)), mesh::IDENTITY));
    }

    #[test]
    fn reset_and_invalid_samples_clear_the_selected_target() {
        let mut snapper = Snapper::default();
        assert!(same_pose(snapper.select(y_rotation(90.)), y_rotation(90.)));
        snapper.reset();
        assert!(same_pose(snapper.select(y_rotation(46.)), mesh::IDENTITY));
        snapper.select(y_rotation(90.));
        assert!(same_pose(
            snapper.select([f32::NAN, 0., 0., 1.]),
            mesh::IDENTITY
        ));
        assert!(same_pose(snapper.select(y_rotation(46.)), mesh::IDENTITY));
    }
}

/// Observed whole-cube poses, independent of fixed-color turn events and TPS.
#[derive(Default)]
pub(crate) struct Recorder {
    reference: Option<Quaternion>,
    snap: Snapper,
    previous: Quaternion,
    live_frame: bool,
    initial: Option<Quaternion>,
}
impl Recorder {
    pub fn has_live_frame(&self) -> bool {
        self.live_frame
    }
    pub fn initial_pose(&self) -> Option<Quaternion> {
        self.initial
    }
    pub fn set_view_frame(
        &mut self,
        basis: Quaternion,
        pose: Quaternion,
        running: bool,
    ) -> Option<(String, Quaternion)> {
        let basis = mesh::normalize(basis)?;
        let pose = mesh::normalize(pose)?;
        if self.live_frame
            && self
                .reference
                .is_some_and(|old| similarity(old, basis) > 0.99999)
        {
            return None;
        }
        let was_live = self.live_frame;
        self.live_frame = true;
        self.reference = Some(basis);
        self.snap.selected = pose;
        if !was_live || !running {
            self.initial = Some(pose);
            self.previous = pose;
            return None;
        }
        self.changed_pose(pose)
    }
    fn changed_pose(&mut self, pose: Quaternion) -> Option<(String, Quaternion)> {
        if similarity(pose, self.previous) > 0.99999 {
            return None;
        }
        let notation = transition(self.previous, pose);
        self.previous = pose;
        Some((notation, pose))
    }
    pub fn observe(&mut self, q: Quaternion, running: bool) -> Option<(String, Quaternion)> {
        let q = mesh::normalize(q)?;
        if self.live_frame {
            let pose = self
                .snap
                .select(mesh::product(mesh::conjugate(self.reference?), q));
            if !running {
                self.initial = Some(pose);
                self.previous = pose;
                return None;
            }
            return self.changed_pose(pose);
        }
        if !running || self.reference.is_none() {
            self.reference = Some(q);
            self.snap.reset();
            self.previous = mesh::IDENTITY;
            return None;
        }
        let pose = self
            .snap
            .select(mesh::product(mesh::conjugate(self.reference?), q));
        if similarity(pose, self.previous) > 0.99999 {
            return None;
        }
        let notation = transition(self.previous, pose);
        self.previous = pose;
        Some((notation, pose))
    }
}
// Shortest intrinsic x/y/z description between two snapped poses. A skipped
// sensor interval can cross two axes; its decomposition is a pose delta, not
// a claim about the hand's unobserved continuous path.
fn transition(from: Quaternion, to: Quaternion) -> String {
    let generators = [
        ("x", [-S, 0., 0., S]),
        ("x'", [S, 0., 0., S]),
        ("x2", [1., 0., 0., 0.]),
        ("y", [0., -S, 0., S]),
        ("y'", [0., S, 0., S]),
        ("y2", [0., 1., 0., 0.]),
        ("z", [0., 0., -S, S]),
        ("z'", [0., 0., S, S]),
        ("z2", [0., 0., 1., 0.]),
    ];
    let mut queue = std::collections::VecDeque::from([(from, String::new())]);
    let mut seen = vec![from];
    while let Some((pose, path)) = queue.pop_front() {
        for (name, rotation) in generators {
            let next = mesh::product(pose, rotation);
            let text = if path.is_empty() {
                name.to_string()
            } else {
                format!("{path} {name}")
            };
            if similarity(next, to) > 0.99999 {
                return text;
            }
            if seen.iter().all(|q| similarity(*q, next) < 0.99999) {
                seen.push(next);
                queue.push_back((next, text));
            }
        }
    }
    unreachable!("proper cube orientations are connected")
}
#[cfg(test)]
mod recording_tests {
    use super::*;
    #[test]
    fn rotations_are_hysteretic_and_have_no_duplicate_or_antipodal_events() {
        let mut r = Recorder::default();
        assert!(r.observe(mesh::IDENTITY, false).is_none());
        assert!(
            r.observe(
                [
                    0.,
                    (48f32.to_radians() / 2.).sin(),
                    0.,
                    (48f32.to_radians() / 2.).cos()
                ],
                true
            )
            .is_none()
        );
        assert_eq!(r.observe([0., S, 0., S], true).unwrap().0, "y'");
        assert!(r.observe([0., -S, 0., -S], true).is_none());
        assert_eq!(r.observe(mesh::IDENTITY, true).unwrap().0, "y");
        assert!(r.observe([0., S, 0., S], false).is_none());
        assert!(r.observe([0., S, 0., S], true).is_none());
    }
    #[test]
    fn every_pose_pair_has_a_valid_shortest_delta() {
        for from in POSES {
            for to in POSES {
                if similarity(from, to) < 0.99999 {
                    assert!(!transition(from, to).is_empty());
                }
            }
        }
    }
}

/// Put the solved cross on the bottom without modifying the recorded facelets.
pub(crate) fn cross_pose(face: u8) -> Quaternion {
    match face {
        b'U' => [0., 0., 1., 0.],
        b'F' => [S, 0., 0., S],
        b'B' => [-S, 0., 0., S],
        b'R' => [0., 0., -S, S],
        b'L' => [0., 0., S, S],
        _ => mesh::IDENTITY,
    }
}
#[cfg(test)]
mod cross_frame_tests {
    use super::*;
    #[test]
    fn every_cross_face_points_down_in_replay() {
        for (face, axis) in [
            (b'U', [0., 1., 0.]),
            (b'D', [0., -1., 0.]),
            (b'R', [1., 0., 0.]),
            (b'L', [-1., 0., 0.]),
            (b'F', [0., 0., 1.]),
            (b'B', [0., 0., -1.]),
        ] {
            let p = mesh::orient(axis, cross_pose(face));
            assert!(p[0].abs() < 1e-5 && (p[1] + 1.).abs() < 1e-5 && p[2].abs() < 1e-5);
        }
    }
}

/// Absolute recorded pose at a playhead; never recalibrate against the first
/// replay sample or accumulate deltas across seeks. Equal timestamps use the
/// last observation, preserving recorder order.
pub(crate) fn initial_live_pose(record: &serde_json::Value) -> Option<Quaternion> {
    if record["rotationFrame"] != "live-v1" {
        return None;
    }
    let a = record["initialOrientation"]
        .as_array()
        .filter(|a| a.len() == 4)?;
    let mut q = [0.; 4];
    for (i, v) in a.iter().enumerate() {
        q[i] = v.as_f64()? as f32;
    }
    mesh::normalize(q)
}
pub(crate) fn replay_pose(record: &serde_json::Value, time: f64) -> Quaternion {
    let pose = recorded_pose(record, time);
    if initial_live_pose(record).is_some() {
        pose
    } else {
        mesh::product(
            cross_pose(crate::recording::reconstruction_cross(record)),
            pose,
        )
    }
}
pub(crate) fn recorded_pose(record: &serde_json::Value, time: f64) -> Quaternion {
    record["rotations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let t = r["t"]
                .as_f64()
                .filter(|t| t.is_finite() && *t >= 0. && *t <= time)?;
            let a = r["pose"].as_array().filter(|a| a.len() == 4)?;
            let mut q = [0.; 4];
            for (i, v) in a.iter().enumerate() {
                q[i] = v.as_f64()? as f32;
            }
            Some((t, mesh::normalize(q)?))
        })
        .fold(None, |last: Option<(f64, Quaternion)>, next| {
            if last.is_none_or(|last| next.0 >= last.0) {
                Some(next)
            } else {
                last
            }
        })
        .map(|(_, q)| q)
        .unwrap_or_else(|| initial_live_pose(record).unwrap_or(mesh::IDENTITY))
}

#[cfg(test)]
mod replay_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn recorded_pose_seeks_without_accumulation_and_ignores_invalid_samples() {
        let r = json!({"rotations":[
            {"t":200.,"pose":[0.,S,0.,S]},
            {"t":100.,"pose":[S,0.,0.,S]},
            {"t":200.,"pose":[0.,0.,S,S]},
            {"t":300.,"pose":[0.,0.,0.,0.]}
        ]});
        for _ in 0..3 {
            assert!(similarity(recorded_pose(&r, 200.), [0., 0., S, S]) > 0.99999);
            assert!(similarity(recorded_pose(&r, 150.), [S, 0., 0., S]) > 0.99999);
            assert_eq!(recorded_pose(&r, 0.), mesh::IDENTITY);
        }
        assert!(similarity(recorded_pose(&r, 400.), [0., 0., S, S]) > 0.99999);
        assert_eq!(recorded_pose(&json!({}), 400.), mesh::IDENTITY);
    }
}

#[cfg(test)]
mod live_replay_frame_tests {
    use super::super::Animator;
    use super::*;
    use serde_json::json;
    #[test]
    fn recorded_replay_matches_live_targets_for_every_hold_and_cross() {
        let basis = mesh::normalize([0.2, 0.3, 0.1, 0.7]).unwrap();
        let tilt = [
            (12f32.to_radians() / 2.).sin(),
            0.,
            0.,
            (12f32.to_radians() / 2.).cos(),
        ];
        for hold in POSES {
            let mut live = Animator::default();
            live.set_orientation(Some(basis));
            let held = mesh::product(basis, mesh::product(hold, tilt));
            live.set_orientation(Some(held));
            let (reference, initial) = live.live_view_frame().unwrap();
            let mut recorder = Recorder::default();
            recorder.set_view_frame(reference, initial, false);
            recorder.observe(held, false);
            assert!(similarity(recorder.initial_pose().unwrap(), initial) > 0.99999);
            let mut record = json!({"cross":"U","rotationFrame":"live-v1","initialOrientation":recorder.initial_pose(),"rotations":[]});
            let mut events = vec![];
            let mut pose = hold;
            for (i, turn) in [[0., -S, 0., S], [-S, 0., 0., S], [0., 0., -S, S]]
                .into_iter()
                .enumerate()
            {
                pose = mesh::product(pose, turn);
                let raw = mesh::product(reference, pose);
                live.set_orientation(Some(raw));
                let t = (i + 1) as f64 * 1000.;
                if let Some((notation, pose)) = recorder.observe(raw, true) {
                    events.push(json!({"notation":notation,"t":t,"pose":pose}));
                }
                record["rotations"] = json!(events);
                for cross in ["U", "D", "R", "L", "F", "B"] {
                    record["cross"] = json!(cross);
                    let mut replay = Animator::default();
                    replay.set_recorded_orientation(&record, t, false);
                    assert!(
                        similarity(replay.target_orientation, live.target_orientation) > 0.99999,
                        "hold {hold:?}, cross {cross}, step {i}"
                    );
                    replay.set_recorded_orientation(&record, 0., false);
                    assert!(similarity(replay.target_orientation, initial) > 0.99999);
                }
            }
        }
    }
    #[test]
    fn recenter_pose_is_recorded_without_changing_the_original_starting_pose() {
        let start = [0., S, 0., S];
        let mut r = Recorder::default();
        r.set_view_frame(mesh::IDENTITY, start, false);
        let (_, pose) = r.set_view_frame(start, mesh::IDENTITY, true).unwrap();
        assert!(similarity(pose, mesh::IDENTITY) > 0.99999);
        assert!(similarity(r.initial_pose().unwrap(), start) > 0.99999);
    }
}
