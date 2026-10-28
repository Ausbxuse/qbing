//! Capture the real eframe/Glow application with synthetic data only.
use crate::{
    cube::{Cube, sequence},
    model::Target,
    render,
    ui::App,
};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub fn run() -> eframe::Result {
    let directory = PathBuf::from(
        std::env::var("CUBE_LAB_RENDER_DIR").unwrap_or("test-results/native-gl".into()),
    );
    std::fs::create_dir_all(&directory).expect("Create native capture directory");
    let view = std::env::var("CUBE_LAB_CAPTURE_VIEW").unwrap_or("result".into());
    let theme = std::env::var("CUBE_LAB_CAPTURE_THEME").unwrap_or("paper".into());
    assert!(matches!(
        theme.as_str(),
        "snappy" | "graphite" | "paper" | "carbon"
    ));
    let size = std::env::var("CUBE_LAB_CAPTURE_SIZE").unwrap_or("1280x900".into());
    let size = size
        .split_once('x')
        .and_then(|(w, h)| Some([w.parse::<f32>().ok()?, h.parse::<f32>().ok()?]))
        .expect("Use WIDTHxHEIGHT for native capture");
    let scale = std::env::var("CUBE_LAB_CAPTURE_SCALE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0)
        .clamp(0.75, 3.0);
    let theme_suffix = if theme == "snappy" {
        String::new()
    } else {
        format!("-{theme}")
    };
    let filename = format!(
        "{}x{}-{}-{}x{}",
        size[0] as u32, size[1] as u32, view, scale, theme_suffix
    );
    let output = directory.join(format!("{filename}.png"));
    let metadata = directory.join(format!("{filename}.json"));
    let saved = Arc::new(AtomicBool::new(false));
    let error = Arc::new(Mutex::new(None));
    let capture_saved = saved.clone();
    let capture_error = error.clone();
    eframe::run_native(
        "Cube Lab · isolated native capture",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(
                    if std::env::var("CUBE_LAB_CAPTURE_RESIZE").as_deref() == Ok("1") {
                        [1440., 1000.]
                    } else {
                        size
                    },
                )
                .with_position([0., 0.])
                .with_decorations(false)
                .with_app_id("lab.cube.NativeCapture"),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            let gl = cc
                .gl
                .as_ref()
                .ok_or_else(|| std::io::Error::other("Native Glow context unavailable"))?;
            render::install_glow(&cc.egui_ctx, gl.clone());
            let mut app = App::new(cc.egui_ctx.clone(), true);
            app.model.settings.theme = theme;
            app.model.settings.session = "Session 1".into();
            let fixture = std::env::var("CUBE_LAB_RENDER_FIXTURE")
                .unwrap_or("../.data/design-fixture.json".into());
            let data: serde_json::Value = serde_json::from_slice(&std::fs::read(fixture)?)?;
            app.model.records = data["records"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(crate::recording::refresh_analysis)
                .collect();
            app.model.preferences = data["preferences"].clone();
            app.model.stats = data["stats"].as_array().cloned().unwrap_or_default();
            app.model.latest = app
                .model
                .records
                .iter()
                .rev()
                .find(|r| r["kind"] == "solve")
                .cloned();
            app.model.connected = false;
            app.model.powered = true;
            app.model.device_name = "Preview cube".into();
            app.model.battery = Some(84);
            app.model.sync_status = "Preview · synthetic records".into();
            app.model.message = "Preview · synthetic records".into();
            let moves = sequence(if view == "commuting" {
                "U2 D R2 L' F2 B U' R2 D F2 L2 B' U2 R D' F L2 U' R2 B2"
            } else {
                "D2 B2 U F2 D R2 F2 L2 U' L2 D2 F' R2 U' B R' U' L' D R2"
            })?;
            app.model.target = Some(Target {
                cube: Cube::default().apply(&moves),
                moves,
                stage: "solve".into(),
                case: None,
                algorithm: None,
            });
            app.model.state = Some(if view == "correction" {
                app.model.guidance_checkpoint =
                    Some(Cube::default().apply(&sequence("D2 B2 U F2 D R")?));
                app.model.recovery = sequence("B' U'")?;
                Cube::default().apply(&sequence("D2 B2 U F2 D R U B")?)
            } else if view == "commuting" {
                Cube::default().apply(&sequence("D U")?)
            } else if view == "half" {
                Cube::default().apply(&sequence("D2 B2 U F2 D R")?)
            } else {
                Cube::default()
            });
            if matches!(view.as_str(), "correction" | "half" | "commuting") {
                let prep = sequence(match view.as_str() {
                    "correction" => "D2 B2 U F2 D R U B",
                    "half" => "D2 B2 U F2 D R",
                    _ => "D U",
                })?;
                let turns = prep
                    .into_iter()
                    .flat_map(|mv| {
                        if mv.turns == 2 {
                            vec![
                                crate::cube::Move {
                                    face: mv.face,
                                    turns: 1
                                };
                                2
                            ]
                        } else {
                            vec![mv]
                        }
                    })
                    .collect::<Vec<_>>();
                let now = Instant::now();
                let first = now - Duration::from_millis(turns.len() as u64 * 160);
                let mut before = Cube::default();
                for (index, mv) in turns.into_iter().enumerate() {
                    let after = before.turn(mv);
                    app.model.scramble.observe(
                        Some(before),
                        after,
                        mv,
                        Some(1000. + index as f64 * 160.),
                        false,
                        first + Duration::from_millis(index as u64 * 160),
                    );
                    before = after;
                }
            }
            if view.starts_with("case-analysis-") {
                let id = match view.as_str() {
                    "case-analysis-f2l" => "f2l-1",
                    "case-analysis-oll" => "oll-27",
                    _ => "pll-t",
                };
                let case = app
                    .model
                    .catalog
                    .cases
                    .iter()
                    .find(|c| c.id == id)
                    .unwrap()
                    .clone();
                for i in 0..12 {
                    let variant =
                        &case.variants[(if i < 8 { 0 } else { 1 }).min(case.variants.len() - 1)];
                    let pos = &variant.positions["D"];
                    let duration = 1800. - i as f64 * 60.;
                    app.model.records.push(serde_json::json!({"id":format!("case-fixture-{id}-{i}"),"createdAt":format!("2026-10-27T10:{i:02}:00Z"),"kind":"drill","source":"bluetooth","status":"complete","penalty":"","caseId":id,"selectedAlgorithmId":variant.id,"cross":"D","duration":duration,"initialState":pos.state,"quality":[],"recognitionSource":"cue","recognitionMs":500.-i as f64*20.,"moves":pos.solution.iter().enumerate().map(|(j,m)|serde_json::json!({"move":m,"t":j as f64*duration/(pos.solution.len()-1) as f64})).collect::<Vec<_>>()}));
                }
                app.model.stats.clear();
                app.selected = id.into();
                app.variant = 0;
                app.case_analysis = true;
                app.page = "Case".into();
            } else if matches!(
                view.as_str(),
                "review-analysis" | "review-analysis-playing" | "review-analysis-pairs"
            ) {
                let record = serde_json::from_str(include_str!(
                    "../../androidApp/src/debug/assets/analysis-fixture.json"
                ))
                .unwrap();
                app.open_review(record);
                app.review_tab = 1;
                if view == "review-analysis-playing" {
                    app.toggle_review_playback();
                }
            } else if view == "settings" {
                app.page = "Settings".into();
            } else if view == "tools-last" {
                app.shell.selected = app
                    .model
                    .records
                    .first()
                    .and_then(|r| r["id"].as_str())
                    .map(str::to_string);
            } else if view == "replay-yellow" {
                let moves =
                    sequence("D R U R' U' R U R' U R U2 R' R U R' U' R' F R2 U' R' U' R U R' F'")?;
                let initial = Cube::default()
                    .apply(&moves.iter().rev().map(|m| m.inverse()).collect::<Vec<_>>());
                let record = crate::recording::refresh_analysis(
                    serde_json::json!({"id":"synthetic-yellow-replay","kind":"solve","source":"bluetooth","status":"complete","cross":"U","duration":2500.,"initialState":initial.to_string(),"moves":moves.iter().enumerate().map(|(i,m)|serde_json::json!({"move":m.to_string(),"t":i*100})).collect::<Vec<_>>(),"quality":[]}),
                );
                assert_eq!(record["analysis"]["cross"], "D");
                app.open_review(record);
            } else if view == "connect" || view == "bluetooth-off" {
                app.show_devices = true;
                app.model.powered = view != "bluetooth-off";
                app.model.devices = vec![crate::bluetooth::DeviceInfo {
                    name: "GAN16 UI".into(),
                    address: "00:00:00:00:00:00".into(),
                }];
            } else if view == "companion" {
                app.shell.widget = true;
            } else if view == "reset" {
                app.reset_confirm = true;
            } else if view == "history" || view.starts_with("history-") {
                app.page = "History".into();
            } else if view == "training" {
                app.start_training("pll-t", 0);
                app.model.message = "Preview · synthetic training".into();
            } else if view == "replay-rotations" {
                if let Some(mut record) = app.model.latest.clone() {
                    let s = std::f32::consts::FRAC_1_SQRT_2;
                    record["rotationFrame"] = serde_json::json!("live-v1");
                    record["initialOrientation"] = serde_json::json!([0., s, 0., s]);
                    record["rotations"] = serde_json::json!([
                        {"notation":"y'","t":250.,"pose":[0.,1.,0.,0.],"timing":"local-gyro"},
                        {"notation":"x'","t":750.,"pose":[0.,s,-s,0.],"timing":"local-gyro"}
                    ]);
                    app.open_review(record);
                    app.seek_review_time(750.);
                    app.review_reconstruction = true;
                }
            } else if view == "phase" || view == "reconstruction" {
                if let Some(record) = app.model.latest.clone() {
                    app.open_phase_review(record, "F2L");
                    app.review_reconstruction = view == "reconstruction";
                }
            } else if view == "live" {
                if let Some(record) = app.model.latest.clone() {
                    let initial = Cube::parse(record["initialState"].as_str().unwrap_or(""))
                        .unwrap_or_default();
                    app.model.state = Some(initial);
                    app.model.arm_current();
                    if let Some(active) = &mut app.model.active {
                        active.started = Some(Instant::now() - Duration::from_secs(4));
                        active.clock = Some(0.);
                        active.moves = record["moves"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|event| event["t"].as_f64().is_some_and(|t| t <= 3000.))
                            .cloned()
                            .collect();
                        let turns = active
                            .moves
                            .iter()
                            .filter_map(|event| {
                                crate::cube::Move::parse(event["move"].as_str().unwrap_or("")).ok()
                            })
                            .collect::<Vec<_>>();
                        app.model.state = Some(initial.apply(&turns));
                    }
                    app.model.message = "Preview · synthetic live solve".into();
                }
            }
            if view.ends_with("-tps") {
                app.shell.graph_metric = crate::graph::Metric::Tps;
            }
            if view.ends_with("-moves") {
                app.shell.graph_metric = crate::graph::Metric::Moves;
            }
            app.animator.snap(app.model.state.unwrap());
            cc.egui_ctx.set_pixels_per_point(scale);
            Ok(Box::new(Capture {
                app,
                output,
                metadata,
                size,
                scale,
                frames: 0,
                scrolled: false,
                requested: false,
                started: Instant::now(),
                saved: capture_saved,
                error: capture_error,
            }))
        }),
    )?;
    if let Some(error) = error.lock().unwrap().take() {
        panic!("Native capture failed: {error}");
    }
    assert!(
        saved.load(Ordering::Relaxed),
        "Native capture was not saved"
    );
    println!(
        "Captured full native application: {}",
        directory.join(format!("{filename}.png")).display()
    );
    Ok(())
}

