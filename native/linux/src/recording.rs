//! Presentation of recorded execution, independent of the scored time/penalty.
use crate::{
    cube::Move,
    io::{number, value_str},
};
use serde_json::Value;

/// Half-turn metric over raw sensor events. Opposite faces commute, so their
/// packets may interleave. Credit a completed double to its closing packet.
/// Never cancel inverse turns or rewrite the authoritative event stream.
pub fn move_weights(events: &[Value]) -> Vec<usize> {
    let mut weights = vec![0; events.len()];
    let mut axis = None;
    let mut pending: [Option<(usize, i8)>; 6] = [None; 6];
    for (index, event) in events.iter().enumerate() {
        let Ok(mv) = Move::parse(value_str(event, "move")) else {
            axis = None;
            pending.fill(None);
            continue;
        };
        let face = b"URFDLB".iter().position(|face| *face == mv.face).unwrap();
        let next_axis = face % 3;
        if axis != Some(next_axis) {
            pending.fill(None);
            axis = Some(next_axis);
        }
        weights[index] = 1;
        if mv.turns.abs() == 2 {
            pending[face] = None;
        } else if let Some((previous, direction)) = pending[face]
            && direction == mv.turns
        {
            weights[previous] = 0;
            pending[face] = None;
        } else {
            pending[face] = Some((index, mv.turns));
        }
    }
    weights
}

pub fn count_moves(events: &[Value]) -> usize {
    move_weights(events).into_iter().sum()
}

#[derive(Clone, Copy, Debug)]
pub struct ExecutionMetrics {
    pub tps: f64,
    pub estimated: bool,
}
impl ExecutionMetrics {
    pub fn label(self) -> String {
        format!("{}{:.2}", if self.estimated { "≈ " } else { "" }, self.tps)
    }
}

/// Count complete, verified executions independently of timestamp quality.
/// A recovered turn is still a turn; a broken clock cannot make it disappear.
pub fn execution_moves(record: &Value) -> Option<usize> {
    if value_str(record, "source") == "manual" || value_str(record, "status") != "complete" {
        return None;
    }
    if record["quality"].as_array().is_some_and(|flags| {
        flags
            .iter()
            .any(|flag| !matches!(flag.as_str(), Some("estimated-timing" | "clock-reset")))
    }) {
        return None;
    }
    let moves = record["moves"]
        .as_array()
        .filter(|moves| !moves.is_empty())?;
    for event in moves {
        Move::parse(value_str(event, "move")).ok()?;
    }
    Some(count_moves(moves))
}

pub fn has_estimated_turns(record: &Value) -> bool {
    record["quality"]
        .as_array()
        .is_some_and(|flags| flags.iter().any(|flag| flag == "estimated-timing"))
        || record["moves"]
            .as_array()
            .is_some_and(|moves| moves.iter().any(|event| event["recovered"] == true))
}

/// A direct device timestamp is precise only when the saved elapsed time agrees
/// with the same direct starting timestamp. Do not silently repair old data.
pub fn turn_timing_estimated(record: &Value, index: usize) -> bool {
    let Some(moves) = record["moves"].as_array() else {
        return true;
    };
    let Some(event) = moves.get(index) else {
        return true;
    };
    if event["recovered"] == true {
        return true;
    }
    if record["quality"]
        .as_array()
        .is_some_and(|flags| flags.iter().any(|flag| flag == "clock-reset"))
    {
        return true;
    }
    if !has_estimated_turns(record) {
        return false;
    }
    let first = &moves[0];
    if first["recovered"] == true {
        return true;
    }
    let timestamps = first["raw"]["cubeTimestamp"]
        .as_f64()
        .zip(event["raw"]["cubeTimestamp"].as_f64())
        .zip(event["t"].as_f64());
    !timestamps.is_some_and(|((start, end), elapsed)| {
        start.is_finite()
            && start >= 0.
            && end.is_finite()
            && elapsed.is_finite()
            && end >= start
            && (end - start - elapsed).abs() <= 1.
            && first["t"].as_f64().is_some_and(|t| t.abs() <= 1.)
    })
}

