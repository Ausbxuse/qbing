//! CFOP phases in the cube's fixed color frame, independent of camera/gyro pose.
use crate::{
    cube::{Cube, FACES, Move},
    io::{number, value_str},
    recording::move_weights,
};
use serde_json::{Value, json};

const PHASES: [&str; 4] = ["Cross", "F2L", "OLL", "PLL"];

struct CrossSelection {
    face: u8,
    source: &'static str,
    evidence: &'static str,
    candidates: Vec<u8>,
}

/// Index into the trace, not elapsed time: recovered turns may share a timestamp.
/// F2L is stronger evidence than a cross which happened to be solved initially.
fn select_cross(record: &Value, trace: &[[[bool; 4]; 6]]) -> CrossSelection {
    let preferred = value_str(record, "cross")
        .as_bytes()
        .first()
        .copied()
        .filter(|f| FACES.contains(f))
        .unwrap_or(b'U');
    let configured = || CrossSelection {
        face: preferred,
        source: "configured",
        evidence: "configured",
        candidates: vec![preferred],
    };
    if value_str(record, "kind") == "drill" || !matches!(value_str(record, "stage"), "" | "solve") {
        return configured();
    }
    for (phase, evidence) in [(1, "f2l"), (0, "cross")] {
        let first = FACES.map(|face| {
            let i = FACES.iter().position(|f| *f == face).unwrap();
            trace.iter().position(|state| state[i][phase])
        });
        let Some(earliest) = first.iter().flatten().copied().min() else {
            continue;
        };
        let candidates: Vec<u8> = FACES
            .iter()
            .zip(first)
            .filter_map(|(face, index)| (index == Some(earliest)).then_some(*face))
            .collect();
        return CrossSelection {
            face: if candidates.contains(&preferred) {
                preferred
            } else {
                candidates[0]
            },
            source: if candidates.len() == 1 {
                "detected"
            } else {
                "ambiguous"
            },
            evidence,
            candidates,
        };
    }
    configured()
}