struct Capture {
    app: App,
    output: PathBuf,
    metadata: PathBuf,
    size: [f32; 2],
    scale: f32,
    frames: usize,
    scrolled: bool,
    requested: bool,
    started: Instant,
    saved: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}
impl eframe::App for Capture {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if std::env::var("CUBE_LAB_CAPTURE_VIEW").as_deref() == Ok("review-analysis-pairs")
            && self.frames > 8
            && !self.scrolled
            && crate::native_ui::reconstruction_ready(&self.app)
        {
            input.events.push(egui::Event::PointerMoved(egui::pos2(
                self.size[0] / 2.,
                self.size[1] - 180.,
            )));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -210.),
                modifiers: egui::Modifiers::default(),
            });
            self.scrolled = true;
            self.started = Instant::now();
        }
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                let bytes = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let capture =
                    image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, bytes)
                        .unwrap();
                let timer = ctx.data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new("practice-timer-bounds"))
                });
                let secondary = ctx.data(|data| {
                    data.get_temp::<[egui::Rect; 2]>(egui::Id::new("practice-secondary-bounds"))
                });
                let metrics = ctx.data(|data| {
                    data.get_temp::<serde_json::Value>(egui::Id::new("practice-metrics"))
                });
                let phase_metrics = ctx.data(|data| {
                    data.get_temp::<serde_json::Value>(egui::Id::new("phase-metrics"))
                });
                let coords =
                    |rect: egui::Rect| [rect.left(), rect.top(), rect.right(), rect.bottom()];
                let metadata = serde_json::json!({"synthetic":true,"renderer":"eframe-glow","pixelsPerPoint":ctx.pixels_per_point(),"logicalSize":[ctx.content_rect().width(),ctx.content_rect().height()],"pixelSize":image.size,"gpu":crate::render::native_profile(),"bounds":self.app.shell.bounds,"timerBounds":timer.map(coords),"secondaryBounds":secondary.map(|rects| rects.map(coords)),"metrics":metrics,"phaseMetrics":phase_metrics,"tools":crate::native_ui::tool_evidence(&self.app),"replayCross":self.app.review.as_ref().map(|r|(crate::recording::reconstruction_cross(r) as char).to_string())});
                match capture
                    .save(&self.output)
                    .map_err(|e| e.to_string())
                    .and_then(|_| {
                        std::fs::write(
                            &self.metadata,
                            serde_json::to_vec_pretty(&metadata).unwrap(),
                        )
                        .map_err(|e| e.to_string())
                    }) {
                    Ok(()) => self.saved.store(true, Ordering::Relaxed),
                    Err(error) => *self.error.lock().unwrap() = Some(error),
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.frames == 0 {
            ctx.set_pixels_per_point(self.scale);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.size.into()));
        }
        // Freeze only this synthetic capture so the active pause affordance is visible.
        if std::env::var("CUBE_LAB_CAPTURE_VIEW").as_deref() == Ok("review-analysis-playing") {
            self.app.playing = Instant::now().checked_sub(Duration::from_millis(700));
        }
        self.app.draw(ctx);
        self.frames += 1;
        let size = ctx.content_rect().size();
        let sized = (size.x - self.size[0]).abs() < 1. && (size.y - self.size[1]).abs() < 1.;
        // Let modal layout and smooth scroll-to-reconstruction finish before
        // capturing; a burst of initial frames can otherwise sample mid-scroll.
        if self.frames >= 8
            && self.started.elapsed() >= Duration::from_millis(600)
            && sized
            && !self.requested
            && crate::native_ui::reconstruction_ready(&self.app)
            && (!self.app.shell.bounds.contains_key("#review-tools")
                || crate::native_ui::tool_evidence(&self.app)["ready"] == true)
            && (std::env::var("CUBE_LAB_CAPTURE_VIEW").as_deref() != Ok("companion")
                || self
                    .output
                    .parent()
                    .unwrap()
                    .join("companion-window.png")
                    .exists())
        {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if self.started.elapsed() > Duration::from_secs(15) {
            *self.error.lock().unwrap() = Some(format!(
                "Timed out at logical size {size:?}, ppp {}",
                ctx.pixels_per_point()
            ));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(40));
    }
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        eframe::App::on_exit(&mut self.app, gl);
    }
}