/// Certainty of the execution duration, independent of final-state validation
/// and TPS eligibility. A real endpoint stays precise after interior recovery.
pub fn execution_duration_estimated(record: &Value) -> bool {
    if value_str(record, "source") == "manual" {
        return false;
    }
    let Some(moves) = record["moves"].as_array().filter(|moves| !moves.is_empty()) else {
        return false;
    };
    if record["quality"]
        .as_array()
        .is_some_and(|flags| flags.iter().any(|flag| flag == "clock-reset"))
    {
        return true;
    }
    let mut previous_clock = None;
    for event in moves.iter().filter(|event| event["recovered"] != true) {
        if let Some(clock) = event["raw"]["cubeTimestamp"].as_f64() {
            if !clock.is_finite()
                || clock < 0.
                || previous_clock.is_some_and(|previous| clock < previous)
            {
                return true;
            }
            previous_clock = Some(clock);
        }
    }
    has_estimated_turns(record)
        && (turn_timing_estimated(record, 0)
            || turn_timing_estimated(record, moves.len() - 1)
            || moves
                .last()
                .and_then(|event| event["t"].as_f64())
                .zip(record["duration"].as_f64())
                .is_none_or(|(end, duration)| {
                    !end.is_finite() || !duration.is_finite() || (end - duration).abs() > 1.
                }))
}

/// Interior packet recovery does not make an entire phase approximate when
/// both its boundaries retain authoritative device timing. Counts identify the
/// boundaries even when adjacent turns share an elapsed timestamp.
pub fn split_timing_estimated(record: &Value, phase: &str) -> bool {
    let Some(splits) = record["analysis"]["splits"].as_array() else {
        return has_estimated_turns(record);
    };
    let mut end = 0;
    for split in splits {
        let start = end;
        end += split["rawMoves"]
            .as_u64()
            .or_else(|| split["moves"].as_u64())
            .unwrap_or(0) as usize;
        if value_str(split, "stage") == phase {
            return start
                .checked_sub(1)
                .is_some_and(|i| turn_timing_estimated(record, i))
                || end
                    .checked_sub(1)
                    .is_some_and(|i| turn_timing_estimated(record, i));
        }
    }
    has_estimated_turns(record)
}

/// Total TPS needs the complete sequence and its endpoints, not precise timing
/// for every interior turn. Retain estimates when endpoint evidence is absent;
/// never change raw timing flags or let a scoring penalty change execution TPS.
pub fn execution_metrics(record: &Value) -> Option<ExecutionMetrics> {
    let count = execution_moves(record)?;
    let quality = record["quality"].as_array();
    if quality.is_some_and(|flags| {
        flags
            .iter()
            .any(|flag| flag.as_str() != Some("estimated-timing"))
    }) {
        return None;
    }
    let duration = record["duration"]
        .as_f64()
        .filter(|v| v.is_finite() && *v > 0.)?;
    let moves = record["moves"]
        .as_array()
        .filter(|moves| !moves.is_empty())?;
    let mut previous = 0.;
    for event in moves {
        Move::parse(value_str(event, "move")).ok()?;
        let t = event["t"]
            .as_f64()
            .filter(|v| v.is_finite() && *v >= previous && *v <= duration + 1.)?;
        previous = t;
    }
    let mut previous_clock = None;
    let clocks_consistent = moves
        .iter()
        .filter(|event| event["recovered"] != true)
        .all(|event| {
            let Some(clock) = event["raw"]["cubeTimestamp"].as_f64() else {
                return true;
            };
            let valid = clock.is_finite()
                && clock >= 0.
                && previous_clock.is_none_or(|previous| clock >= previous);
            previous_clock = Some(clock);
            valid
        });
    let estimated = !clocks_consistent
        || has_estimated_turns(record)
            && (turn_timing_estimated(record, 0)
                || turn_timing_estimated(record, moves.len() - 1)
                || (previous - duration).abs() > 1.);
    Some(ExecutionMetrics {
        tps: count as f64 * 1000. / duration,
        estimated,
    })
}

