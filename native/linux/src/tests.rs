use crate::{cube::*, gan::*, io};
use serde_json::Value;

#[test]
fn cube_matches_independent_fixtures() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../shared/src/commonMain/composeResources/files/cube-oracle.json"
    ))
    .unwrap();
    for f in fixtures {
        let moves = sequence(f["alg"].as_str().unwrap()).unwrap();
        let state = Cube::default().apply(&moves);
        assert_eq!(state.to_string(), f["facelets"].as_str().unwrap());
        assert!(state.valid());
        let inverse = moves.iter().rev().map(|m| m.inverse()).collect::<Vec<_>>();
        assert_eq!(state.apply(&inverse), Cube::default());
    }
}
#[test]
fn catalog_drills_solve_in_every_cross_frame() {
    let catalog = io::catalog();
    assert_eq!(catalog.cases.len(), 125);
    for case in catalog.cases {
        for variant in case.variants {
            for (cross, p) in variant.positions {
                let state = Cube::parse(&p.state).unwrap();
                let setup = p
                    .setup
                    .iter()
                    .map(|m| Move::parse(m).unwrap())
                    .collect::<Vec<_>>();
                let solution = p
                    .solution
                    .iter()
                    .map(|m| Move::parse(m).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(Cube::default().apply(&setup), state, "{} {cross}", case.id);
                assert_eq!(state.apply(&solution), Cube::default());
                assert!(
                    !state.goal(&case.stage, cross.as_bytes()[0]),
                    "{} {cross}",
                    case.id
                );
            }
        }
    }
}
#[test]
fn illegal_piece_states_are_rejected() {
    let mut c = Cube::default();
    c.0.swap(5, 10);
    assert!(!c.valid());
    let mut c = Cube::default();
    for i in 0..54 {
        c.0[i] = b'U';
    }
    assert!(!c.valid());
}
#[test]
fn aes_matches_independent_upstream_vector() {
    let cipher = Cipher::new("12:34:56:78:9A:BC").unwrap();
    let expected = "9294f283805a5d11623b6663e66c5ad021727f85";
    assert_eq!(
        cipher
            .command("state")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        expected
    );
    let mut plain = vec![0u8; 20];
    plain[..4].copy_from_slice(&[0xdd, 4, 0, 0xed]);
    assert_eq!(
        cipher.transform(&cipher.command("state"), false).unwrap(),
        plain
    );
}
#[test]
fn serial_gaps_recover_across_wrap_without_duplicate_moves() {
    let mut stream = Stream::default();
    stream.accept(Packet::State {
        serial: 254,
        cube: Cube::default(),
    });
    let r = Move::parse("R").unwrap();
    let u = Move::parse("U").unwrap();
    let expected = Cube::default().apply(&[r, u]);
    stream.accept(Packet::State {
        serial: 0,
        cube: expected,
    });
    let events = stream.accept(Packet::Turn {
        serial: 0,
        mv: u,
        clock: Some(200.),
        recovered: false,
    });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StreamEvent::History(0, 2)))
    );
    let events = stream.accept(Packet::Turn {
        serial: 255,
        mv: r,
        clock: None,
        recovered: true,
    });
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, StreamEvent::Turn(..)))
            .count(),
        2
    );
    assert_eq!(stream.cube, Some(expected));
    assert!(events.iter().any(|e| matches!(e, StreamEvent::Verified(0))));
    assert!(
        stream
            .accept(Packet::Turn {
                serial: 0,
                mv: u,
                clock: Some(200.),
                recovered: false
            })
            .is_empty()
    );
}
#[test]
fn scramble_marks_half_turns_and_wrong_turns() {
    let moves = sequence("R2 U' F").unwrap();
    let halfway = Cube::default().turn(Move::parse("R").unwrap());
    let p = scramble_progress(halfway, &moves);
    assert_eq!(p.completed, 0);
    assert!(p.partial && p.matched);
    let p = scramble_progress(Cube::default().apply(&moves[..2]), &moves);
    assert_eq!(p.completed, 2);
    assert!(!p.partial);
    assert!(!scramble_progress(halfway.turn(Move::parse("L").unwrap()), &moves).matched);
    assert_eq!(
        scramble_progress(Cube::default().apply(&moves), &moves).completed,
        3
    );
}
#[test]
fn dnf_rules_trim_one_extreme() {
    let records = (1..=5)
        .map(|n| serde_json::json!({"duration":n*1000,"status":"complete","penalty":""}))
        .collect::<Vec<_>>();
    assert_eq!(io::average(&records, 5), Some(3000.));
    let mut r = records;
    r[4]["penalty"] = serde_json::json!("DNF");
    assert_eq!(io::average(&r, 5), Some(3000.));
    r[3]["penalty"] = serde_json::json!("DNF");
    assert!(io::average(&r, 5).unwrap().is_infinite());
}

