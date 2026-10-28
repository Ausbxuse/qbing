//! Readable notation inferred from fixed-color outer-face sensor events.
//!
//! This is a presentation layer, never a replacement for the recorded events,
//! their timestamps, the authoritative cube state, or sensor-event TPS. Fast
//! opposite turns can also be deliberate outer-face turns, so every inferred
//! slice remains explicitly marked and retains its original event span.
use crate::cube::{FACES, Move};
use serde_json::Value;

const GROUP_WINDOW_MS: f64 = 100.;

#[derive(Clone, Debug, PartialEq)]
pub struct NotationTurn {
    pub move_text: String,
    /// Inclusive index of the first recorded sensor event.
    pub raw_start: usize,
    /// Exclusive index; seek here to see the state after this notation turn.
    pub raw_end: usize,
    pub start: Option<f64>,
    pub elapsed: Option<f64>,
    pub inferred: bool,
    pub recovered: bool,
    pub rotation: bool,
}

#[derive(Clone, Copy)]
struct Event {
    mv: Option<Move>,
    time: Option<f64>,
    recovered: bool,
    ordered: bool,
}

fn elapsed(event: &Value) -> Option<f64> {
    event["t"].as_f64().filter(|t| t.is_finite() && *t >= 0.)
}

fn notation(face: u8, amount: i8) -> String {
    format!(
        "{}{}",
        face as char,
        match amount.rem_euclid(4) {
            2 => "2",
            3 => "'",
            _ => "",
        }
    )
}

fn in_frame(mv: Move, frame: &[u8; 6]) -> Move {
    Move {
        face: FACES[frame.iter().position(|face| *face == mv.face).unwrap()],
        turns: mv.turns,
    }
}

fn advance_frame(frame: &mut [u8; 6], slice: u8, amount: i8) {
    // New visible URFDLB centers after one positive slice turn. These are
    // permutations of the previous view, not changes to the fixed-color cube.
    let permutation = match slice {
        b'M' => [5, 1, 0, 2, 4, 3],
        b'E' => [0, 2, 4, 3, 5, 1],
        b'S' => [4, 0, 2, 1, 3, 5],
        _ => unreachable!(),
    };
    for _ in 0..amount.rem_euclid(4) {
        let previous = *frame;
        *frame = permutation.map(|i| previous[i]);
    }
}

fn trusted_window(events: &[Event]) -> bool {
    !events.is_empty()
        && events.iter().all(|event| {
            event.mv.is_some() && event.time.is_some() && event.ordered && !event.recovered
        })
        && events
            .windows(2)
            .all(|pair| pair[1].time.unwrap() - pair[0].time.unwrap() <= GROUP_WINDOW_MS)
}

/// Returns the longest balanced, same-axis prefix, at most four sensor events.
/// This includes both interleaved M M reports and R R L' L' half-turn reports.
fn slice_group(events: &[Event], frame: &[u8; 6]) -> Option<(usize, u8, i8)> {
    let first = in_frame(events.first()?.mv?, frame);
    let axis = first.axis().0;
    let (positive, negative, slice, sign) = match axis {
        0 => (b'R', b'L', b'M', 1),
        1 => (b'U', b'D', b'E', 1),
        _ => (b'F', b'B', b'S', -1),
    };
    let mut totals = [0_i8; 2];
    let mut absolute_totals = [0_i8; 2];
    let mut directions = [0_i8; 2];
    let mut seen = [false; 2];
    let mut result = None;
    for (i, event) in events.iter().take(4).enumerate() {
        if !trusted_window(&events[..=i]) {
            break;
        }
        let mv = in_frame(event.mv.unwrap(), frame);
        if mv.axis().0 != axis {
            break;
        }
        let side = if mv.face == positive { 0 } else { 1 };
        debug_assert!(mv.face == positive || mv.face == negative);
        absolute_totals[side] += mv.turns.abs();
        if absolute_totals[side] > 2 {
            break;
        }
        // Do not hide a correction (R R') inside a net slice. A direct half
        // turn has no unique direction and may combine with either sign.
        if mv.turns != 2 {
            if directions[side] != 0 && directions[side] != mv.turns {
                break;
            }
            directions[side] = mv.turns;
        }
        seen[side] = true;
        totals[side] = (totals[side] + mv.turns).rem_euclid(4);
        if seen.iter().all(|seen| *seen)
            && totals[0] != 0
            && (totals[0] + totals[1]).rem_euclid(4) == 0
        {
            result = Some((i + 1, slice, (totals[0] * sign).rem_euclid(4)));
        }
    }
    result
}