/// Upgrade only derived analysis. The configured face and every raw recording
/// field remain unchanged, including notes, penalties and timing-quality flags.
/// An older server cannot replace corrected cached phase boundaries on sync.
pub fn refresh_analysis(mut record: Value) -> Value {
    if value_str(&record, "source") != "manual"
        && record["moves"]
            .as_array()
            .is_some_and(|moves| !moves.is_empty())
        && record["analysis"]["version"].as_u64().unwrap_or(0) < crate::model::ANALYSIS_VERSION
    {
        let mut local = crate::model::analysis(&record);
        if local.is_object() {
            let old = &record["analysis"];
            let previous_cross = old["cross"].as_str().unwrap_or(value_str(&record, "cross"));
            let same_cross = local["cross"].as_str() == Some(previous_cross);
            if same_cross {
                // Preserve richer server annotations only for the same physical
                // frame. Per-phase case/algorithm labels also require unchanged
                // boundaries, since an earlier false milestone changes the case.
                for key in ["pairEvents", "reducedTurns"] {
                    if !old[key].is_null() {
                        local[key] = old[key].clone();
                    }
                }
                let mut same_boundaries = true;
                if let Some(splits) = local["splits"].as_array_mut() {
                    for split in splits {
                        let previous = old["splits"].as_array().and_then(|splits| {
                            splits.iter().find(|prior| prior["stage"] == split["stage"])
                        });
                        let compatible = previous.is_some_and(|prior| {
                            ["start", "end"].iter().all(|key| prior[key] == split[key])
                                && prior["rawMoves"]
                                    .as_u64()
                                    .or_else(|| prior["moves"].as_u64())
                                    == split["rawMoves"]
                                        .as_u64()
                                        .or_else(|| split["moves"].as_u64())
                        });
                        same_boundaries &= compatible;
                        if compatible {
                            let previous = previous.unwrap();
                            // Old algorithm matches used a different slice signature;
                            // keep the case, but let the server recompute the execution.
                            for key in ["caseId"] {
                                if !previous[key].is_null() {
                                    split[key] = previous[key].clone();
                                }
                            }
                            if old["version"].as_u64().unwrap_or(0) >= 3
                                && !previous["algorithm"].is_null()
                            {
                                split["algorithm"] = previous["algorithm"].clone();
                            }
                        }
                    }
                }
                if same_boundaries && !old["cases"].is_null() {
                    local["cases"] = old["cases"].clone();
                }
            }
            record["analysis"] = local;
        }
    }
    if record["crossOptimal"].is_object() {
        let chosen = record["analysis"]["cross"]
            .as_str()
            .unwrap_or(value_str(&record, "cross"));
        let optimal_cross = record["crossOptimal"]["cross"]
            .as_str()
            .unwrap_or(value_str(&record, "cross"));
        if chosen != optimal_cross {
            // This is a derived comparison for a different face, never raw data.
            record["crossOptimal"] = Value::Null;
        }
    }
    record
}

pub fn refresh_records(records: &mut [Value]) -> bool {
    let mut changed = false;
    for record in records {
        let refreshed = refresh_analysis(record.clone());
        changed |= *record != refreshed;
        *record = refreshed;
    }
    changed
}

/// Reviews also work with old imports before a cache or server round trip.
pub fn for_review(record: Value) -> Value {
    refresh_analysis(record)
}

