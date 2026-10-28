use crate::{
    cube::{Cube, Move},
    io::number,
    recording::count_moves,
};
use serde_json::{Value, json};
use std::time::Instant;

#[derive(Default)]
pub struct Tracker {
    first: Option<Instant>,
    clock: Option<f64>,
    live_clock_offset: f64,
    frozen: Option<Value>,
    pub moves: Vec<Value>,
    pub quality: Vec<String>,
}
impl Tracker {
    pub fn observe(
        &mut self,
        before: Option<Cube>,
        after: Cube,
        mv: Move,
        clock: Option<f64>,
        recovered: bool,
        at: Instant,
    ) {
        if before == Some(Cube::default()) {
            *self = Self {
                first: Some(at),
                clock,
                ..Default::default()
            };
        }
        self.frozen = None;
        let Some(first) = self.first else { return };
        if self.clock.is_none() && !recovered {
            self.clock = crate::timing::estimated_origin(&self.moves, clock);
        }
        let local = at.saturating_duration_since(first).as_secs_f64() * 1000.;
        let mut t = clock
            .zip(self.clock)
            .map(|(c, s)| c - s)
            .unwrap_or_else(|| {
                if recovered && clock.is_none() {
                    crate::timing::estimated_recovered_elapsed(&self.moves)
                } else {
                    local
                }
            });
        let previous_clock = self
            .moves
            .iter()
            .rev()
            .find_map(|event| event["raw"]["cubeTimestamp"].as_f64());
        if t < 0.
            || clock
                .zip(previous_clock)
                .is_some_and(|(current, previous)| current < previous)
        {
            if !self.quality.iter().any(|flag| flag == "clock-reset") {
                self.quality.push("clock-reset".into());
            }
            t = local;
        }
        if recovered && !self.quality.iter().any(|q| q == "estimated-timing") {
            self.quality.push("estimated-timing".into());
        }
        if !recovered && !self.quality.iter().any(|q| q == "clock-reset") {
            crate::timing::settle_recovered(&mut self.moves, self.clock, clock);
            if clock.is_some() && self.clock.is_some() {
                self.live_clock_offset = self.live_clock_offset.max(t - local);
            }
        }
        t = t.max(self.moves.last().map(|m| number(m, "t")).unwrap_or(0.));
        self.moves.push(json!({"move":mv.to_string(),"t":t,"recovered":recovered,"raw":{"cubeTimestamp":clock,"localTimestamp":local}}));
        if after == Cube::default() {
            *self = Self::default();
        }
    }
    pub fn freeze(&mut self) {
        if self.frozen.is_none() {
            self.frozen = Some(stats(&self.moves, &self.quality));
        }
    }
    pub fn stats(&self) -> Value {
        self.frozen
            .clone()
            .unwrap_or_else(|| stats(&self.moves, &self.quality))
    }
    pub fn live_stats(&self) -> Value {
        self.live_stats_at(Instant::now())
    }
    fn live_stats_at(&self, at: Instant) -> Value {
        if let Some(result) = &self.frozen {
            return result.clone();
        }
        let mut result = self.stats();
        let Some(first) = self.first else {
            return result;
        };
        let duration = (at.saturating_duration_since(first).as_secs_f64() * 1000.
            + self.live_clock_offset)
            .max(number(&result, "duration"));
        let usable = self.quality.iter().all(|flag| flag == "estimated-timing");
        result["duration"] = json!(duration);
        result["tps"] = json!(
            (usable && duration > 0.).then(|| count_moves(&self.moves) as f64 * 1000. / duration)
        );
        result["estimated"] = json!(
            self.clock.is_none()
                || self
                    .moves
                    .first()
                    .is_some_and(|event| event["recovered"] == true)
        );
        result
    }
}
pub fn stats(moves: &[Value], quality: &[String]) -> Value {
    let duration = moves.last().map(|m| number(m, "t")).unwrap_or(0.);
    let count = count_moves(moves);
    let metric = crate::recording::execution_metrics(
        &json!({"source":"bluetooth","status":"complete","duration":duration,"moves":moves,"quality":quality}),
    );
    json!({"duration":duration,"moves":count,"rawMoves":moves.len(),"tps":metric.map(|m|m.tps),"estimated":metric.is_some_and(|m|m.estimated)})
}

