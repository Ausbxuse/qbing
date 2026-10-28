//! Discrete reconstruction positions preserve grouped turns and gyro-only events.
use crate::{notation, recording};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Position {
    pub raw_index: usize,
    pub time: f64,
}
pub(crate) fn positions(record: &Value, raw: bool) -> Vec<Position> {
    let turns = if raw {
        notation::raw_turns(record)
    } else {
        notation::reconstruct_for_view(record)
    };
    let mut positions = vec![Position {
        raw_index: 0,
        time: 0.,
    }];
    positions.extend(
        notation::with_rotations(turns, record)
            .into_iter()
            .map(|t| Position {
                raw_index: t.raw_end,
                time: t
                    .elapsed
                    .unwrap_or_else(|| recording::turn_elapsed(record, t.raw_end))
                    .max(0.),
            }),
    );
    positions
}
pub(crate) fn adjacent(
    positions: &[Position],
    current: Position,
    cursor: Option<usize>,
    forward: bool,
) -> usize {
    let cursor = cursor.filter(|&i| positions.get(i) == Some(&current));
    if let Some(i) = cursor {
        return if forward {
            (i + 1).min(positions.len() - 1)
        } else {
            i.saturating_sub(1)
        };
    }
    let before = |p: &Position| {
        p.time < current.time || (p.time == current.time && p.raw_index < current.raw_index)
    };
    let after = |p: &Position| {
        p.time > current.time || (p.time == current.time && p.raw_index > current.raw_index)
    };
    if forward {
        positions
            .iter()
            .skip(1)
            .position(after)
            .map(|i| i + 1)
            .unwrap_or(positions.len() - 1)
    } else {
        positions.iter().rposition(before).unwrap_or(0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn grouped_turns_and_rotations_step_reversibly_without_skipping_zero_time_events() {
        let r = json!({"cross":"D","moves":[{"move":"R","t":0},{"move":"L'","t":60},{"move":"R","t":120},{"move":"L'","t":180},{"move":"U","t":400}],"rotations":[{"notation":"y","t":250},{"notation":"z","t":250}]});
        let p = positions(&r, false);
        assert_eq!(
            p.iter().map(|p| p.raw_index).collect::<Vec<_>>(),
            vec![0, 4, 4, 4, 5]
        );
        assert_eq!(
            p.iter().map(|p| p.time).collect::<Vec<_>>(),
            vec![0., 180., 250., 250., 400.]
        );
        let mut cursor = 0;
        for expected in 1..p.len() {
            cursor = adjacent(&p, p[cursor], Some(cursor), true);
            assert_eq!(cursor, expected);
        }
        for expected in (0..p.len() - 1).rev() {
            cursor = adjacent(&p, p[cursor], Some(cursor), false);
            assert_eq!(cursor, expected);
        }
        assert_eq!(positions(&r, true).len(), 8);
        let zero = positions(
            &json!({"moves":[{"move":"R","t":0},{"move":"U","t":0}]}),
            true,
        );
        assert_eq!(adjacent(&zero, zero[0], None, true), 1);
        assert_eq!(adjacent(&zero, zero[1], None, false), 0);
        assert_eq!(
            adjacent(
                &p,
                Position {
                    raw_index: 4,
                    time: 300.
                },
                None,
                false
            ),
            3
        );
    }
}
