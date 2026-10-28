// GAN Gen4 protocol derived from gan-web-bluetooth 3.0.2 (MIT).
use crate::cube::{Cube, Move};
use aes::{
    Aes128,
    cipher::{BlockDecrypt, BlockEncrypt, KeyInit, generic_array::GenericArray},
};
use anyhow::{Result, bail, ensure};
use std::{
    collections::{BTreeMap, VecDeque},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Cipher {
    key: [u8; 16],
    iv: [u8; 16],
}
impl Cipher {
    pub fn new(mac: &str) -> Result<Self> {
        let salt = mac
            .split(':')
            .map(|s| u8::from_str_radix(s, 16))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(salt.len() == 6, "Enter a valid cube MAC address");
        let mut key = [
            1, 2, 0x42, 0x28, 0x31, 0x91, 0x16, 7, 0x20, 5, 0x18, 0x54, 0x42, 0x11, 0x12, 0x53,
        ];
        let mut iv = [
            0x11, 3, 0x32, 0x28, 0x21, 1, 0x76, 0x27, 0x20, 0x95, 0x78, 0x14, 0x32, 0x12, 2, 0x43,
        ];
        for i in 0..6 {
            key[i] = ((key[i] as u16 + salt[5 - i] as u16) % 255) as u8;
            iv[i] = ((iv[i] as u16 + salt[5 - i] as u16) % 255) as u8;
        }
        Ok(Self { key, iv })
    }
    pub fn transform(&self, input: &[u8], encrypt: bool) -> Result<Vec<u8>> {
        ensure!((16..=128).contains(&input.len()), "Invalid packet length");
        let mut out = input.to_vec();
        let aes = Aes128::new(GenericArray::from_slice(&self.key));
        let offsets = if input.len() == 16 {
            vec![0]
        } else if encrypt {
            vec![0, input.len() - 16]
        } else {
            vec![input.len() - 16, 0]
        };
        for offset in offsets {
            let mut block = GenericArray::clone_from_slice(&out[offset..offset + 16]);
            if encrypt {
                for (i, b) in block.iter_mut().enumerate() {
                    *b ^= self.iv[i];
                }
                aes.encrypt_block(&mut block);
            } else {
                aes.decrypt_block(&mut block);
                for (i, b) in block.iter_mut().enumerate() {
                    *b ^= self.iv[i];
                }
            }
            out[offset..offset + 16].copy_from_slice(&block);
        }
        Ok(out)
    }
    pub fn command(&self, name: &str) -> Vec<u8> {
        let prefix: &[u8] = match name {
            "state" => &[0xdd, 4, 0, 0xed, 0, 0],
            "battery" => &[0xdd, 4, 0, 0xef, 0, 0],
            "reset" => &[
                0xd2, 13, 5, 0x39, 0x77, 0, 0, 1, 0x23, 0x45, 0x67, 0x89, 0xab, 0, 0, 0,
            ],
            _ => &[0xdf, 3, 0, 0, 0],
        };
        let mut b = [0u8; 20];
        b[..prefix.len()].copy_from_slice(prefix);
        self.transform(&b, true).unwrap()
    }
    pub fn history(&self, serial: u8, count: u8) -> Vec<u8> {
        let start = if serial.is_multiple_of(2) {
            serial.wrapping_sub(1)
        } else {
            serial
        };
        let count = (if count.is_multiple_of(2) {
            count as u16
        } else {
            count as u16 + 1
        })
        .min(start as u16 + 1)
        .min(16) as u8;
        let mut b = [0; 20];
        b[..6].copy_from_slice(&[0xd1, 4, start, 0, count, 0]);
        self.transform(&b, true).unwrap()
    }
    pub fn decode(&self, encrypted: &[u8]) -> Result<Vec<Packet>> {
        decode(&self.transform(encrypted, false)?)
    }
}
#[derive(Clone, Debug)]
pub enum Packet {
    State {
        serial: u8,
        cube: Cube,
    },
    Turn {
        serial: u8,
        mv: Move,
        clock: Option<f64>,
        recovered: bool,
    },
    Battery(u8),
    /// Normalized quaternion in cube renderer coordinates, x/y/z/w.
    Orientation([f32; 4]),
    Disconnected,
}
fn bits(data: &[u8], offset: usize, width: usize) -> Result<usize> {
    ensure!(offset + width <= data.len() * 8, "Truncated packet");
    Ok((offset..offset + width).fold(0, |v, i| {
        (v << 1) | ((data[i / 8] >> (7 - i % 8)) & 1) as usize
    }))
}
pub fn decode(b: &[u8]) -> Result<Vec<Packet>> {
    ensure!(
        b.len() >= 16 && b[1] as usize <= b.len() - 2,
        "Malformed packet"
    );
    let length = b[1] as usize;
    let mut out = vec![];
    match b[0] {
        1 => {
            let face = match bits(b, 66, 6)? {
                2 => b'U',
                32 => b'R',
                8 => b'F',
                1 => b'D',
                16 => b'L',
                4 => b'B',
                _ => bail!("Invalid face"),
            };
            let d = bits(b, 64, 2)?;
            ensure!(d < 2, "Invalid direction");
            out.push(Packet::Turn {
                serial: b[6],
                mv: Move {
                    face,
                    turns: if d == 0 { 1 } else { -1 },
                },
                clock: Some(u32::from_le_bytes(b[2..6].try_into()?) as f64),
                recovered: false,
            });
        }
        0xd1 => {
            ensure!((1..=17).contains(&length), "History length");
            for i in 0..(length - 1) * 2 {
                let face = bits(b, 24 + 4 * i, 3)?;
                ensure!(face < 6, "Invalid history move");
                out.push(Packet::Turn {
                    serial: b[2].wrapping_sub(i as u8),
                    mv: Move {
                        face: b"DUBFLR"[face],
                        turns: if bits(b, 27 + 4 * i, 1)? == 0 { 1 } else { -1 },
                    },
                    clock: None,
                    recovered: true,
                });
            }
        }
        0xed => {
            let mut cp = (0..7)
                .map(|i| bits(b, 32 + i * 3, 3))
                .collect::<Result<Vec<_>>>()?;
            let mut co = (0..7)
                .map(|i| bits(b, 53 + i * 2, 2))
                .collect::<Result<Vec<_>>>()?;
            let mut ep = (0..11)
                .map(|i| bits(b, 69 + i * 4, 4))
                .collect::<Result<Vec<_>>>()?;
            let mut eo = (0..11)
                .map(|i| bits(b, 113 + i, 1))
                .collect::<Result<Vec<_>>>()?;
            ensure!(
                cp.iter().sum::<usize>() <= 28 && ep.iter().sum::<usize>() <= 66,
                "Invalid permutation"
            );
            cp.push(28 - cp.iter().sum::<usize>());
            co.push((3 - co.iter().sum::<usize>() % 3) % 3);
            ep.push(66 - ep.iter().sum::<usize>());
            eo.push((2 - eo.iter().sum::<usize>() % 2) % 2);
            out.push(Packet::State {
                serial: b[2],
                cube: Cube::from_cubies(&cp, &co, &ep, &eo)?,
            });
        }
        0xec if length >= 8 => {
            let component = |offset| {
                let n = u16::from_be_bytes([b[offset], b[offset + 1]]);
                (if n & 0x8000 == 0 { 1. } else { -1. }) * (n & 0x7fff) as f32 / 32767.
            };
            // GAN's right-handed axes are red +X, blue +Y, white +Z.
            // Our renderer uses red +X, white +Y, green +Z; preserve the
            // body-to-world rotation while changing basis to (x, z, -y).
            let q = [component(4), component(8), -component(6), component(2)];
            let norm = q.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 0.0001 {
                out.push(Packet::Orientation(q.map(|v| v / norm)));
            }
        }
        0xef => out.push(Packet::Battery(b[1 + length].min(100))),
        0xea => out.push(Packet::Disconnected),
        _ => {}
    }
    Ok(out)
}
#[derive(Clone, Debug)]
pub enum StreamEvent {
    State(Cube),
    Turn(Packet, Cube),
    Fault(&'static str),
    History(u8, u8),
    Verified(u8),
}
#[derive(Default)]
pub struct Stream {
    pub cube: Option<Cube>,
    pub serial: Option<u8>,
    pending: BTreeMap<u8, Packet>,
    snapshots: BTreeMap<u8, Cube>,
    recent: VecDeque<(u8, Cube)>,
    waiting: Option<Instant>,
    last_live: Option<Instant>,
    history_request: Option<(u8, u8, Instant)>,
}
impl Stream {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn accept(&mut self, p: Packet) -> Vec<StreamEvent> {
        self.accept_at(p, Instant::now())
    }
    pub fn accept_at(&mut self, p: Packet, now: Instant) -> Vec<StreamEvent> {
        let mut out = vec![];
        match p {
            Packet::State { serial, cube } => {
                if !cube.valid() {
                    self.reset();
                    return vec![StreamEvent::Fault("invalid-state")];
                }
                let Some(current) = self.serial else {
                    self.cube = Some(cube);
                    self.serial = Some(serial);
                    self.recent.push_back((serial, cube));
                    return vec![StreamEvent::State(cube)];
                };
                let d = serial.wrapping_sub(current);
                if d == 0 {
                    if self.cube == Some(cube) {
                        out.push(StreamEvent::Verified(serial));
                    } else {
                        self.cube = Some(cube);
                        self.pending.clear();
                        self.snapshots.clear();
                        self.recent.clear();
                        self.recent.push_back((serial, cube));
                        self.waiting = None;
                        out.extend([
                            StreamEvent::Fault("state-mismatch"),
                            StreamEvent::State(cube),
                        ]);
                    }
                } else if d < 128 {
                    self.snapshots.insert(serial, cube);
                    self.waiting.get_or_insert(now);
                    // A periodic snapshot may overtake its live notification.
                    // csTimer and gan-web-bluetooth wait for a quiet interval
                    // before treating that ordering as a missing move.
                    out.extend(self.poll(now));
                } else if let Some((_, expected)) = self.recent.iter().find(|(s, _)| *s == serial) {
                    if *expected == cube {
                        out.push(StreamEvent::Verified(serial));
                    } else {
                        self.reset();
                        out.push(StreamEvent::Fault("state-mismatch"));
                    }
                }
            }
            Packet::Turn {
                serial, recovered, ..
            } => {
                let Some(current) = self.serial else {
                    return out;
                };
                let d = serial.wrapping_sub(current);
                if d == 0 || d >= 128 {
                    return out;
                }
                if recovered {
                    // History is padded to pairs. Only admit moves within a
                    // gap evidenced by a live move or a verified snapshot.
                    let bound = self
                        .pending
                        .keys()
                        .chain(self.snapshots.keys())
                        .map(|s| s.wrapping_sub(current))
                        .max()
                        .unwrap_or(0);
                    if d > bound {
                        return out;
                    }
                    // A recovered copy has no timestamp. Never overwrite the
                    // live packet already buffered while waiting for a gap.
                    self.pending.entry(serial).or_insert(p);
                } else {
                    self.last_live = Some(now);
                    self.pending.insert(serial, p);
                }
                if d > 1 {
                    self.waiting.get_or_insert(now);
                    if !recovered {
                        out.extend(self.poll(now));
                    }
                }
                while let Some(p) = self.pending.remove(&self.serial.unwrap().wrapping_add(1)) {
                    if let Packet::Turn { serial, mv, .. } = p {
                        let cube = self.cube.unwrap().turn(mv);
                        self.cube = Some(cube);
                        self.serial = Some(serial);
                        self.recent.push_back((serial, cube));
                        while self.recent.len() > 96 {
                            self.recent.pop_front();
                        }
                        out.push(StreamEvent::Turn(p, cube));
                        if let Some(snapshot) = self.snapshots.remove(&serial) {
                            if snapshot != cube {
                                self.cube = Some(snapshot);
                                self.pending.clear();
                                self.snapshots.clear();
                                self.recent.clear();
                                self.recent.push_back((serial, snapshot));
                                self.waiting = None;
                                out.extend([
                                    StreamEvent::Fault("state-mismatch"),
                                    StreamEvent::State(snapshot),
                                ]);
                                break;
                            } else {
                                out.push(StreamEvent::Verified(serial));
                            }
                        }
                    }
                }
                if self.pending.is_empty() && self.snapshots.is_empty() {
                    self.waiting = None;
                    self.history_request = None;
                }
                if self.pending.len() > 16 {
                    self.reset();
                    out.push(StreamEvent::Fault("missing-moves"));
                }
            }
            _ => {}
        }
        out
    }
    /// Retry a real serial gap, or recover an ahead snapshot once live turns
    /// have been quiet for 500 ms. This also handles a lost final move when no
    /// later move arrives to trigger recovery.
    pub fn poll(&mut self, now: Instant) -> Vec<StreamEvent> {
        let Some(current) = self.serial else {
            return vec![];
        };
        let pending = self
            .pending
            .keys()
            .copied()
            .filter_map(|s| {
                let d = s.wrapping_sub(current);
                (d > 1 && d < 128).then_some((s, d))
            })
            .min_by_key(|(_, d)| *d);
        let request = if let Some(gap) = pending {
            Some(gap)
        } else {
            let quiet = self
                .last_live
                .or(self.waiting)
                .is_some_and(|at| now.saturating_duration_since(at) >= Duration::from_millis(500));
            if !quiet {
                return vec![];
            }
            self.snapshots
                .keys()
                .copied()
                .filter_map(|s| {
                    let d = s.wrapping_sub(current);
                    // Snapshot serial zero is ambiguous at the firmware wrap.
                    // Including the next serial compensates for GAN's odd
                    // history-window alignment and recovers an even final turn.
                    (s != 0 && d > 0 && d <= 16).then_some(if d == 16 && s.is_multiple_of(2) {
                        (s, d)
                    } else {
                        (s.wrapping_add(1), d + 1)
                    })
                })
                .min_by_key(|(_, count)| *count)
        };
        let Some((serial, count)) = request else {
            return vec![];
        };
        if self.history_request.is_some_and(|(s, n, at)| {
            s == serial
                && n == count
                && now.saturating_duration_since(at) < Duration::from_millis(200)
        }) {
            return vec![];
        }
        self.history_request = Some((serial, count, now));
        vec![StreamEvent::History(serial, count)]
    }
    pub fn expire(&mut self) -> bool {
        if self
            .waiting
            .is_some_and(|t| t.elapsed() > Duration::from_millis(1200))
        {
            self.reset();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod orientation_tests {
    use super::*;
    #[test]
    fn gen4_gyro_uses_sign_magnitude_and_renderer_axes() {
        let mut b = vec![0; 20];
        b[0] = 0xec;
        b[1] = 10;
        // Raw w=+0.5, x=-0.5, y=+0.5, z=-0.5.
        for (i, value) in [0x4000u16, 0xc000, 0x4000, 0xc000].into_iter().enumerate() {
            b[2 + i * 2..4 + i * 2].copy_from_slice(&value.to_be_bytes());
        }
        let Packet::Orientation(q) = decode(&b).unwrap()[0] else {
            panic!("orientation expected")
        };
        for (actual, expected) in q.into_iter().zip([-0.5, -0.5, -0.5, 0.5]) {
            assert!((actual - expected).abs() < 0.00001);
        }
        b[2..10].fill(0);
        assert!(decode(&b).unwrap().is_empty());
    }
    #[test]
    fn gen4_gyro_maps_signed_quarter_turns_on_each_sensor_axis() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        // Raw order is w,x,y,z. Equal positive magnitudes encode a quarter turn;
        // bit 15 is a sign bit, not two's-complement arithmetic.
        for (axis, renderer_axis) in [(1, [1., 0., 0.]), (2, [0., 0., -1.]), (3, [0., 1., 0.])] {
            for sign in [1., -1.] {
                let mut b = [0; 20];
                b[0] = 0xec;
                b[1] = 8;
                b[2..4].copy_from_slice(&0x5a82u16.to_be_bytes());
                let word = if sign > 0. { 0x5a82u16 } else { 0xda82u16 };
                b[2 + axis * 2..4 + axis * 2].copy_from_slice(&word.to_be_bytes());
                let Packet::Orientation(q) = decode(&b).unwrap()[0] else {
                    panic!("orientation expected")
                };
                let expected = [
                    renderer_axis[0] * s * sign,
                    renderer_axis[1] * s * sign,
                    renderer_axis[2] * s * sign,
                    s,
                ];
                for (actual, expected) in q.into_iter().zip(expected) {
                    assert!((actual - expected).abs() < 0.00001);
                }
            }
        }
    }
    #[test]
    fn delayed_final_state_is_verified_after_next_scramble_begins() {
        let mut stream = Stream::default();
        stream.accept(Packet::State {
            serial: 254,
            cube: Cube::default().turn(Move::parse("R").unwrap()),
        });
        stream.accept(Packet::Turn {
            serial: 255,
            mv: Move::parse("R'").unwrap(),
            clock: Some(100.),
            recovered: false,
        });
        let next = stream.accept(Packet::Turn {
            serial: 0,
            mv: Move::parse("U").unwrap(),
            clock: Some(200.),
            recovered: false,
        });
        assert!(matches!(next[0], StreamEvent::Turn(_, _)));
        let events = stream.accept(Packet::State {
            serial: 255,
            cube: Cube::default(),
        });
        assert!(matches!(events[0], StreamEvent::Verified(255)));
        assert_eq!(
            stream.cube,
            Some(Cube::default().turn(Move::parse("U").unwrap()))
        );
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    fn turn(serial: u8, text: &str, clock: Option<f64>) -> Packet {
        Packet::Turn {
            serial,
            mv: Move::parse(text).unwrap(),
            clock,
            recovered: clock.is_none(),
        }
    }
    fn initialized(serial: u8, at: Instant) -> Stream {
        let mut stream = Stream::default();
        stream.accept_at(
            Packet::State {
                serial,
                cube: Cube::default(),
            },
            at,
        );
        stream
    }
    #[test]
    fn history_never_replaces_a_buffered_hardware_timestamp() {
        let at = Instant::now();
        let mut stream = initialized(209, at);
        let r = Move::parse("R").unwrap();
        let u = Move::parse("U").unwrap();
        stream.accept_at(turn(211, "U", Some(492877.)), at);
        // Actual GAN history is newest first, and repeats the buffered move.
        assert!(stream.accept_at(turn(211, "U", None), at).is_empty());
        let result = stream.accept_at(turn(210, "R", None), at);
        let turns = result
            .into_iter()
            .filter_map(|e| {
                if let StreamEvent::Turn(p, _) = e {
                    Some(p)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            turns[0],
            Packet::Turn {
                serial: 210,
                recovered: true,
                clock: None,
                ..
            }
        ));
        assert!(matches!(
            turns[1],
            Packet::Turn {
                serial: 211,
                recovered: false,
                clock: Some(492877.),
                ..
            }
        ));
        assert_eq!(stream.cube, Some(Cube::default().apply(&[r, u])));
        assert!(stream.poll(at + Duration::from_secs(1)).is_empty());
    }
    #[test]
    fn delayed_live_packets_upgrade_buffered_recovery_and_remain_exact() {
        let at = Instant::now();
        let mut stream = initialized(0, at);
        stream.accept_at(turn(3, "F", Some(300.)), at);
        stream.accept_at(turn(2, "U", None), at);
        stream.accept_at(turn(2, "U", Some(200.)), at);
        let result = stream.accept_at(turn(1, "R", Some(100.)), at);
        let turns = result
            .into_iter()
            .filter_map(|e| {
                if let StreamEvent::Turn(p, _) = e {
                    Some(p)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(turns.len(), 3);
        assert!(turns.iter().all(|p| matches!(
            p,
            Packet::Turn {
                recovered: false,
                clock: Some(_),
                ..
            }
        )));
    }
    #[test]
    fn ahead_snapshot_waits_for_the_live_notification_before_recovery() {
        let at = Instant::now();
        let mut stream = initialized(0, at);
        let r = Move::parse("R").unwrap();
        assert!(
            stream
                .accept_at(
                    Packet::State {
                        serial: 1,
                        cube: Cube::default().turn(r)
                    },
                    at
                )
                .is_empty()
        );
        assert!(stream.poll(at + Duration::from_millis(499)).is_empty());
        let result = stream.accept_at(turn(1, "R", Some(100.)), at + Duration::from_millis(30));
        assert!(result.iter().any(|e| matches!(e, StreamEvent::Verified(1))));
        assert!(stream.poll(at + Duration::from_secs(1)).is_empty());
    }
    #[test]
    fn a_lost_even_final_turn_is_requested_after_quiet_time_and_verified() {
        let at = Instant::now();
        let mut stream = initialized(1, at);
        let target = Cube::default().turn(Move::parse("U").unwrap());
        stream.accept_at(
            Packet::State {
                serial: 2,
                cube: target,
            },
            at,
        );
        let request = stream.poll(at + Duration::from_millis(500));
        assert!(matches!(request.as_slice(), [StreamEvent::History(3, 2)]));
        // Odd alignment pads history with a future slot: it must not move us.
        assert!(
            stream
                .accept_at(turn(3, "F", None), at + Duration::from_millis(510))
                .is_empty()
        );
        assert_eq!(stream.serial, Some(1));
        let result = stream.accept_at(turn(2, "U", None), at + Duration::from_millis(520));
        assert!(result.iter().any(|e| matches!(e, StreamEvent::Verified(2))));
        assert_eq!(stream.cube, Some(target));
    }
    #[test]
    fn snapshot_at_wrap_is_not_used_to_invent_a_zero_serial_move() {
        let at = Instant::now();
        let mut stream = initialized(255, at);
        stream.accept_at(
            Packet::State {
                serial: 0,
                cube: Cube::default().turn(Move::parse("D").unwrap()),
            },
            at,
        );
        assert!(stream.poll(at + Duration::from_millis(600)).is_empty());
        assert!(stream.accept_at(turn(1, "D", None), at).is_empty());
        assert_eq!(stream.serial, Some(255));
    }
    #[test]
    fn a_full_even_snapshot_window_recovers_in_two_bounded_requests() {
        let at = Instant::now();
        let mut stream = initialized(0, at);
        stream.accept_at(
            Packet::State {
                serial: 16,
                cube: Cube::default(),
            },
            at,
        );
        assert!(matches!(
            stream.poll(at + Duration::from_millis(500)).as_slice(),
            [StreamEvent::History(16, 16)]
        ));
        for serial in (1..16).rev() {
            stream.accept_at(turn(serial, "R", None), at + Duration::from_millis(510));
        }
        assert_eq!(stream.serial, Some(15));
        assert!(matches!(
            stream.poll(at + Duration::from_millis(520)).as_slice(),
            [StreamEvent::History(17, 2)]
        ));
        let result = stream.accept_at(turn(16, "R", None), at + Duration::from_millis(530));
        assert!(
            result
                .iter()
                .any(|e| matches!(e, StreamEvent::Verified(16)))
        );
        assert_eq!(stream.cube, Some(Cube::default()));
    }
    #[test]
    fn an_unanswered_gap_retries_without_flooding_the_command_queue() {
        let at = Instant::now();
        let mut stream = initialized(0, at);
        assert!(matches!(
            stream.accept_at(turn(2, "U", Some(200.)), at).as_slice(),
            [StreamEvent::History(2, 2)]
        ));
        assert!(stream.poll(at + Duration::from_millis(199)).is_empty());
        assert!(matches!(
            stream.poll(at + Duration::from_millis(200)).as_slice(),
            [StreamEvent::History(2, 2)]
        ));
    }
}