pub fn turn_elapsed(record: &Value, index: usize) -> f64 {
    index
        .checked_sub(1)
        .and_then(|index| record["moves"].get(index))
        .map(|event| number(event, "t"))
        .unwrap_or(0.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record() -> Value {
        json!({"source":"bluetooth", "status":"complete", "penalty":"", "duration":1000.,
            "quality":[], "initialState":crate::cube::Cube::default().turn(Move::parse("R").unwrap()).turn(Move::parse("U").unwrap()).to_string(),
            "cross":"U", "moves":[{"move":"U'","t":0.},{"move":"R'","t":1000.}]})
    }
    #[test]
    fn half_turn_counts_commute_without_cancelling_inverse_turns() {
        for (text, expected) in [
            ("R", 1),
            ("R'", 1),
            ("R2", 1),
            ("R R", 1),
            ("R' R'", 1),
            ("R R'", 2),
            ("R R R", 2),
            ("R R R R", 2),
            ("R2 R", 2),
            ("R L R L", 2),
            ("R R L L", 2),
            ("R L'", 2),
            ("R U R", 3),
            ("R L R' L", 3),
            ("U D U D", 2),
            ("F B F B", 2),
        ] {
            let events = text
                .split_whitespace()
                .enumerate()
                .map(|(i, mv)| json!({"move":mv,"t":i*3000}))
                .collect::<Vec<_>>();
            assert_eq!(count_moves(&events), expected, "{text}");
        }
        let events = ["R", "L", "R", "L"].map(|mv| json!({"move":mv}));
        assert_eq!(move_weights(&events), vec![0, 0, 1, 1]);
    }
    #[test]
    fn recovered_interior_turn_does_not_estimate_precise_total_endpoints() {
        let mut r = record();
        r["quality"] = json!(["estimated-timing"]);
        r["moves"] = json!([
            {"move":"R","t":0.,"raw":{"cubeTimestamp":5000.}},
            {"move":"U","t":400.,"recovered":true},
            {"move":"R'","t":1000.,"raw":{"cubeTimestamp":6000.}}
        ]);
        let original = r.clone();
        assert!(!execution_metrics(&r).unwrap().estimated);
        assert!(!execution_duration_estimated(&r));
        assert!(turn_timing_estimated(&r, 1));
        assert!(!turn_timing_estimated(&r, 2));
        assert_eq!(r, original, "quality and timestamps remain authoritative");
        r["moves"][2]["raw"]["cubeTimestamp"] = json!(5900.);
        assert!(
            execution_metrics(&r).unwrap().estimated,
            "clamped endpoint is not precise"
        );
        assert!(execution_duration_estimated(&r));
        r["moves"][2]["raw"] = Value::Null;
        assert!(
            execution_metrics(&r).unwrap().estimated,
            "legacy missing evidence remains estimated"
        );
    }
    #[test]
    fn endpoint_estimate_is_visible_even_before_final_state_verification() {
        let mut r = record();
        r["quality"] = json!(["estimated-timing", "unverified-final-state"]);
        r["moves"] = json!([
            {"move":"R","t":0.,"raw":{"cubeTimestamp":5000.}},
            {"move":"U","t":50.,"recovered":true,"raw":{"cubeTimestamp":null,"receivedAt":1200.}}
        ]);
        r["duration"] = json!(50.);
        assert!(execution_metrics(&r).is_none());
        assert!(execution_duration_estimated(&r));
        r["source"] = json!("manual");
        assert!(!execution_duration_estimated(&r));
    }
    #[test]
    fn counts_survive_timing_failure_but_not_incomplete_sequences() {
        let mut r = record();
        r["quality"] = json!(["clock-reset"]);
        assert_eq!(execution_moves(&r), Some(2));
        assert!(execution_metrics(&r).is_none());
        r["quality"] = json!(["missing-moves"]);
        assert_eq!(execution_moves(&r), None);
        r["quality"] = json!(["unverified-final-state"]);
        assert_eq!(execution_moves(&r), None);
    }
    #[test]
    fn averages_use_available_raw_metrics_with_independent_sample_counts() {
        let mut records = (1..=5)
            .map(|n| {
                let mut r = record();
                r["duration"] = json!(n * 1000);
                r["moves"][1]["t"] = json!(n * 1000);
                r
            })
            .collect::<Vec<_>>();
        records[1]["quality"] = json!(["estimated-timing"]);
        records[1]["moves"][1]["recovered"] = json!(true);
        records[2]["quality"] = json!(["clock-reset"]);
        records[3]["source"] = json!("manual");
        let metrics = crate::practice_ui::average_metrics(&records, 5);
        assert_eq!(metrics.time, Some(3000.));
        assert_eq!(metrics.tps, Some(1.));
        assert_eq!(metrics.moves, Some(2.));
        assert!(metrics.tps_estimated);
        assert_eq!(metrics.tps_label(), "≈ 1.00");
        assert_eq!(
            (metrics.tps_samples, metrics.move_samples, metrics.retained),
            (1, 2, 3)
        );
    }
    #[test]
    fn phase_precision_uses_raw_boundaries_after_half_turn_grouping() {
        let mut r = record();
        r["quality"] = json!(["estimated-timing"]);
        r["moves"] = json!([
            {"move":"R","t":0.,"raw":{"cubeTimestamp":5000.}},
            {"move":"R","t":200.,"recovered":true},
            {"move":"U","t":600.,"raw":{"cubeTimestamp":5600.}},
            {"move":"R'","t":1000.,"raw":{"cubeTimestamp":6000.}}
        ]);
        r["analysis"] = json!({"splits":[
            {"stage":"Cross","moves":2,"rawMoves":3},
            {"stage":"F2L","moves":1,"rawMoves":1}
        ]});
        assert!(!split_timing_estimated(&r, "Cross"));
        assert!(!split_timing_estimated(&r, "F2L"));
        r["moves"][2]["recovered"] = json!(true);
        assert!(split_timing_estimated(&r, "Cross"));
        assert!(split_timing_estimated(&r, "F2L"));
    }
    #[test]
    fn execution_rate_survives_missing_analysis_and_scoring_penalties() {
        let mut r = record();
        for penalty in ["", "+2", "DNF"] {
            r["penalty"] = json!(penalty);
            let metric = execution_metrics(&r).unwrap();
            assert_eq!(metric.tps, 2.);
            assert!(!metric.estimated);
        }
        let review = for_review(r);
        assert_eq!(review["analysis"]["moves"], 2);
        assert_eq!(
            review["analysis"]["finalState"],
            crate::cube::Cube::default().to_string()
        );
        assert_eq!(turn_elapsed(&review, 2), 1000.);
        assert_eq!(turn_elapsed(&review, 0), 0.);
    }
    #[test]
    fn recovered_complete_record_has_explicitly_estimated_rate() {
        let mut r = record();
        r["quality"] = json!(["estimated-timing"]);
        r["moves"][1]["recovered"] = json!(true);
        assert_eq!(execution_metrics(&r).unwrap().label(), "≈ 2.00");
        assert!(
            !crate::io::timed(&r),
            "strict statistical eligibility stays separate"
        );
    }
    #[test]
    fn old_analysis_refresh_preserves_raw_record_and_compatible_annotations() {
        let mut r = record();
        r["notes"] = json!("Keep the original practice note");
        r["penalty"] = json!("+2");
        r["quality"] = json!(["estimated-timing"]);
        r["analysis"] = crate::model::analysis(&r);
        let effective = r["analysis"]["cross"].clone();
        r["analysis"]["version"] = json!(1);
        r["analysis"]["cases"] = json!({"PLL":"retained-case"});
        r["analysis"]["splits"][3]["caseId"] = json!("retained-case");
        r["analysis"]["splits"][3]["algorithm"] = json!({"id":"retained-algorithm"});
        r["crossOptimal"] = json!({"distance":2,"moves":["U'","R'"],"cross":effective});
        let before = r.clone();
        let refreshed = refresh_analysis(r);
        assert_eq!(
            refreshed["analysis"]["version"],
            crate::model::ANALYSIS_VERSION
        );
        assert_eq!(refreshed["analysis"]["cases"]["PLL"], "retained-case");
        assert!(refreshed["analysis"]["splits"][3]["algorithm"].is_null());
        for (key, value) in before.as_object().unwrap() {
            if key != "analysis" {
                assert_eq!(&refreshed[key], value, "raw field {key} changed");
            }
        }
    }
    #[test]
    fn new_server_analysis_is_preserved_and_refresh_is_idempotent() {
        let mut r = record();
        r["analysis"] = crate::model::analysis(&r);
        r["analysis"]["serverAnnotations"] = json!({"case":"rich server data"});
        let expected = r.clone();
        let mut records = vec![r];
        assert!(!refresh_records(&mut records));
        assert_eq!(records[0], expected);
        assert_eq!(for_review(records[0].clone()), expected);
    }
    #[test]
    fn detected_face_change_drops_old_phase_cases_and_wrong_cross_optimum() {
        let turns = crate::cube::sequence("R U R' U R U2 R'").unwrap();
        let setup = turns.iter().rev().map(|m| m.inverse()).collect::<Vec<_>>();
        let mut r = json!({
            "source":"bluetooth", "kind":"solve", "status":"complete", "penalty":"",
            "duration":600., "quality":[], "cross":"U", "notes":"preserved",
            "initialState":crate::cube::Cube::default().apply(&setup).to_string(),
            "moves":turns.iter().enumerate().map(|(i,m)| json!({"move":m.to_string(),"t":i as f64*100.})).collect::<Vec<_>>(),
            "analysis":{"version":1,"cross":"U","cases":{"OLL":"wrong-face"},"pairEvents":[{"slot":"wrong-face"}]},
            "crossOptimal":{"distance":3,"moves":["U","R","U'"]}
        });
        let raw = r["moves"].clone();
        let refreshed = refresh_analysis(r.clone());
        assert_eq!(refreshed["analysis"]["cross"], "D");
        assert_ne!(refreshed["analysis"]["cases"]["OLL"], "wrong-face");
        assert_ne!(
            refreshed["analysis"]["pairEvents"],
            r["analysis"]["pairEvents"]
        );
        assert!(refreshed["crossOptimal"].is_null());
        assert_eq!(refreshed["cross"], "U");
        assert_eq!(refreshed["notes"], "preserved");
        assert_eq!(refreshed["moves"], raw);
        r["crossOptimal"]["cross"] = json!("D");
        assert_eq!(
            refresh_analysis(r.clone())["crossOptimal"],
            r["crossOptimal"]
        );
    }
    #[test]
    fn unreliable_or_manual_record_does_not_gain_measured_tps() {
        for flag in [
            "missing-moves",
            "state-mismatch",
            "clock-reset",
            "unverified-final-state",
        ] {
            let mut r = record();
            r["quality"] = json!([flag]);
            assert!(execution_metrics(&r).is_none());
        }
        let mut r = record();
        r["source"] = json!("manual");
        assert!(execution_metrics(&r).is_none());
        let mut r = record();
        r["moves"][1]["t"] = json!(-1.);
        assert!(execution_metrics(&r).is_none());
        let mut r = record();
        r["status"] = json!("interrupted");
        assert!(execution_metrics(&r).is_none());
    }
}

/// Prefer the solve's detected face over a mutable session preference.
pub fn reconstruction_cross(record: &Value) -> u8 {
    record["analysis"]["cross"]
        .as_str()
        .filter(|s| s.len() == 1 && b"URFDLB".contains(&s.as_bytes()[0]))
        .or_else(|| {
            record["cross"]
                .as_str()
                .filter(|s| s.len() == 1 && b"URFDLB".contains(&s.as_bytes()[0]))
        })
        .unwrap_or("D")
        .as_bytes()[0]
}
#[cfg(test)]
mod reconstruction_face_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn detected_yellow_cross_overrides_the_white_session_setting() {
        assert_eq!(
            reconstruction_cross(&json!({"cross":"U","analysis":{"cross":"D"}})),
            b'D'
        );
        assert_eq!(
            reconstruction_cross(&json!({"cross":"U","analysis":{"cross":""}})),
            b'U'
        );
    }
}