#[test]
fn average_tps_and_moves_use_the_same_retained_attempts_as_time() {
    use crate::practice_ui::average_metrics;
    let mut records = (1..=5)
        .map(|n| {
            serde_json::json!({
                "duration": n * 1000, "status":"complete", "penalty":"",
                "source":"bluetooth", "quality":[], "analysis":{"tps":n * 2},
                "moves":(0..n as usize * 10).map(|i| serde_json::json!({"move":if i%2==0 {"R"} else {"U"},"t":i as f64 / (n * 10 - 1) as f64 * (n*1000) as f64})).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let result = average_metrics(&records, 5);
    assert_eq!(result.time, Some(3000.));
    assert_eq!(result.tps, Some(10.));
    assert_eq!(result.moves, Some(30.));
    // Manual fastest/worst results are trimmed consistently from all columns.
    records[0]["source"] = serde_json::json!("manual");
    records[4]["penalty"] = serde_json::json!("DNF");
    assert_eq!(average_metrics(&records, 5).tps, Some(10.));
    // A retained manual result cannot erase other sensor samples. No older
    // solve is substituted, and the partial denominator remains available.
    records[2]["source"] = serde_json::json!("manual");
    let mixed = average_metrics(&records, 5);
    assert_eq!(mixed.time, Some(3000.));
    assert_eq!(mixed.tps, Some(10.));
    assert_eq!(mixed.moves, Some(30.));
    assert_eq!(
        (mixed.tps_samples, mixed.move_samples, mixed.retained),
        (2, 2, 3)
    );
    assert_eq!(average_metrics(&records, 12).time, None);
}

#[test]
fn next_scramble_does_not_wait_for_history_sync() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Condvar, Mutex, mpsc},
        time::Duration,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    struct Release(Arc<(Mutex<bool>, Condvar)>);
    impl Drop for Release {
        fn drop(&mut self) {
            *self.0.0.lock().unwrap() = true;
            self.0.1.notify_all();
        }
    }
    let release = Release(gate.clone());
    let (entered_tx, entered_rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(3) {
            let gate = gate.clone();
            let entered_tx = entered_tx.clone();
            std::thread::spawn(move || {
                let mut stream = stream.unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = [0; 8192];
                let count = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..count]);
                let body = if request.starts_with("GET /bootstrap ") {
                    entered_tx.send(()).unwrap();
                    let mut ready = gate.0.lock().unwrap();
                    while !*ready {
                        ready = gate.1.wait(ready).unwrap();
                    }
                    r#"{"records":[],"preferences":{}}"#
                } else if request.starts_with("GET /scramble ") {
                    r#"{"scramble":"R U F"}"#
                } else {
                    "[]"
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            });
        }
    });
    let (tx, rx) = io::api_worker(eframe::egui::Context::default());
    let settings = io::Settings {
        endpoint: format!("http://{address}"),
        token: "isolated-test".into(),
        ..Default::default()
    };
    tx.send(io::Work::Sync {
        settings: settings.clone(),
        pending: vec![],
    })
    .unwrap();
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    tx.send(io::Work::Request {
        settings,
        route: "scramble".into(),
        method: "GET".into(),
        body: None,
        tag: "quick-scramble".into(),
    })
    .unwrap();
    let guidance = rx
        .recv_timeout(Duration::from_secs(3))
        .expect("Scramble queued behind blocked history sync");
    assert_eq!(guidance.tag, "quick-scramble");
    assert_eq!(guidance.result.unwrap()["scramble"], "R U F");
    drop(release);
    assert_eq!(rx.recv_timeout(Duration::from_secs(3)).unwrap().tag, "sync");
}