pub fn raw_turns(record: &Value) -> Vec<NotationTurn> {
    record["moves"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, e)| {
            let time = elapsed(e);
            NotationTurn {
                move_text: e["move"].as_str().unwrap_or("?").into(),
                raw_start: i,
                raw_end: i + 1,
                start: time,
                elapsed: time,
                inferred: false,
                recovered: e["recovered"] == true,
                rotation: false,
            }
        })
        .collect()
}

pub fn reconstruct(record: &Value) -> Vec<NotationTurn> {
    reconstruct_frame(record, *FACES)
}
pub fn reconstruct_for_view(record: &Value) -> Vec<NotationTurn> {
    let frame = *match crate::recording::reconstruction_cross(record) {
        b'U' => b"DLFURB",
        b'F' => b"BRUFLD",
        b'B' => b"FRDBLU",
        b'R' => b"LUFRDB",
        b'L' => b"RDFLUB",
        _ => FACES,
    };
    reconstruct_frame(record, frame)
}
fn reconstruct_frame(record: &Value, initial_frame: [u8; 6]) -> Vec<NotationTurn> {
    let Some(moves) = record["moves"].as_array() else {
        return vec![];
    };
    let timing_reliable = !record["quality"].as_array().is_some_and(|quality| {
        quality
            .iter()
            .any(|flag| matches!(flag.as_str(), Some("clock-reset" | "missing-moves")))
    });
    let events: Vec<_> = moves
        .iter()
        .enumerate()
        .map(|(i, event)| {
            let time = elapsed(event);
            Event {
                mv: event["move"].as_str().and_then(|s| Move::parse(s).ok()),
                time,
                recovered: event["recovered"].as_bool().unwrap_or(false),
                ordered: i == 0
                    || elapsed(&moves[i - 1])
                        .zip(time)
                        .is_some_and(|(a, b)| b >= a),
            }
        })
        .collect();
    let mut frame = initial_frame;
    let mut result = Vec::with_capacity(moves.len());
    let mut i = 0;
    while i < events.len() {
        let mut count = 1;
        let mut inferred = false;
        let mut text = events[i]
            .mv
            .map(|mv| {
                let mv = in_frame(mv, &frame);
                notation(mv.face, mv.turns)
            })
            .unwrap_or_else(|| moves[i]["move"].as_str().unwrap_or("?").to_owned());
        if timing_reliable {
            if let Some((length, slice, amount)) = slice_group(&events[i..], &frame) {
                count = length;
                text = notation(slice, amount);
                inferred = true;
                advance_frame(&mut frame, slice, amount);
            } else if i + 1 < events.len()
                && trusted_window(&events[i..i + 2])
                && events[i].mv == events[i + 1].mv
                && events[i].mv.is_some_and(|mv| mv.turns != 2)
            {
                count = 2;
                text = notation(in_frame(events[i].mv.unwrap(), &frame).face, 2);
            }
        }
        result.push(NotationTurn {
            move_text: text,
            rotation: false,
            raw_start: i,
            raw_end: i + count,
            start: events[i].time,
            elapsed: events[i + count - 1].time,
            inferred,
            recovered: events[i..i + count].iter().any(|event| event.recovered),
        });
        i += count;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::{Cube, geometry, rotate, sequence};
    use serde_json::json;

    fn record(raw: &str, interval: f64) -> Value {
        json!({"quality":[],"moves":raw.split_whitespace().enumerate().map(|(i,mv)|
            json!({"move":mv,"t":i as f64*interval,"recovered":false})).collect::<Vec<_>>()})
    }

    fn text(record: &Value) -> String {
        reconstruct(record)
            .iter()
            .map(|turn| turn.move_text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn slice_directions_and_both_packet_orders_are_preserved() {
        for (raw, expected) in [
            ("R L'", "M"),
            ("L' R", "M"),
            ("R' L", "M'"),
            ("L R'", "M'"),
            ("U D'", "E"),
            ("D' U", "E"),
            ("U' D", "E'"),
            ("D U'", "E'"),
            ("F' B", "S"),
            ("B F'", "S"),
            ("F B'", "S'"),
            ("B' F", "S'"),
        ] {
            let record = record(raw, 30.);
            assert_eq!(text(&record), expected, "{raw}");
            let turn = &reconstruct(&record)[0];
            assert!(turn.inferred);
            assert_eq!((turn.raw_start, turn.raw_end), (0, 2));
            assert_eq!((turn.start, turn.elapsed), (Some(0.), Some(30.)));
        }
    }

    #[test]
    fn half_turns_accept_interleaved_grouped_and_direct_half_packets() {
        for (first, second, expected) in [("R", "L'", "M2"), ("U", "D'", "E2"), ("F'", "B", "S2")] {
            for raw in [
                format!("{first} {second} {first} {second}"),
                format!("{first} {first} {second} {second}"),
                format!("{second} {first} {first} {second}"),
                format!("{second} {second} {first} {first}"),
            ] {
                assert_eq!(text(&record(&raw, 25.)), expected, "{raw}");
            }
        }
        for (raw, expected) in [
            ("R2 L2", "M2"),
            ("D2 U2", "E2"),
            ("B2 F2", "S2"),
            ("R2 L' L'", "M2"),
        ] {
            assert_eq!(text(&record(raw, 30.)), expected);
        }
        assert_eq!(text(&record("R R", 30.)), "R2");
        assert!(!reconstruct(&record("R R", 30.))[0].inferred);
        assert_eq!(text(&record("R R'", 30.)), "R R'");
        assert_eq!(text(&record("R2 R L", 20.)), "R2 R L");
        assert_eq!(text(&record("R L' R' L", 20.)), "M M'");
    }

    #[test]
    fn double_slices_use_adjacent_packet_gaps_not_the_total_burst_duration() {
        for (raw, expected) in [
            ("R R L' L'", "M2"),
            ("U U D' D'", "E2"),
            ("F' F' B B", "S2"),
            ("R L' R L'", "M2"),
        ] {
            assert_eq!(text(&record(raw, 60.)), expected, "{raw}");
            let turns = reconstruct(&record(raw, 60.));
            assert_eq!((turns[0].raw_start, turns[0].raw_end), (0, 4));
            assert_eq!(turns[0].elapsed, Some(180.));
        }
        assert_eq!(text(&record("R R L' L'", 101.)), "R R L' L'");
    }

    #[test]
    fn following_turns_use_the_updated_notation_frame() {
        for (raw, expected) in [("R L' B", "M U"), ("U D' F", "E R"), ("F' B L", "S U")] {
            assert_eq!(text(&record(raw, 40.)), expected);
        }
        assert_eq!(text(&record("R' L F", 40.)), "M' U");
    }

    #[test]
    fn uncertain_or_slow_timing_never_invents_a_slice() {
        assert_eq!(text(&record("R L'", 101.)), "R L'");
        assert_eq!(text(&record("R L'", 100.)), "M");
        for flag in ["clock-reset", "missing-moves"] {
            let mut r = record("R L'", 20.);
            r["quality"] = json!([flag]);
            assert_eq!(text(&r), "R L'");
        }
        let mut r = record("R L'", 0.);
        r["moves"][0]["recovered"] = json!(true);
        assert_eq!(text(&r), "R L'");
        assert!(reconstruct(&r)[0].recovered);
        r["moves"][0]["recovered"] = json!(false);
        r["moves"][0]["t"] = Value::Null;
        assert_eq!(text(&r), "R L'");
        assert_eq!(reconstruct(&r)[0].elapsed, None);
        r["moves"][0]["t"] = json!(100.);
        r["moves"][1]["t"] = json!(80.);
        assert_eq!(text(&r), "R L'");
    }

    #[test]
    fn spans_keep_all_events_and_do_not_mutate_raw_history() {
        let r = record("R L' B F' B U U bad R L'", 20.);
        let before = r.clone();
        let turns = reconstruct(&r);
        assert_eq!(r, before);
        assert_eq!(turns.first().unwrap().raw_start, 0);
        assert_eq!(
            turns.last().unwrap().raw_end,
            r["moves"].as_array().unwrap().len()
        );
        for adjacent in turns.windows(2) {
            assert_eq!(adjacent[0].raw_end, adjacent[1].raw_start);
        }
        assert!(
            turns
                .iter()
                .any(|turn| turn.move_text == "bad" && turn.raw_end == turn.raw_start + 1)
        );
        assert_eq!(text(&record("R U L'", 20.)), "R U L'");
    }

    // Independent geometric oracle: unlike the inference code, this moves the
    // actual middle plane and normalizes physical cube-center orientation.
    fn geometric_turn(cube: Cube, axis: usize, layer: Option<f32>, sign: f32) -> Cube {
        let mut result = cube;
        for (i, sticker) in geometry().iter().enumerate() {
            if layer.is_some_and(|layer| sticker.p[axis] != layer) {
                continue;
            }
            let position =
                rotate(sticker.p, axis, sign * std::f32::consts::FRAC_PI_2).map(f32::round);
            let normal =
                rotate(sticker.n, axis, sign * std::f32::consts::FRAC_PI_2).map(f32::round);
            let j = geometry()
                .iter()
                .position(|s| s.p == position && s.n == normal)
                .unwrap();
            result.0[j] = cube.0[i];
        }
        result
    }

    fn literal_turn(mut cube: Cube, text: &str) -> Cube {
        let amount = if text.ends_with('2') {
            2
        } else if text.ends_with('\'') {
            3
        } else {
            1
        };
        let (axis, layer, sign) = match text.as_bytes()[0] {
            b'M' => (0, 0., 1.),
            b'E' => (1, 0., 1.),
            b'S' => (2, 0., -1.),
            face => Move { face, turns: 1 }.axis(),
        };
        for _ in 0..amount {
            cube = geometric_turn(cube, axis, Some(layer), sign);
        }
        cube
    }

    fn normalized(cube: Cube) -> Cube {
        let mut pending = vec![cube];
        let mut index = 0;
        while index < pending.len() {
            let current = pending[index];
            if FACES
                .iter()
                .enumerate()
                .all(|(face, color)| current.0[face * 9 + 4] == *color)
            {
                return current;
            }
            for axis in 0..3 {
                let next = geometric_turn(current, axis, None, 1.);
                if !pending
                    .iter()
                    .any(|seen| (0..6).all(|face| seen.0[face * 9 + 4] == next.0[face * 9 + 4]))
                {
                    pending.push(next);
                }
            }
            index += 1;
        }
        panic!("No normalized center frame");
    }

    #[test]
    fn literal_slices_preserve_normalized_cube_state_at_every_group_boundary() {
        let initial = Cube::default().apply(&sequence("R U F2 L' D B R2 U'").unwrap());
        for raw in [
            "R L' B U D' F F' B L R' L U' D F B'",
            "R R L' L' U F' B D2 U2 R' L B F' R R",
            "L R' F D U' R B F' U L2 R2 F B' U2 D2",
        ] {
            let r = record(raw, 20.);
            let raw_moves = sequence(raw).unwrap();
            let mut physical = initial;
            for group in reconstruct(&r) {
                physical = literal_turn(physical, &group.move_text);
                assert_eq!(
                    normalized(physical),
                    initial.apply(&raw_moves[..group.raw_end]),
                    "{raw}: {} at {}",
                    group.move_text,
                    group.raw_end
                );
            }
        }
    }
}

/// Rotation observations have an empty raw span: seeking never applies them as
/// face turns. Their x/y/z labels describe physical pose deltas in the initial
/// holding frame, while sensor face labels stay in the fixed color frame.
pub fn with_rotations(mut turns: Vec<NotationTurn>, record: &Value) -> Vec<NotationTurn> {
    if let (Some(rotations), Some(moves)) =
        (record["rotations"].as_array(), record["moves"].as_array())
    {
        for r in rotations {
            let Some(t) = r["t"].as_f64().filter(|t| t.is_finite() && *t >= 0.) else {
                continue;
            };
            let Some(notation) = r["notation"].as_str().filter(|s| {
                s.split_whitespace()
                    .all(|m| ["x", "x'", "x2", "y", "y'", "y2", "z", "z'", "z2"].contains(&m))
            }) else {
                continue;
            };
            let index = moves
                .iter()
                .take_while(|m| m["t"].as_f64().is_some_and(|v| v <= t))
                .count();
            turns.push(NotationTurn {
                move_text: notation.to_string(),
                raw_start: index,
                raw_end: index,
                start: Some(t),
                elapsed: Some(t),
                inferred: false,
                recovered: false,
                rotation: true,
            });
        }
    }
    turns.sort_by(|a, b| {
        a.elapsed
            .partial_cmp(&b.elapsed)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    turns
}

#[cfg(test)]
mod gyro_timeline_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rotations_interleave_without_mutating_face_turns_or_move_counts() {
        let r = json!({"source":"bluetooth","status":"complete","duration":1000.,"quality":[],"moves":[{"move":"R","t":0},{"move":"U","t":1000}],"rotations":[{"notation":"y'","t":500,"pose":[0,0.707,0,0.707],"timing":"local-gyro"}]});
        assert_eq!(crate::recording::execution_moves(&r), Some(2));
        assert_eq!(crate::recording::execution_metrics(&r).unwrap().tps, 2.);
        let rotations = json!([{"move":"x"},{"move":"x'"},{"move":"x2"},{"move":"y"},{"move":"y'"},{"move":"y2"},{"move":"z"},{"move":"z'"},{"move":"z2"}]);
        assert_eq!(
            crate::recording::count_moves(rotations.as_array().unwrap()),
            0
        );
        let raw = r["moves"].as_array().unwrap().clone();
        let count = crate::recording::count_moves(&raw);
        let timeline = with_rotations(reconstruct(&r), &r);
        assert_eq!(
            timeline
                .iter()
                .map(|t| t.move_text.as_str())
                .collect::<Vec<_>>(),
            vec!["R", "y'", "U"]
        );
        assert!(timeline[1].rotation);
        assert_eq!(timeline[1].raw_start, 1);
        assert_eq!(timeline[1].raw_end, 1);
        assert_eq!(r["moves"], json!(raw));
        assert_eq!(crate::recording::count_moves(&raw), count);
        assert!(
            with_rotations(reconstruct(&json!({"moves":raw})), &json!({"moves":raw}))
                .iter()
                .all(|t| !t.rotation)
        );
    }
}

#[cfg(test)]
mod view_frame_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn cross_down_notation_matches_the_view_and_raw_events_stay_original() {
        let r = json!({"cross":"D","analysis":{"cross":"U"},"moves":[{"move":"R","t":0},{"move":"U","t":200}]});
        assert_eq!(
            reconstruct_for_view(&r)
                .iter()
                .map(|t| t.move_text.as_str())
                .collect::<Vec<_>>(),
            vec!["L", "D"]
        );
        assert_eq!(
            reconstruct(&r)
                .iter()
                .map(|t| t.move_text.as_str())
                .collect::<Vec<_>>(),
            vec!["R", "U"]
        );
        let yellow = json!({"cross":"U","analysis":{"cross":"D"},"moves":r["moves"]});
        assert_eq!(reconstruct_for_view(&yellow), reconstruct(&yellow));
    }
}