/// Exact event-count boundaries keep equal packet timestamps in the right stage.
pub fn reconstruction_ranges(record: &Value) -> Vec<(String, usize, usize)> {
    let count = record["moves"].as_array().map_or(0, Vec::len);
    let mut cursor = 0;
    let mut ranges = Vec::new();
    for split in record["analysis"]["splits"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(n) = split["rawMoves"].as_u64() else {
            return vec![("Recorded".into(), 0, count)];
        };
        let end = cursor + n as usize;
        if end > count {
            return vec![("Recorded".into(), 0, count)];
        }
        ranges.push((value_str(split, "stage").to_owned(), cursor, end));
        cursor = end;
    }
    if cursor != count || ranges.is_empty() {
        vec![("Recorded".into(), 0, count)]
    } else {
        ranges
    }
}

#[cfg(test)]
mod reconstruction_range_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn grouping_uses_raw_counts_even_when_timestamps_are_equal() {
        let record = json!({"moves":[{"t":0},{"t":0},{"t":0}],"analysis":{"splits":[{"stage":"Cross","rawMoves":2,"moves":1},{"stage":"F2L","rawMoves":1,"moves":1},{"stage":"OLL","rawMoves":0,"moves":0}]}});
        assert_eq!(
            reconstruction_ranges(&record),
            vec![
                ("Cross".into(), 0, 2),
                ("F2L".into(), 2, 3),
                ("OLL".into(), 3, 3)
            ]
        );
        assert_eq!(
            reconstruction_ranges(&json!({"moves":[{}]})),
            vec![("Recorded".into(), 0, 1)]
        );
    }
}

