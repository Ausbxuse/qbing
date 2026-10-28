//! Display the original scramble and splice repairs at its retained cursor.
//! This is presentation only: the original algorithm and physical state stay intact.
use crate::cube::{Cube, Move, scramble_progress};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Origin {
    Scramble(usize),
    Correction(usize),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Token {
    pub mv: Move,
    pub origin: Origin,
    pub done: bool,
    pub current: bool,
    pub partial: bool,
    pub allowed: bool,
}

pub(super) fn tokens(
    setup: &[Move],
    actual: Option<Cube>,
    checkpoint: Option<Cube>,
    correction: &[Move],
) -> Vec<Token> {
    let actual = actual.map(|cube| scramble_progress(cube, setup));
    let on_track = actual.as_ref().is_some_and(|progress| progress.matched);
    let progress = if on_track {
        actual
    } else {
        checkpoint
            .map(|cube| scramble_progress(cube, setup))
            .filter(|progress| progress.matched)
    };
    let fixing = !on_track && !correction.is_empty();
    let cursor = progress.as_ref().map(|p| p.completed).unwrap_or(0);
    let mut result = Vec::with_capacity(setup.len() + if fixing { correction.len() } else { 0 });
    for index in 0..=setup.len() {
        if fixing && index == cursor {
            result.extend(correction.iter().enumerate().map(|(index, &mv)| Token {
                mv,
                origin: Origin::Correction(index),
                done: false,
                current: index == 0,
                partial: false,
                allowed: index == 0,
            }));
        }
        if let Some(&mv) = setup.get(index) {
            result.push(Token {
                mv,
                origin: Origin::Scramble(index),
                done: progress.as_ref().is_some_and(|p| p.done.contains(&index)),
                current: !fixing && progress.as_ref().is_some_and(|p| p.completed == index),
                partial: progress
                    .as_ref()
                    .is_some_and(|p| p.partial_moves.contains(&index)),
                allowed: !fixing
                    && progress
                        .as_ref()
                        .is_some_and(|p| p.next_moves.contains(&index)),
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::sequence;

    #[test]
    fn repairs_are_inserted_without_losing_original_partial_or_commuting_progress() {
        let setup = sequence("R U2 D F").unwrap();
        let checkpoint = Cube::default().apply(&sequence("R D U").unwrap());
        let actual = checkpoint.turn(Move::parse("B").unwrap());
        let correction = sequence("B'").unwrap();
        let result = tokens(&setup, Some(actual), Some(checkpoint), &correction);
        assert_eq!(
            result.iter().map(|t| t.mv.to_string()).collect::<Vec<_>>(),
            ["R", "B'", "U2", "D", "F"]
        );
        assert!(
            result[0].done && result[3].done,
            "out-of-order opposite face remains completed"
        );
        assert!(
            result[2].partial,
            "a partial half turn remains on its original token"
        );
        assert!(result[1].current && !result[2].current);
        assert_eq!(
            result
                .iter()
                .filter_map(|t| match t.origin {
                    Origin::Scramble(i) => Some(i),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        let resumed = tokens(&setup, Some(checkpoint), Some(checkpoint), &[]);
        assert_eq!(resumed.len(), setup.len());
        assert!(resumed[1].current && resumed[1].partial);
    }

    #[test]
    fn a_legal_opposite_face_is_available_without_two_primary_cursors() {
        let setup = sequence("R U2 D2 F").unwrap();
        let state = Cube::default().turn(setup[0]);
        let result = tokens(&setup, Some(state), None, &[]);
        assert!(result[1].current && result[1].allowed);
        assert!(!result[2].current && result[2].allowed);
        assert_eq!(result.iter().filter(|t| t.current).count(), 1);
    }

    #[test]
    fn repaired_state_drops_stale_correction_and_full_checkpoint_appends_at_end() {
        let setup = sequence("R U").unwrap();
        let state = Cube::default().apply(&setup);
        let undo = sequence("F'").unwrap();
        let off_track = tokens(
            &setup,
            Some(state.turn(Move::parse("F").unwrap())),
            Some(state),
            &undo,
        );
        assert_eq!(off_track.last().unwrap().origin, Origin::Correction(0));
        assert!(off_track[..2].iter().all(|t| t.done));
        let restored = tokens(&setup, Some(state), Some(state), &undo);
        assert_eq!(restored.len(), 2);
        assert!(restored.iter().all(|t| t.done));
    }
}
