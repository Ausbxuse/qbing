//! csTimer's unmodified solver runs headlessly, off the native UI thread.
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::cell::RefCell;
thread_local! {static ENGINE: RefCell<Option<quick_js::Context>> = const { RefCell::new(None) };}
pub fn request(route: &str, body: Option<&Value>) -> Result<Value> {
    ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        if engine.is_none() {
            let context = quick_js::Context::builder()
                .memory_limit(64 * 1024 * 1024)
                .build()
                .map_err(|e| anyhow!("{e}"))?;
            context
                .eval(&format!(
                    "try{{ {} }}catch(e){{throw new Error(e.stack)}}",
                    include_str!("../../assets/solver.js")
                ))
                .map_err(|e| anyhow!("{e}"))?;
            *engine = Some(context);
        }
        let input = json!({"route":route,"body":body}).to_string();
        let output = engine
            .as_ref()
            .unwrap()
            .eval(&format!(
                "try{{nativeRequest({})}}catch(e){{throw new Error(e.stack)}}",
                serde_json::to_string(&input)?
            ))
            .map_err(|e| anyhow!("{e}"))?;
        let quick_js::JsValue::String(output) = output else {
            return Err(anyhow!("Invalid solver result"));
        };
        Ok(serde_json::from_str(&output)?)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::{Cube, sequence};
    #[test]
    fn random_state_and_recovery_are_verified() {
        let result = request("scramble", None).unwrap();
        let state = Cube::default().apply(&sequence(result["scramble"].as_str().unwrap()).unwrap());
        assert_eq!(state.to_string(), result["state"].as_str().unwrap());
        let target = Cube::default().apply(&sequence("R U F2 D L'").unwrap());
        let recovery = request(
            "recovery",
            Some(&json!({"state":state.to_string(),"target":target.to_string()})),
        )
        .unwrap();
        let moves = recovery["moves"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m.as_str().unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(state.apply(&sequence(&moves).unwrap()), target);
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn pll_alternatives_are_verified_on_the_reconstructed_case() {
        let moves = "R U R' U' R' F R2 U' R' U' R U R' F'"
            .split_whitespace()
            .collect::<Vec<_>>();
        let solution = crate::cube::sequence(&moves.join(" ")).unwrap();
        let initial = crate::cube::Cube::default().apply(
            &solution
                .iter()
                .rev()
                .map(|m| m.inverse())
                .collect::<Vec<_>>(),
        );
        let mut record = crate::recording::refresh_analysis(
            json!({"id":"synthetic-pll","kind":"solve","source":"bluetooth","status":"complete","penalty":"","cross":"D","duration":1300,"initialState":initial.to_string(),"moves":moves.iter().enumerate().map(|(i,m)|json!({"move":m,"t":i*100})).collect::<Vec<_>>(),"quality":[]}),
        );
        record["analysis"] = json!({"cross":"D","splits":[{"stage":"PLL","rawMoves":moves.len()}]});
        let result = request(
            "native-algorithms",
            Some(&json!({"record":record,"stage":"PLL"})),
        )
        .unwrap();
        assert!(
            result["matches"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["id"] == "pll-t")
        );
        for variant in result["variants"].as_array().unwrap() {
            let (actual, _) = super::reconstruction_case_tests::apply_algorithm(
                initial,
                variant["algorithm"].as_str().unwrap(),
                b'D',
            );
            assert_eq!(
                actual,
                crate::cube::Cube::default(),
                "PLL alternative must solve the recorded case: {}",
                variant["algorithm"]
            );
        }
    }
}

#[cfg(test)]
mod dashboard_tests {
    use super::*;
    use crate::cube::{CORNERS, Cube, EDGES, sequence};
    #[test]
    fn dashboard_routes_solve_the_actual_cross_and_pair_for_every_face() {
        for scramble in [
            "R U F2 D L' B2 R2 U' F R2 D2 L2",
            "D2 B2 U F2 D R2 F2 L2 U' L2 D2 F' R2 U' B R' U' L' D R2",
        ] {
            let start = Cube::default().apply(&sequence(scramble).unwrap());
            let result = request(
                "native-cross-dashboard",
                Some(&json!({"state":start.to_string(),"faces":"UDLRFB"})),
            )
            .unwrap();
            let routes = result["routes"].as_array().unwrap();
            assert_eq!(routes.len(), 12);
            for r in routes {
                assert_eq!(r["verified"], true);
                let face = r["face"].as_str().unwrap().as_bytes()[0];
                let text = r["fixedMoves"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(" ");
                let end = start.apply(&sequence(&text).unwrap());
                assert!(end.goal("Cross", face));
                if !start.goal("Cross", face) {
                    assert!(r["length"].as_u64().unwrap() > 0);
                }
                if r["kind"] == "xcross" {
                    let solved = Cube::default();
                    let correct = |ids: &[usize]| ids.iter().all(|i| end.0[*i] == solved.0[*i]);
                    assert!(
                        CORNERS
                            .iter()
                            .any(|c| c.iter().any(|i| solved.0[*i] == face)
                                && correct(c)
                                && EDGES.iter().any(|e| e.iter().all(|i| solved.0[*i] != face
                                    && c.iter().any(|j| solved.0[*i] == solved.0[*j]))
                                    && correct(e)))
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod reconstruction_case_tests {
    use super::*;
    pub(super) fn apply_algorithm(
        mut cube: crate::cube::Cube,
        algorithm: &str,
        cross: u8,
    ) -> (crate::cube::Cube, u8) {
        use crate::cube::{Cube, FACES, geometry, rotate};
        for token in algorithm.split_whitespace() {
            let face = token.as_bytes()[0];
            let upper = face.to_ascii_uppercase();
            let (axis, layer, sign) = match upper {
                b'R' => (0, Some(1.), -1.),
                b'L' => (0, Some(-1.), 1.),
                b'U' => (1, Some(1.), -1.),
                b'D' => (1, Some(-1.), 1.),
                b'F' => (2, Some(1.), -1.),
                b'B' => (2, Some(-1.), 1.),
                b'M' => (0, Some(0.), 1.),
                b'E' => (1, Some(0.), 1.),
                b'S' => (2, Some(0.), -1.),
                b'X' => (0, None, -1.),
                b'Y' => (1, None, -1.),
                b'Z' => (2, None, -1.),
                _ => panic!("Unsupported test algorithm token {token}"),
            };
            let wide =
                token.contains('w') || face.is_ascii_lowercase() && b"urfdlb".contains(&face);
            let count = if token.contains('2') {
                2
            } else if token.ends_with('\'') {
                3
            } else {
                1
            };
            for _ in 0..count {
                let mut out = cube;
                for (i, sticker) in geometry().iter().enumerate() {
                    if let Some(layer) = layer {
                        if if wide {
                            sticker.p[axis] * layer < 0.
                        } else {
                            sticker.p[axis] != layer
                        } {
                            continue;
                        }
                    }
                    let point =
                        rotate(sticker.p, axis, sign * std::f32::consts::FRAC_PI_2).map(f32::round);
                    let normal =
                        rotate(sticker.n, axis, sign * std::f32::consts::FRAC_PI_2).map(f32::round);
                    let j = geometry()
                        .iter()
                        .position(|s| s.p == point && s.n == normal)
                        .unwrap();
                    out.0[j] = cube.0[i];
                }
                cube = out;
            }
        }
        let centers = [
            cube.0[4], cube.0[13], cube.0[22], cube.0[31], cube.0[40], cube.0[49],
        ];
        let mapped = FACES[centers.iter().position(|c| *c == cross).unwrap()];
        (
            Cube(
                cube.0
                    .map(|c| FACES[centers.iter().position(|f| *f == c).unwrap()]),
            ),
            mapped,
        )
    }
    #[test]
    fn oll_alternatives_work_on_the_actual_case_and_not_just_on_a_solved_cube() {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../shared/src/commonMain/composeResources/files/catalog.json"
        ))
        .unwrap();
        let case = catalog["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == "oll-27")
            .unwrap();
        let pos = &case["variants"][0]["positions"]["D"];
        let moves = pos["solution"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, m)| json!({"move":m,"t":i*100}))
            .collect::<Vec<_>>();
        let record = json!({"initialState":pos["state"],"cross":"D","moves":moves,"analysis":{"cross":"D","splits":[{"stage":"OLL","rawMoves":moves.len()}]}});
        let result = request(
            "native-algorithms",
            Some(&json!({"record":record,"stage":"OLL"})),
        )
        .unwrap();
        assert!(result["variants"].as_array().unwrap().len() >= 2);
        for variant in result["variants"].as_array().unwrap() {
            assert_eq!(variant["caseId"], "oll-27");
            let (state, face) = apply_algorithm(
                crate::cube::Cube::parse(pos["state"].as_str().unwrap()).unwrap(),
                variant["algorithm"].as_str().unwrap(),
                b'D',
            );
            assert!(
                state.progress(face)[2],
                "{} must orient LL and retain F2L",
                variant["algorithm"]
            );
        }
    }
    #[test]
    fn displayed_cross_routes_work_in_the_reconstruction_frame_for_every_face() {
        let initial =
            crate::cube::Cube::default().apply(&crate::cube::sequence("R U F2 L D B2").unwrap());
        let raw = crate::cube::sequence("B2 D' L' F2 U' R'")
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, m)| json!({"move":m.to_string(),"t":i*100}))
            .collect::<Vec<_>>();
        for face in ["U", "D", "R", "L", "F", "B"] {
            let record = json!({"initialState":initial.to_string(),"cross":face,"moves":raw,"analysis":{"cross":face,"splits":[{"stage":"Cross","rawMoves":raw.len()}]}});
            let data = request("native-review", Some(&json!({"record":record}))).unwrap();
            let view = match face {
                "U" => "z2",
                "F" => "x'",
                "B" => "x",
                "R" => "z",
                "L" => "z'",
                _ => "",
            };
            for v in data["groups"][0]["variants"].as_array().unwrap() {
                let (state, cross) = apply_algorithm(
                    initial,
                    &format!("{view} {}", v["viewAlgorithm"].as_str().unwrap()),
                    face.as_bytes()[0],
                );
                assert!(state.progress(cross)[0], "{face}: {}", v["viewAlgorithm"]);
            }
        }
    }
    #[test]
    fn f2l_cases_are_classified_and_alternatives_preserve_the_cross_and_other_pairs() {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../shared/src/commonMain/composeResources/files/catalog.json"
        ))
        .unwrap();
        for face in ["U", "D", "R", "L", "F", "B"] {
            let case = catalog["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == "f2l-1")
                .unwrap();
            let pos = &case["variants"][0]["positions"][face];
            let moves = pos["solution"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, m)| json!({"move":m,"t":i*200}))
                .collect::<Vec<_>>();
            let mut r = crate::recording::refresh_analysis(
                json!({"id":"f2l-test","kind":"solve","source":"bluetooth","status":"complete","penalty":"","cross":face,"duration":1000.,"initialState":pos["state"],"moves":moves,"quality":[]}),
            );
            // Isolate the case classifier from automatic cross selection.
            r["analysis"] = json!({"cross":face,"splits":[{"stage":"Cross","rawMoves":0},{"stage":"F2L","rawMoves":r["moves"].as_array().unwrap().len()},{"stage":"OLL","rawMoves":0},{"stage":"PLL","rawMoves":0}]});
            let result = request("native-review", Some(&json!({"record":r}))).unwrap();
            let groups = result["groups"].as_array().unwrap();
            let pair = groups
                .iter()
                .find(|g| g["stage"] == "F2L" && g["rawStart"] != g["rawEnd"])
                .unwrap();
            assert!(
                pair["caseIds"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("f2l-1")),
                "{face}: {pair}"
            );
            assert!(!pair["variants"].as_array().unwrap().is_empty());
            for variant in pair["variants"].as_array().unwrap() {
                assert_eq!(variant["verified"], true);
                let initial = crate::cube::Cube::parse(pos["state"].as_str().unwrap()).unwrap();
                let (actual, view_cross) = apply_algorithm(
                    initial,
                    variant["algorithm"].as_str().unwrap(),
                    face.as_bytes()[0],
                );
                let view = match face {
                    "U" => "z2",
                    "F" => "x'",
                    "B" => "x",
                    "R" => "z",
                    "L" => "z'",
                    _ => "",
                };
                let (in_view, view_face) = apply_algorithm(
                    initial,
                    &format!("{view} {}", variant["viewAlgorithm"].as_str().unwrap()),
                    face.as_bytes()[0],
                );
                assert!(
                    in_view.progress(view_face)[1],
                    "Displayed algorithm must work in the cross-down replay frame"
                );
                assert!(
                    actual.progress(view_cross)[1],
                    "Independent geometry rejected {}",
                    variant["algorithm"]
                );
                let end = crate::cube::Cube::parse(variant["endState"].as_str().unwrap()).unwrap();
                assert!(
                    end.progress(face.as_bytes()[0])[1],
                    "Each suggestion must finish this single remaining pair and preserve F2L"
                );
            }
        }
    }
}
