//! Resolve provisional recovery times inside an active recording only.
use serde_json::{Value, json};

// GAN history contains move order but no move timestamps. csTimer uses this
// same nominal spacing for missing endpoint timestamps. It is an estimate,
// never a hardware measurement, and cannot reconstruct a pause in lost data.
const RECOVERED_TURN_MS: f64 = 50.;

/// A history response's arrival is unrelated to when its turns happened. Use
/// a provisional elapsed time until a genuine following clock can bracket it.
pub fn estimated_recovered_elapsed(events: &[Value]) -> f64 {
    events
        .last()
        .and_then(|event| event["t"].as_f64())
        .filter(|elapsed| elapsed.is_finite() && *elapsed >= 0.)
        .map(|elapsed| elapsed + RECOVERED_TURN_MS)
        .unwrap_or(0.)
}

/// When the first turns were recovered, the first genuine device clock can
/// anchor an estimated origin. Raw first-turn clocks remain absent, keeping
/// total timing explicitly estimated even after genuine timestamps resume.
pub fn estimated_origin(events: &[Value], next: Option<f64>) -> Option<f64> {
    let next = next.filter(|clock| clock.is_finite() && *clock >= 0.)?;
    if events.is_empty()
        || !events
            .iter()
            .all(|event| event["recovered"] == true && event["raw"]["cubeTimestamp"].is_null())
    {
        return None;
    }
    Some(next - RECOVERED_TURN_MS * events.len() as f64)
}

/// Bluetooth history has no device clock. Once the next live turn arrives,
/// interpolate only the trailing recovered elapsed times between trusted
/// anchors. Raw receive/device timestamps and recovery flags stay untouched.
pub fn settle_recovered(events: &mut [Value], origin: Option<f64>, next: Option<f64>) {
    let Some((origin, next)) = origin.zip(next) else {
        return;
    };
    if !origin.is_finite() || !next.is_finite() || next < origin {
        return;
    }
    if events.last().is_none_or(|event| event["recovered"] != true) {
        return;
    }
    let Some(anchor) = events.iter().rposition(|event| event["recovered"] != true) else {
        return;
    };
    let Some((clock, start)) = events[anchor]["raw"]["cubeTimestamp"]
        .as_f64()
        .zip(events[anchor]["t"].as_f64())
    else {
        return;
    };
    if !clock.is_finite()
        || !start.is_finite()
        || clock < origin
        || next < clock
        || (clock - origin - start).abs() > 1.
    {
        return;
    }
    let end = next - origin;
    let steps = events.len() - anchor;
    for (index, event) in events.iter_mut().enumerate().skip(anchor + 1) {
        event["t"] = json!(start + (end - start) * (index - anchor) as f64 / steps as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn turn(t: f64, clock: Option<f64>, received: f64, recovered: bool) -> Value {
        json!({"move":"R","t":t,"recovered":recovered,"raw":{"cubeTimestamp":clock,"receivedAt":received,"serial":7}})
    }
    #[test]
    fn delayed_history_does_not_push_a_trusted_endpoint_forward() {
        let mut events = vec![
            turn(0., Some(1000.), 0., false),
            turn(200., Some(1200.), 210., false),
            turn(550., None, 550., true),
        ];
        let before = events.clone();
        settle_recovered(&mut events, Some(1000.), Some(1400.));
        assert_eq!(events[2]["t"], 300.);
        for (before, after) in before.iter().zip(&events) {
            assert_eq!(before["raw"], after["raw"]);
            assert_eq!(before["recovered"], after["recovered"]);
        }
        assert_eq!(&events[..2], &before[..2]);
    }
    #[test]
    fn multiple_recovered_turns_stay_between_their_real_anchors() {
        let mut events = vec![
            turn(200., Some(1200.), 210., false),
            turn(700., None, 700., true),
            turn(700., None, 700., true),
        ];
        settle_recovered(&mut events, Some(1000.), Some(1500.));
        assert_eq!(
            events
                .iter()
                .map(|event| event["t"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            [200., 300., 400.]
        );
    }
    #[test]
    fn history_arrival_never_sets_a_missing_endpoint_time() {
        for received in [300., 1200., 20_000.] {
            let mut events = vec![
                turn(0., Some(1000.), 0., false),
                turn(200., Some(1200.), 210., false),
            ];
            for expected in [250., 300.] {
                let elapsed = estimated_recovered_elapsed(&events);
                assert_eq!(elapsed, expected);
                events.push(turn(elapsed, None, received, true));
            }
            assert_eq!(events.last().unwrap()["t"], 300.);
            assert_eq!(events.last().unwrap()["raw"]["receivedAt"], received);
        }
    }
    #[test]
    fn genuine_endpoint_replaces_an_overshooting_nominal_recovery_estimate() {
        let mut events = vec![
            turn(0., Some(1000.), 0., false),
            turn(200., Some(1200.), 210., false),
            turn(250., None, 1100., true),
            turn(300., None, 1100., true),
        ];
        settle_recovered(&mut events, Some(1000.), Some(1230.));
        assert_eq!(events[2]["t"], 210.);
        assert_eq!(events[3]["t"], 220.);
        assert_eq!(events[3]["raw"]["receivedAt"], 1100.);
    }
    #[test]
    fn unknown_first_clock_keeps_an_estimated_origin_and_raw_evidence() {
        let events = vec![turn(0., None, 0., true), turn(50., None, 0., true)];
        let before = events.clone();
        assert_eq!(estimated_origin(&events, Some(1100.)), Some(1000.));
        assert_eq!(events, before);
        assert_eq!(estimated_origin(&events, None), None);
        assert_eq!(estimated_origin(&events, Some(f64::NAN)), None);
        assert_eq!(estimated_origin(&[], Some(1100.)), None);
        assert_eq!(estimated_recovered_elapsed(&[]), 0.);
        let direct = vec![turn(0., Some(1000.), 0., false)];
        assert_eq!(estimated_origin(&direct, Some(1100.)), None);
    }
    #[test]
    fn absent_or_inconsistent_anchors_never_manufacture_precision() {
        for (origin, next, anchor) in [
            (None, Some(1400.), Some(1200.)),
            (Some(1000.), Some(1100.), Some(1200.)),
            (Some(1000.), Some(1400.), None),
            (Some(1000.), Some(f64::NAN), Some(1200.)),
        ] {
            let mut events = vec![
                turn(200., anchor, 210., false),
                turn(550., None, 550., true),
            ];
            let before = events.clone();
            settle_recovered(&mut events, origin, next);
            assert_eq!(events, before);
        }
        let mut events = vec![turn(550., None, 550., true)];
        let before = events.clone();
        settle_recovered(&mut events, Some(1000.), Some(1400.));
        assert_eq!(events, before);
    }
}