/// Reduce only adjacent turns of the same face; this preserves the exact state.
pub fn simplify(moves: impl IntoIterator<Item = Move>) -> Vec<Move> {
    let mut out: Vec<Move> = vec![];
    for mv in moves {
        if out.last().is_some_and(|last| last.face == mv.face) {
            let previous = out.pop().unwrap();
            let n = (previous.turns + mv.turns).rem_euclid(4);
            if n != 0 {
                out.push(Move {
                    face: mv.face,
                    turns: if n == 3 { -1 } else { n },
                });
            }
        } else {
            out.push(mv);
        }
    }
    out
}

/// Undo only the mistaken turns, returning to the exact retained checkpoint.
/// A partly completed double stays on its original notation token.
pub fn correction(
    before: Option<Cube>,
    after: Cube,
    mv: Move,
    setup: &[Move],
    current: &[Move],
) -> Option<Vec<Move>> {
    use crate::cube::scramble_progress;
    if scramble_progress(after, setup).matched {
        return Some(vec![]);
    }
    let before = before?;
    if !current.is_empty() {
        return Some(simplify(
            std::iter::once(mv.inverse()).chain(current.iter().copied()),
        ));
    }
    let progress = scramble_progress(before, setup);
    if !progress.matched {
        return None;
    }
    Some(vec![mv.inverse()])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_scramble_rate_updates_during_pauses_without_changing_saved_duration() {
        let mut tracker = Tracker::default();
        let at = Instant::now();
        let r = Move::parse("R").unwrap();
        let u = Move::parse("U").unwrap();
        let first = Cube::default().turn(r);
        tracker.observe(Some(Cube::default()), first, r, Some(1000.), false, at);
        tracker.observe(
            Some(first),
            first.turn(u),
            u,
            Some(1400.),
            false,
            at + std::time::Duration::from_millis(400),
        );
        assert_eq!(
            tracker.live_stats_at(at + std::time::Duration::from_millis(500))["tps"],
            4.
        );
        assert_eq!(
            tracker.live_stats_at(at + std::time::Duration::from_millis(1000))["tps"],
            2.
        );
        assert_eq!(tracker.stats()["duration"], 400.);
        assert_eq!(tracker.stats()["tps"], 5.);
    }
    #[test]
    fn scramble_recovery_preserves_real_endpoint_timing() {
        let mut tracker = Tracker::default();
        let at = Instant::now();
        let mut state = Cube::default();
        for (name, clock, received, recovered) in [
            ("R", Some(1000.), 0, false),
            ("U", Some(1200.), 200, false),
            ("F", None, 550, true),
            ("B", Some(1400.), 560, false),
        ] {
            let mv = Move::parse(name).unwrap();
            let next = state.turn(mv);
            tracker.observe(
                Some(state),
                next,
                mv,
                clock,
                recovered,
                at + std::time::Duration::from_millis(received),
            );
            state = next;
        }
        assert_eq!(tracker.moves[2]["t"], 300.);
        assert_eq!(tracker.moves[2]["raw"]["localTimestamp"], 550.);
        assert_eq!(tracker.stats()["duration"], 400.);
        assert_eq!(tracker.stats()["tps"], 10.);
        assert_eq!(tracker.stats()["estimated"], false);
        assert_eq!(tracker.quality, ["estimated-timing"]);
    }
    #[test]
    fn recovered_scramble_endpoint_excludes_history_delivery_latency() {
        for delay in [100, 1000] {
            let mut tracker = Tracker::default();
            let at = Instant::now();
            let mut state = Cube::default();
            for (name, clock, received, recovered) in [
                ("R", Some(1000.), 0, false),
                ("U", Some(1200.), 200, false),
                ("F", None, 200 + delay, true),
                ("B", None, 200 + delay, true),
            ] {
                let mv = Move::parse(name).unwrap();
                let next = state.turn(mv);
                tracker.observe(
                    Some(state),
                    next,
                    mv,
                    clock,
                    recovered,
                    at + std::time::Duration::from_millis(received),
                );
                state = next;
            }
            assert_eq!(tracker.stats()["duration"], 300.);
            assert_eq!(tracker.stats()["estimated"], true);
            assert_eq!(tracker.moves[2]["t"], 250.);
            assert_eq!(tracker.moves[3]["t"], 300.);
            assert_eq!(
                tracker.moves[3]["raw"]["localTimestamp"],
                (200 + delay) as f64
            );
            assert!(tracker.moves[3]["raw"]["cubeTimestamp"].is_null());
            assert_eq!(tracker.quality, ["estimated-timing"]);
        }
    }
    #[test]
    fn recovered_scramble_start_remains_estimated_after_genuine_clock_resumes() {
        for delay in [100, 1000] {
            let mut tracker = Tracker::default();
            let at = Instant::now();
            let mut state = Cube::default();
            for (name, clock, received, recovered) in [
                ("R", None, delay, true),
                ("U", None, delay, true),
                ("F", Some(1100.), 100, false),
                ("B", Some(1300.), 300, false),
            ] {
                let mv = Move::parse(name).unwrap();
                let next = state.turn(mv);
                tracker.observe(
                    Some(state),
                    next,
                    mv,
                    clock,
                    recovered,
                    at + std::time::Duration::from_millis(received),
                );
                state = next;
            }
            assert_eq!(tracker.stats()["duration"], 300.);
            assert_eq!(tracker.stats()["estimated"], true);
            assert_eq!(tracker.moves[0]["t"], 0.);
            assert_eq!(tracker.moves[1]["t"], 50.);
            assert_eq!(tracker.moves[2]["t"], 100.);
            assert!(tracker.moves[0]["raw"]["cubeTimestamp"].is_null());
            assert_eq!(tracker.moves[3]["raw"]["cubeTimestamp"], 1300.);
            assert_eq!(tracker.quality, ["estimated-timing"]);
        }
    }
    #[test]
    fn genuine_scramble_endpoint_replaces_overshooting_recovered_estimates() {
        for delay in [100, 1000] {
            let mut tracker = Tracker::default();
            let at = Instant::now();
            let mut state = Cube::default();
            for (name, clock, received, recovered) in [
                ("R", Some(1000.), 0, false),
                ("U", None, delay, true),
                ("F", None, delay, true),
                ("B", Some(1060.), 60, false),
            ] {
                let mv = Move::parse(name).unwrap();
                let next = state.turn(mv);
                tracker.observe(
                    Some(state),
                    next,
                    mv,
                    clock,
                    recovered,
                    at + std::time::Duration::from_millis(received),
                );
                state = next;
            }
            assert_eq!(tracker.stats()["duration"], 60.);
            assert_eq!(tracker.stats()["estimated"], false);
            assert_eq!(tracker.moves[1]["t"], 20.);
            assert_eq!(tracker.moves[2]["t"], 40.);
            assert_eq!(tracker.moves[2]["raw"]["localTimestamp"], delay as f64);
            assert!(tracker.moves[2]["raw"]["cubeTimestamp"].is_null());
            assert_eq!(tracker.moves[3]["raw"]["cubeTimestamp"], 1060.);
        }
    }
    #[test]
    fn scramble_includes_corrections_and_restarts_only_from_solved() {
        let mut t = Tracker::default();
        let now = Instant::now();
        let mut state = Cube::default();
        for (i, s) in ["R", "F", "F'", "U"].iter().enumerate() {
            let mv = Move::parse(s).unwrap();
            let next = state.turn(mv);
            t.observe(
                Some(state),
                next,
                mv,
                Some(1000. + i as f64 * 200.),
                false,
                now,
            );
            state = next;
        }
        assert_eq!(t.moves.len(), 4);
        assert_eq!(t.stats()["duration"], 600.);
        assert!((t.stats()["tps"].as_f64().unwrap() - 6.6666667).abs() < 0.0001);
        assert_eq!(
            state,
            Cube::default().apply(&crate::cube::sequence("R U").unwrap())
        );
    }
    #[test]
    fn scramble_half_turns_count_once_including_interleaved_opposite_faces() {
        let now = Instant::now();
        for notation in ["R R L L", "R L R L", "R' L' R' L'"] {
            let mut tracker = Tracker::default();
            let mut state = Cube::default();
            for (i, mv) in crate::cube::sequence(notation)
                .unwrap()
                .into_iter()
                .enumerate()
            {
                let next = state.turn(mv);
                tracker.observe(Some(state), next, mv, Some(i as f64 * 200.), false, now);
                state = next;
            }
            let metrics = tracker.stats();
            assert_eq!(tracker.moves.len(), 4, "raw events remain intact");
            assert_eq!(metrics["moves"], 2, "{notation}");
            assert_eq!(metrics["rawMoves"], 4);
            assert_eq!(metrics["duration"], 600.);
            assert!((metrics["tps"].as_f64().unwrap() - 2. / 0.6).abs() < 1e-9);
        }
        let explicit = vec![json!({"move":"R2","t":0.}), json!({"move":"L2","t":600.})];
        assert_eq!(stats(&explicit, &[])["moves"], 2);
        let undo = vec![json!({"move":"R","t":0.}), json!({"move":"R'","t":600.})];
        assert_eq!(
            stats(&undo, &[])["moves"],
            2,
            "mistakes are actual execution work"
        );
    }
    #[test]
    fn repairs_only_mistakes_and_preserves_partial_half_turn() {
        let setup = crate::cube::sequence("D2 B2 U F2 D R2 F2").unwrap();
        let anchor = Cube::default()
            .apply(&setup[..5])
            .turn(Move::parse("R").unwrap());
        let u = Move::parse("U").unwrap();
        let b = Move::parse("B").unwrap();
        let one = anchor.turn(u);
        let plan = correction(Some(anchor), one, u, &setup, &[]).unwrap();
        let two = one.turn(b);
        let plan = correction(Some(one), two, b, &setup, &plan).unwrap();
        assert_eq!(
            plan.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["B'", "U'"]
        );
        assert_eq!(two.apply(&plan), anchor);
        let undo = b.inverse();
        let plan = correction(Some(two), two.turn(undo), undo, &setup, &plan).unwrap();
        assert_eq!(
            plan.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["U'"]
        );
    }
    #[test]
    fn opposite_faces_accept_every_order_and_direction() {
        use crate::cube::{scramble_progress, sequence};
        fn visit(state: Cube, setup: &[Move], a: Move, b: Move, left_a: usize, left_b: usize) {
            let p = scramble_progress(state, setup);
            assert!(
                p.matched,
                "legal interleaving rejected: {setup:?}, {left_a}, {left_b}"
            );
            for (mv, remaining, next_a, next_b) in [
                (a, left_a, left_a.saturating_sub(1), left_b),
                (b, left_b, left_a, left_b.saturating_sub(1)),
            ] {
                if remaining == 0 {
                    continue;
                }
                let next = state.turn(mv);
                assert_eq!(correction(Some(state), next, mv, setup, &[]), Some(vec![]));
                visit(next, setup, a, b, next_a, next_b);
            }
            if left_a == 0 && left_b == 0 {
                assert_eq!(p.done, [0, 1, 2]);
                assert_eq!(p.completed, 3);
                assert!(!p.partial);
            }
        }
        for (a, b, prefix) in [("U", "D", "F"), ("R", "L", "U"), ("F", "B", "U")] {
            for suffix_a in ["", "'", "2"] {
                for suffix_b in ["", "'", "2"] {
                    let setup = sequence(&format!("{prefix} {a}{suffix_a} {b}{suffix_b}")).unwrap();
                    for direction_a in [1, -1] {
                        for direction_b in [1, -1] {
                            let qa = if suffix_a == "2" {
                                direction_a
                            } else {
                                setup[1].turns
                            };
                            let qb = if suffix_b == "2" {
                                direction_b
                            } else {
                                setup[2].turns
                            };
                            visit(
                                Cube::default().turn(setup[0]),
                                &setup,
                                Move {
                                    face: setup[1].face,
                                    turns: qa,
                                },
                                Move {
                                    face: setup[2].face,
                                    turns: qb,
                                },
                                if suffix_a == "2" { 2 } else { 1 },
                                if suffix_b == "2" { 2 } else { 1 },
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn interleaved_half_turns_report_independent_status_and_exact_recovery() {
        use crate::cube::{scramble_progress, sequence};
        let setup = sequence("R U2 D2 F").unwrap();
        let state = Cube::default().apply(&sequence("R D U' D").unwrap());
        let p = scramble_progress(state, &setup);
        assert!(p.matched);
        assert_eq!(p.completed, 1);
        assert_eq!(p.done, [0, 2]);
        assert_eq!(p.partial_moves, [1]);
        assert_eq!(p.next_moves, [1]);
        assert_eq!(p.finish, sequence("U'").unwrap());
        let mistake = Move::parse("L").unwrap();
        let wrong = state.turn(mistake);
        let recovery = correction(Some(state), wrong, mistake, &setup, &[]).unwrap();
        assert_eq!(recovery, sequence("L'").unwrap());
        assert_eq!(wrong.apply(&recovery), state);
        let both = scramble_progress(Cube::default().apply(&sequence("R D U'").unwrap()), &setup);
        assert_eq!(both.partial_moves, [1, 2]);
        assert_eq!(both.next_moves, [1, 2]);
        assert_eq!(both.finish, sequence("U' D").unwrap());
    }

    #[test]
    fn opposite_face_progress_does_not_cross_a_noncommuting_turn() {
        use crate::cube::{scramble_progress, sequence};
        let setup = sequence("R U D F B L").unwrap();
        let state = Cube::default().apply(&sequence("R D").unwrap());
        let p = scramble_progress(state, &setup);
        assert_eq!(p.done, [0, 2]);
        assert_eq!(p.next_moves, [1]);
        let f = Move::parse("F").unwrap();
        let wrong = state.turn(f);
        assert!(!scramble_progress(wrong, &setup).matched);
        assert_eq!(
            correction(Some(state), wrong, f, &setup, &[]).unwrap(),
            sequence("F'").unwrap()
        );
        let resumed = wrong.turn(f.inverse()).turn(Move::parse("U").unwrap());
        assert_eq!(scramble_progress(resumed, &setup).next_moves, [3, 4]);
        assert!(
            !scramble_progress(Cube::default().turn(Move::parse("U").unwrap()), &setup).matched
        );
    }
}

#[cfg(test)]
mod freeze_tests {
    use super::*;
    #[test]
    fn completion_freezes_device_time_through_inspection_and_resumes_only_on_another_turn() {
        let mut t = Tracker::default();
        let at = Instant::now();
        let r = Move::parse("R").unwrap();
        let u = Move::parse("U2").unwrap();
        let first = Cube::default().turn(r);
        let goal = first.turn(u);
        t.observe(Some(Cube::default()), first, r, Some(1000.), false, at);
        t.observe(
            Some(first),
            goal,
            u,
            Some(1400.),
            false,
            at + std::time::Duration::from_millis(400),
        );
        t.freeze();
        let frozen = t.stats();
        assert_eq!(frozen["moves"], 2);
        assert_eq!(
            t.live_stats_at(at + std::time::Duration::from_secs(15)),
            frozen
        );
        let f = Move::parse("F").unwrap();
        t.observe(
            Some(goal),
            goal.turn(f),
            f,
            Some(17000.),
            false,
            at + std::time::Duration::from_secs(16),
        );
        assert!(
            number(
                &t.live_stats_at(at + std::time::Duration::from_secs(17)),
                "duration"
            ) > number(&t.stats(), "duration")
        );
    }
}
