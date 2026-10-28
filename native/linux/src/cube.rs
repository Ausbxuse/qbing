use anyhow::{Result, bail, ensure};
use std::{fmt, sync::OnceLock};

pub const FACES: &[u8; 6] = b"URFDLB";
pub type Cubies = (Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>);
pub const CORNERS: [[usize; 3]; 8] = [
    [8, 9, 20],
    [6, 18, 38],
    [0, 36, 47],
    [2, 45, 11],
    [29, 26, 15],
    [27, 44, 24],
    [33, 53, 42],
    [35, 17, 51],
];
pub const EDGES: [[usize; 2]; 12] = [
    [5, 10],
    [7, 19],
    [3, 37],
    [1, 46],
    [32, 16],
    [28, 25],
    [30, 43],
    [34, 52],
    [23, 12],
    [21, 41],
    [50, 39],
    [48, 14],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cube(pub [u8; 54]);
impl Default for Cube {
    fn default() -> Self {
        Self(std::array::from_fn(|i| FACES[i / 9]))
    }
}
impl fmt::Display for Cube {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub face: u8,
    pub turns: i8,
}
impl Move {
    pub fn parse(s: &str) -> Result<Self> {
        let b = s.as_bytes();
        ensure!(!b.is_empty() && FACES.contains(&b[0]), "Invalid move {s}");
        let turns = match &b[1..] {
            b"" => 1,
            b"'" => -1,
            b"2" => 2,
            _ => bail!("Invalid move {s}"),
        };
        Ok(Self { face: b[0], turns })
    }
    pub fn inverse(self) -> Self {
        Self {
            turns: if self.turns == 2 { 2 } else { -self.turns },
            ..self
        }
    }
    pub fn axis(self) -> (usize, f32, f32) {
        match self.face {
            b'R' => (0, 1., -1.),
            b'L' => (0, -1., 1.),
            b'U' => (1, 1., -1.),
            b'D' => (1, -1., 1.),
            b'F' => (2, 1., -1.),
            _ => (2, -1., 1.),
        }
    }
}
impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            self.face as char,
            match self.turns {
                -1 => "'",
                2 => "2",
                _ => "",
            }
        )
    }
}
pub fn sequence(s: &str) -> Result<Vec<Move>> {
    s.split_whitespace().map(Move::parse).collect()
}