/// Keep local gyro evidence when an older server omits optional replay fields.
/// Never attach it to changed face-turn evidence or replace a complete remote frame.
pub(crate) fn retain_replay_metadata(mut remote: Value, local: &Value) -> Value {
    if remote["id"] != local["id"]
        || remote["initialState"] != local["initialState"]
        || remote["duration"]
            .as_f64()
            .zip(local["duration"].as_f64())
            .is_none_or(|(a, b)| a != b)
    {
        return remote;
    }
    let same_moves = remote["moves"]
        .as_array()
        .zip(local["moves"].as_array())
        .is_some_and(|(a, b)| {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a["move"] == b["move"]
                        && a["t"]
                            .as_f64()
                            .zip(b["t"].as_f64())
                            .is_some_and(|(a, b)| a == b)
                })
        });
    if !same_moves
        || remote["rotationFrame"]
            .as_str()
            .is_some_and(|f| f != "live-v1")
    {
        return remote;
    }
    if let Some(start) = crate::render::orientation::initial_live_pose(&remote) {
        let same_start =
            crate::render::orientation::initial_live_pose(local).is_some_and(|other| {
                start
                    .iter()
                    .zip(other)
                    .map(|(a, b)| a * b)
                    .sum::<f32>()
                    .abs()
                    > 0.99999
            });
        if remote["rotations"].as_array().is_none_or(Vec::is_empty) && same_start {
            if let Some(poses) = local.get("rotations") {
                remote["rotations"] = poses.clone();
            }
        }
        return remote;
    }
    let same_rotations = remote["rotations"]
        .as_array()
        .zip(local["rotations"].as_array())
        .is_some_and(|(a, b)| {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a["notation"] == b["notation"]
                        && a["t"]
                            .as_f64()
                            .zip(b["t"].as_f64())
                            .is_some_and(|(a, b)| a == b)
                        && a["pose"]
                            .as_array()
                            .zip(b["pose"].as_array())
                            .is_some_and(|(a, b)| {
                                a.len() == 4
                                    && b.len() == 4
                                    && a.iter().zip(b).all(|(a, b)| {
                                        a.as_f64().zip(b.as_f64()).is_some_and(|(a, b)| a == b)
                                    })
                            })
                })
        });
    if remote["rotations"]
        .as_array()
        .is_some_and(|v| !v.is_empty())
        && !same_rotations
    {
        return remote;
    }
    for key in ["rotations", "rotationFrame", "initialOrientation"] {
        if let Some(value) = local.get(key).filter(|v| !v.is_null()) {
            remote[key] = value.clone();
        }
    }
    remote
}