pub(super) fn analyze(record: &Value) -> Value {
    let Ok(initial) = Cube::parse(value_str(record, "initialState")) else {
        return Value::Null;
    };
    let moves = record["moves"].as_array().cloned().unwrap_or_default();
    let weights = move_weights(&moves);
    let move_count: usize = weights.iter().sum();
    let parsed: Vec<_> = moves
        .iter()
        .map(|event| Move::parse(value_str(event, "move")).ok())
        .collect();
    let mut cube = initial;
    let mut trace = Vec::with_capacity(moves.len() + 1);
    trace.push(FACES.map(|face| cube.progress(face)));
    // Replay geometry once. The resulting color traces drive both detection and
    // boundaries, so all six candidates see exactly the same raw turn order.
    for mv in &parsed {
        if let Some(mv) = mv {
            cube = cube.turn(*mv);
        }
        trace.push(FACES.map(|face| cube.progress(face)));
    }
    let selected = select_cross(record, &trace);
    let face_index = FACES
        .iter()
        .position(|face| *face == selected.face)
        .unwrap();
    let mut milestones = trace[0][face_index].map(|done| done.then_some(0.));
    let mut counts = [0; 4];
    let mut raw_counts = [0; 4];
    let mut pauses = vec![];
    let mut signals = vec![];
    let mut previous = 0.;
    let mut previous_move: Option<Move> = None;
    for (i, event) in moves.iter().enumerate() {
        let stage = milestones.iter().position(Option::is_none).unwrap_or(3);
        raw_counts[stage] += 1;
        let Some(mv) = parsed[i] else { continue };
        // A half turn may arrive as two packets, with opposite-face packets
        // interleaved. Credit its one HTM move where the second packet lands;
        // raw counts remain authoritative for replay and phase boundaries.
        counts[stage] += weights[i];
        let t = number(event, "t");
        if i > 0 && t - previous >= 500. {
            pauses.push(json!({"index":i,"t":previous,"duration":t-previous,"stage":PHASES[stage],"estimated":event["recovered"].as_bool().unwrap_or(false) || moves[i-1]["recovered"].as_bool().unwrap_or(false)}));
        }
        if previous_move == Some(mv.inverse()) {
            signals.push(json!({"type":"undo","index":i,"stage":PHASES[stage],"text":format!("{} {}",mv.inverse(),mv),"t":t}));
        }
        let progress = trace[i + 1][face_index];
        for phase in 0..4 {
            // Later algorithms temporarily disturb earlier solved pieces; a
            // reached boundary remains reached within this selected color frame.
            if milestones[phase].is_none()
                && progress[phase]
                && (phase == 0 || milestones[phase - 1].is_some())
            {
                milestones[phase] = Some(t);
            }
        }
        previous = t;
        previous_move = Some(mv);
    }
    let mut start = 0.;
    let splits = PHASES.iter().enumerate().map(|(i, stage)| {
        let end = milestones[i];
        let time = end.map(|t| t - start);
        let pause = pauses.iter().filter(|p| value_str(p,"stage") == *stage).map(|p| number(p,"duration")).sum::<f64>();
        let split = json!({"stage":stage,"start":start,"end":end,"time":time,"moves":counts[i],"rawMoves":raw_counts[i],"pause":pause,"tps":time.filter(|t| *t > 0.).map(|t| counts[i] as f64 * 1000. / t)});
        if let Some(t) = end { start = t; }
        split
    }).collect::<Vec<_>>();
    let duration = number(record, "duration");
    let paused = pauses
        .iter()
        .map(|value| number(value, "duration"))
        .sum::<f64>();
    json!({"version":crate::model::ANALYSIS_VERSION,"cross":(selected.face as char).to_string(),"crossSource":selected.source,"crossEvidence":selected.evidence,"crossCandidates":selected.candidates.into_iter().map(|face|(face as char).to_string()).collect::<Vec<_>>(),"splits":splits,"pauses":pauses,"signals":signals,"pairEvents":[],"moves":move_count,"rawMoves":moves.len(),"duration":duration,"pauseTime":paused,"pauseFraction":if duration>0.{paused/duration}else{0.},"tps":if duration>0.{Some(move_count as f64/(duration/1000.))}else{None},"finalState":cube.to_string()})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::{geometry, rotate, sequence};

    const SOLUTION: &str = "D R U R' U' R U R' U R U2 R' R U R' U' R' F R2 U' R' U' R U R' F'";

    fn solution(cross: u8) -> Vec<Move> {
        let (axis, quarters) = match cross {
            b'U' => (0, 2.),
            b'R' => (2, 1.),
            b'F' => (0, -1.),
            b'D' => (0, 0.),
            b'L' => (2, -1.),
            b'B' => (0, 1.),
            _ => unreachable!(),
        };
        sequence(SOLUTION)
            .unwrap()
            .into_iter()
            .map(|mv| {
                let i = FACES.iter().position(|face| *face == mv.face).unwrap();
                let normal = rotate(
                    geometry()[i * 9 + 4].n,
                    axis,
                    quarters * std::f32::consts::FRAC_PI_2,
                )
                .map(f32::round);
                let face = FACES
                    .iter()
                    .find(|face| {
                        let index = FACES.iter().position(|f| f == *face).unwrap();
                        geometry()[index * 9 + 4].n == normal
                    })
                    .unwrap();
                Move { face: *face, ..mv }
            })
            .collect()
    }
    fn record(cross: u8, length: usize) -> Value {
        let moves = solution(cross);
        let initial = Cube::default().apply(
            &moves
                .iter()
                .rev()
                .map(|mv| mv.inverse())
                .collect::<Vec<_>>(),
        );
        json!({"kind":"solve","stage":"solve","cross":"U","initialState":initial.to_string(),"duration":length.saturating_sub(1) * 100,
            "moves":moves.iter().take(length).enumerate().map(|(i,mv)|json!({"move":mv.to_string(),"t":i * 100})).collect::<Vec<_>>()})
    }
    #[test]
    fn all_six_cross_colors_keep_the_same_cfop_boundaries() {
        for face in FACES {
            let record = record(*face, 26);
            let a = analyze(&record);
            assert_eq!(a["cross"], (*face as char).to_string());
            assert_eq!(a["crossSource"], "detected");
            assert_eq!(a["crossEvidence"], "f2l");
            assert_eq!(a["version"], crate::model::ANALYSIS_VERSION);
            assert_eq!(
                a["splits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s["moves"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                [1, 3, 8, 14]
            );
            assert_eq!(
                a["splits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s["end"].as_f64().unwrap())
                    .collect::<Vec<_>>(),
                [0., 300., 1100., 2500.]
            );
            assert_eq!(a["finalState"], Cube::default().to_string());
            assert_eq!(
                record["cross"], "U",
                "the configured color remains untouched"
            );
        }
    }
    #[test]
    fn live_color_uses_observed_cross_then_f2l_and_ignores_gyro() {
        let early = analyze(&record(b'D', 1));
        assert_eq!(early["cross"], "D");
        assert_eq!(early["crossEvidence"], "cross");
        let mut r = record(b'D', 4);
        let f2l = analyze(&r);
        assert_eq!(f2l["cross"], "D");
        assert_eq!(f2l["crossEvidence"], "f2l");
        assert_eq!(f2l["splits"][1]["end"], 300.);
        assert!(f2l["splits"][2]["end"].is_null());
        r["orientation"] = json!([0.7, 0., 0., 0.7]);
        assert_eq!(analyze(&r), f2l);
    }
    #[test]
    fn tied_times_do_not_erase_the_order_of_phase_evidence() {
        let mut r = record(b'D', 26);
        for event in r["moves"].as_array_mut().unwrap() {
            event["t"] = json!(0.);
        }
        let a = analyze(&r);
        assert_eq!(a["cross"], "D");
        assert_eq!(a["splits"][1]["moves"], 3);
        assert_eq!(a["splits"][3]["moves"], 14);
    }
    #[test]
    fn double_turn_packets_count_once_without_changing_phase_boundaries() {
        let mut r = record(b'D', 26);
        let moves: Vec<_> = solution(b'D')
            .into_iter()
            .flat_map(|mv| {
                if mv.turns == 2 {
                    vec![Move { turns: 1, ..mv }; 2]
                } else {
                    vec![mv]
                }
            })
            .collect();
        r["moves"] = json!(
            moves
                .iter()
                .enumerate()
                .map(|(i, mv)| json!({"move":mv.to_string(),"t":i * 100}))
                .collect::<Vec<_>>()
        );
        r["duration"] = json!((moves.len() - 1) * 100);
        let a = analyze(&r);
        assert_eq!(a["cross"], "D");
        assert_eq!(
            a["splits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["moves"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [1, 3, 8, 14]
        );
        assert_eq!(
            a["splits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["rawMoves"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [1, 3, 9, 15]
        );
        assert_eq!(a["moves"], 26);
        assert_eq!(a["rawMoves"], 28);
        assert_eq!(
            a["splits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["end"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            [0., 300., 1200., 2700.]
        );
        assert_eq!(a["finalState"], Cube::default().to_string());
    }
    #[test]
    fn opposite_face_half_turns_share_one_count_per_completed_face_turn() {
        let turns = sequence("R L R L").unwrap();
        let initial = Cube::default().apply(
            &turns
                .iter()
                .rev()
                .map(|mv| mv.inverse())
                .collect::<Vec<_>>(),
        );
        let r = json!({"kind":"drill","stage":"Cross","cross":"D",
            "initialState":initial.to_string(),"duration":600.,
            "moves":turns.iter().enumerate().map(|(i,mv)| json!({"move":mv.to_string(),"t":i * 200})).collect::<Vec<_>>()});
        let a = analyze(&r);
        assert_eq!(a["moves"], 2);
        assert_eq!(a["rawMoves"], 4);
        assert_eq!(a["splits"][0]["moves"], 2);
        assert_eq!(a["splits"][0]["rawMoves"], 4);
        assert!((a["tps"].as_f64().unwrap() - 2. / 0.6).abs() < 1e-9);
        assert_eq!(a["splits"][0]["tps"], a["tps"]);
        assert_eq!(a["finalState"], Cube::default().to_string());
    }
    #[test]
    fn a_double_spanning_a_phase_boundary_is_credited_at_its_closing_packet() {
        let middle = Cube::default().apply(&sequence("R U R' U'").unwrap());
        assert!(middle.progress(b'D')[0]);
        assert!(!middle.progress(b'D')[1]);
        let initial = middle.turn(Move::parse("R'").unwrap());
        let turns = sequence("R R R' U R U' R'").unwrap();
        let r = json!({"kind":"drill","stage":"F2L","cross":"D",
            "initialState":initial.to_string(),"duration":600.,
            "moves":turns.iter().enumerate().map(|(i,mv)| json!({"move":mv.to_string(),"t":i * 100})).collect::<Vec<_>>()});
        let a = analyze(&r);
        assert_eq!(a["moves"], 6);
        assert_eq!(a["rawMoves"], 7);
        assert_eq!(a["splits"][0]["rawMoves"], 1);
        assert_eq!(a["splits"][0]["moves"], 0);
        assert_eq!(a["splits"][1]["moves"], 6);
        assert_eq!(a["splits"][1]["rawMoves"], 6);
        assert_eq!(a["splits"][0]["end"], 0.);
        assert_eq!(a["splits"][1]["end"], 600.);
        assert_eq!(a["finalState"], Cube::default().to_string());
        assert_eq!(
            a["splits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["moves"].as_u64().unwrap())
                .sum::<u64>(),
            6
        );
    }
    #[test]
    fn cross_solution_response_is_bound_to_the_requested_color() {
        use crate::{io::ApiEvent, model::Model};
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = std::sync::mpsc::channel().0;
        let mut r = record(b'D', 26);
        r["id"] = json!("cross-test");
        r["analysis"] = analyze(&r);
        m.records = vec![r];
        m.api_event(ApiEvent {
            tag: "cross:cross-test:U".into(),
            result: Ok(json!({"distance":6,"moves":["U"]})),
            sent: vec![],
        });
        assert!(
            m.records[0]["crossOptimal"].is_null(),
            "a late result for the configured color must not label the detected cross"
        );
        m.api_event(ApiEvent {
            tag: "cross:cross-test:D".into(),
            result: Ok(json!({"distance":1,"moves":["D"]})),
            sent: vec![],
        });
        assert_eq!(m.records[0]["crossOptimal"]["cross"], "D");
        assert_eq!(m.records[0]["crossOptimal"]["distance"], 1);
    }
    #[test]
    fn drills_keep_the_selected_cross_and_ambiguous_solves_keep_preference() {
        let mut r = record(b'D', 26);
        r["kind"] = json!("drill");
        assert_eq!(analyze(&r)["cross"], "U");
        assert_eq!(analyze(&r)["crossSource"], "configured");
        r["kind"] = json!("solve");
        r["stage"] = json!("Cross");
        assert_eq!(analyze(&r)["crossSource"], "configured");
        for face in FACES {
            let r = json!({"cross":(*face as char).to_string(),"initialState":Cube::default().to_string(),"moves":[]});
            let a = analyze(&r);
            assert_eq!(a["cross"], r["cross"]);
            assert_eq!(a["crossSource"], "ambiguous");
            assert_eq!(a["crossCandidates"].as_array().unwrap().len(), 6);
        }
        let unsolved = Cube::default().apply(&sequence("F R U B L D R2").unwrap());
        assert!(FACES.iter().all(|face| !unsolved.progress(*face)[0]));
        let no_evidence = json!({"cross":"F","initialState":unsolved.to_string(),"moves":[]});
        assert_eq!(analyze(&no_evidence)["cross"], "F");
        assert_eq!(analyze(&no_evidence)["crossSource"], "configured");
    }
    #[test]
    fn auf_only_solves_skip_cross_f2l_and_oll() {
        let initial = Cube::default().turn(Move::parse("U'").unwrap());
        let r = json!({"cross":"U","initialState":initial.to_string(),"duration":200.,"moves":[{"move":"U","t":200.}]});
        let a = analyze(&r);
        assert_eq!(a["cross"], "D");
        for i in 0..3 {
            assert_eq!(a["splits"][i]["moves"], 0);
            assert_eq!(a["splits"][i]["time"], 0.);
        }
        assert_eq!(a["splits"][3]["moves"], 1);
        assert_eq!(a["splits"][3]["time"], 200.);
    }
}