#[derive(Clone, Copy)]
pub struct Sticker {
    pub p: [f32; 3],
    pub n: [f32; 3],
}
pub fn geometry() -> &'static [Sticker; 54] {
    static GEO: OnceLock<[Sticker; 54]> = OnceLock::new();
    GEO.get_or_init(|| {
        std::array::from_fn(|i| {
            let f = i / 9;
            let r = (i % 9 / 3) as f32;
            let c = (i % 3) as f32;
            let p = match f {
                0 => [c - 1., 1., r - 1.],
                1 => [1., 1. - r, 1. - c],
                2 => [c - 1., 1. - r, 1.],
                3 => [c - 1., -1., 1. - r],
                4 => [-1., 1. - r, c - 1.],
                _ => [1. - c, 1. - r, -1.],
            };
            Sticker {
                p,
                n: [
                    [0., 1., 0.],
                    [1., 0., 0.],
                    [0., 0., 1.],
                    [0., -1., 0.],
                    [-1., 0., 0.],
                    [0., 0., -1.],
                ][f],
            }
        })
    })
}
pub fn rotate(p: [f32; 3], axis: usize, angle: f32) -> [f32; 3] {
    let (c, s) = (angle.cos(), angle.sin());
    match axis {
        0 => [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c],
        1 => [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c],
        _ => [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]],
    }
}
impl Cube {
    pub fn parse(s: &str) -> Result<Self> {
        ensure!(s.len() == 54, "Expected 54 stickers");
        let mut a = [0; 54];
        a.copy_from_slice(s.as_bytes());
        let cube = Self(a);
        ensure!(cube.valid(), "Invalid cube state");
        Ok(cube)
    }
    pub fn turn(self, m: Move) -> Self {
        static PERM: OnceLock<[[usize; 54]; 6]> = OnceLock::new();
        let perms = PERM.get_or_init(|| {
            std::array::from_fn(|f| {
                let (a, l, d) = Move {
                    face: FACES[f],
                    turns: 1,
                }
                .axis();
                let mut p = std::array::from_fn(|i| i);
                for (i, g) in geometry().iter().enumerate() {
                    if (g.p[a] - l).abs() < 0.1 {
                        let pos =
                            rotate(g.p, a, d * std::f32::consts::FRAC_PI_2).map(|v| v.round());
                        let n = rotate(g.n, a, d * std::f32::consts::FRAC_PI_2).map(|v| v.round());
                        let j = geometry()
                            .iter()
                            .position(|x| x.p == pos && x.n == n)
                            .unwrap();
                        p[j] = i;
                    }
                }
                p
            })
        });
        let p = &perms[FACES.iter().position(|f| *f == m.face).unwrap()];
        let mut s = self;
        for _ in 0..m.turns.rem_euclid(4) {
            s = Self(std::array::from_fn(|i| s.0[p[i]]));
        }
        s
    }
    pub fn apply(self, moves: &[Move]) -> Self {
        moves.iter().fold(self, |s, m| s.turn(*m))
    }
    pub fn progress(self, cross: u8) -> [bool; 4] {
        let solved = Self::default();
        let cross_done = EDGES
            .iter()
            .filter(|e| e.iter().any(|i| solved.0[*i] == cross))
            .all(|e| e.iter().all(|i| self.0[*i] == solved.0[*i]));
        let axis = geometry()[FACES.iter().position(|f| *f == cross).unwrap() * 9 + 4].n;
        let cross_axis = axis.iter().position(|n| *n != 0.).unwrap();
        let f2l = cross_done
            && geometry()
                .iter()
                .enumerate()
                .filter(|(_, g)| g.p[cross_axis] * axis[cross_axis] > -0.5)
                .all(|(i, _)| self.0[i] == solved.0[i]);
        let top = match cross {
            b'U' => b'D',
            b'D' => b'U',
            b'R' => b'L',
            b'L' => b'R',
            b'F' => b'B',
            _ => b'F',
        };
        let start = FACES.iter().position(|f| *f == top).unwrap() * 9;
        [
            cross_done,
            f2l,
            f2l && self.0[start..start + 9].iter().all(|c| *c == top),
            self == solved,
        ]
    }
    pub fn goal(self, stage: &str, cross: u8) -> bool {
        self.progress(cross)[match stage {
            "Cross" => 0,
            "F2L" => 1,
            "OLL" => 2,
            _ => 3,
        }]
    }
    pub fn cubies(self) -> Result<Cubies> {
        let solved = Self::default();
        let (mut cp, mut co, mut ep, mut eo) = (vec![], vec![], vec![], vec![]);
        for ids in CORNERS {
            let colors = ids.map(|i| self.0[i]);
            let o = colors
                .iter()
                .position(|c| *c == b'U' || *c == b'D')
                .ok_or_else(|| anyhow::anyhow!("Corner orientation"))?;
            let p = CORNERS
                .iter()
                .position(|home| (0..3).all(|i| solved.0[home[i]] == colors[(i + o) % 3]))
                .ok_or_else(|| anyhow::anyhow!("Corner identity"))?;
            cp.push(p);
            co.push(o);
        }
        for ids in EDGES {
            let colors = ids.map(|i| self.0[i]);
            let (p, o) = (0..12)
                .flat_map(|p| (0..2).map(move |o| (p, o)))
                .find(|(p, o)| (0..2).all(|i| solved.0[EDGES[*p][i]] == colors[(i + o) % 2]))
                .ok_or_else(|| anyhow::anyhow!("Edge identity"))?;
            ep.push(p);
            eo.push(o);
        }
        Ok((cp, co, ep, eo))
    }
    pub fn valid(self) -> bool {
        if FACES.iter().enumerate().any(|(i, f)| {
            self.0[i * 9 + 4] != *f || self.0.iter().filter(|c| **c == *f).count() != 9
        }) {
            return false;
        }
        let Ok((cp, co, ep, eo)) = self.cubies() else {
            return false;
        };
        let parity = |a: &Vec<usize>| {
            a.iter()
                .enumerate()
                .map(|(i, v)| a[i + 1..].iter().filter(|x| *x < v).count())
                .sum::<usize>()
                % 2
        };
        let mut c = cp.clone();
        c.sort();
        let mut e = ep.clone();
        e.sort();
        c == (0..8).collect::<Vec<_>>()
            && e == (0..12).collect::<Vec<_>>()
            && co.iter().sum::<usize>() % 3 == 0
            && eo.iter().sum::<usize>() % 2 == 0
            && parity(&cp) == parity(&ep)
    }
    pub fn from_cubies(cp: &[usize], co: &[usize], ep: &[usize], eo: &[usize]) -> Result<Self> {
        ensure!(
            cp.len() == 8 && co.len() == 8 && ep.len() == 12 && eo.len() == 12,
            "Cubie length"
        );
        let mut s = Self::default();
        let home = s;
        for i in 0..8 {
            ensure!(cp[i] < 8 && co[i] < 3, "Corner value");
            for j in 0..3 {
                s.0[CORNERS[i][(j + co[i]) % 3]] = home.0[CORNERS[cp[i]][j]];
            }
        }
        for i in 0..12 {
            ensure!(ep[i] < 12 && eo[i] < 2, "Edge value");
            for j in 0..2 {
                s.0[EDGES[i][(j + eo[i]) % 2]] = home.0[EDGES[ep[i]][j]];
            }
        }
        ensure!(s.valid(), "Illegal cube state");
        Ok(s)
    }
}