#[cfg(test)]
mod replay_metadata_sync_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn sync_keeps_matching_gyro_evidence_but_not_for_a_changed_solve() {
        let local = json!({"id":"pose","initialState":"state","duration":1000.,"moves":[{"move":"R","t":0.},{"move":"U","t":1000.}],"rotationFrame":"live-v1","initialOrientation":[0.,1.,0.,0.],"rotations":[{"notation":"x","t":500.,"pose":[0.,0.,0.,1.]}]});
        let mut remote = local.clone();
        for key in ["rotations", "rotationFrame", "initialOrientation"] {
            remote.as_object_mut().unwrap().remove(key);
        }
        remote["penalty"] = json!("+2");
        // JavaScript APIs serialize 0.0 / 1000.0 as integer JSON numbers.
        remote["duration"] = json!(1000);
        remote["moves"][0]["t"] = json!(0);
        remote["moves"][1]["t"] = json!(1000);
        let merged = retain_replay_metadata(remote.clone(), &local);
        assert_eq!(merged["initialOrientation"], local["initialOrientation"]);
        assert_eq!(merged["rotations"], local["rotations"]);
        assert_eq!(merged["penalty"], "+2");
        remote["moves"][0]["move"] = json!("L");
        assert!(
            retain_replay_metadata(remote, &local)
                .get("rotations")
                .is_none()
        );
        let mut partial = local.clone();
        for key in ["rotationFrame", "initialOrientation"] {
            partial.as_object_mut().unwrap().remove(key);
        }
        partial["rotations"][0]["pose"] = json!([0, 0, 0, 1]);
        partial["rotations"][0]["t"] = json!(500);
        assert_eq!(
            retain_replay_metadata(partial, &local)["rotationFrame"],
            "live-v1"
        );
        let mut no_poses = local.clone();
        no_poses["rotations"] = json!([]);
        assert_eq!(
            retain_replay_metadata(no_poses, &local)["rotations"],
            local["rotations"]
        );
        let mut rich = local.clone();
        rich["initialOrientation"] = json!([0., 0., 0., 1.]);
        assert_eq!(retain_replay_metadata(rich.clone(), &local), rich);
    }
}