/// Matches physical quarter turns to a scramble. Opposite faces commute, so each
/// uninterrupted axis block may be performed in either order (or interleaved
/// while completing half turns). Order on an individual face is preserved.
#[derive(Debug)]
pub struct ScrambleProgress {
    /// First unfinished notation token, retained for callers that need a cursor.
    pub completed: usize,
    pub partial: bool,
    pub matched: bool,
    pub done: Vec<usize>,
    pub partial_moves: Vec<usize>,
    pub next_moves: Vec<usize>,
    /// Quarter turns that complete only the half turns already in progress.
    pub finish: Vec<Move>,
}

#[derive(Default)]
struct FaceProgress {
    turns: i8,
    done: Vec<usize>,
    partial: Option<(usize, Move)>,
}

fn face_progress(moves: &[Move], start: usize, end: usize, face: u8) -> Vec<FaceProgress> {
    let mut options = vec![FaceProgress::default()];
    let mut turns: i8 = 0;
    let mut done = vec![];
    for (i, mv) in moves.iter().enumerate().take(end).skip(start) {
        if mv.face != face {
            continue;
        }
        if mv.turns == 2 {
            for quarter in [1, -1] {
                options.push(FaceProgress {
                    turns: (turns + quarter).rem_euclid(4),
                    done: done.clone(),
                    partial: Some((
                        i,
                        Move {
                            face,
                            turns: quarter,
                        },
                    )),
                });
            }
        }
        turns = (turns + mv.turns).rem_euclid(4);
        done.push(i);
        options.push(FaceProgress {
            turns,
            done: done.clone(),
            partial: None,
        });
    }
    options
}

pub fn scramble_progress(cube: Cube, moves: &[Move]) -> ScrambleProgress {
    let mut found = ScrambleProgress {
        completed: 0,
        partial: false,
        matched: cube == Cube::default(),
        done: vec![],
        partial_moves: vec![],
        next_moves: vec![],
        finish: vec![],
    };
    let mut position = Cube::default();
    let mut start = 0;
    let mut best_score = 0;
    while start < moves.len() {
        let axis = moves[start].axis().0;
        let end = (start..moves.len())
            .find(|&i| moves[i].axis().0 != axis)
            .unwrap_or(moves.len());
        let faces = match axis {
            0 => [b'R', b'L'],
            1 => [b'U', b'D'],
            _ => [b'F', b'B'],
        };
        let a = face_progress(moves, start, end, faces[0]);
        let b = face_progress(moves, start, end, faces[1]);
        for first in &a {
            let left = if first.turns == 0 {
                position
            } else {
                position.turn(Move {
                    face: faces[0],
                    turns: first.turns,
                })
            };
            for second in &b {
                let candidate = if second.turns == 0 {
                    left
                } else {
                    left.turn(Move {
                        face: faces[1],
                        turns: second.turns,
                    })
                };
                if candidate != cube {
                    continue;
                }
                let score = (start + first.done.len() + second.done.len()) * 2
                    + usize::from(first.partial.is_some())
                    + usize::from(second.partial.is_some());
                if found.matched && score < best_score {
                    continue;
                }
                best_score = score;
                found.matched = true;
                found.done = (0..start)
                    .chain(first.done.iter().copied())
                    .chain(second.done.iter().copied())
                    .collect();
                found.done.sort_unstable();
                let mut partials: Vec<_> =
                    first.partial.into_iter().chain(second.partial).collect();
                partials.sort_by_key(|(i, _)| *i);
                found.partial_moves = partials.iter().map(|(i, _)| *i).collect();
                found.finish = partials.iter().map(|(_, mv)| *mv).collect();
            }
        }
        position = position.apply(&moves[start..end]);
        start = end;
    }
    found.completed = (0..moves.len())
        .find(|i| !found.done.contains(i))
        .unwrap_or(moves.len());
    found.partial = !found.partial_moves.is_empty();
    if found.matched && found.completed < moves.len() {
        let axis = moves[found.completed].axis().0;
        let mut faces = vec![];
        for (i, mv) in moves.iter().enumerate().skip(found.completed) {
            if mv.axis().0 != axis {
                break;
            }
            if !found.done.contains(&i) && !faces.contains(&mv.face) {
                found.next_moves.push(i);
                faces.push(mv.face);
            }
        }
    }
    found
}
