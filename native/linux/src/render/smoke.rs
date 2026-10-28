//! Isolated real-GL smoke capture: no model, network, Bluetooth, settings or history.
use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub fn run() -> eframe::Result {
    let directory = std::path::PathBuf::from(
        std::env::var("CUBE_LAB_RENDER_DIR").unwrap_or("test-results/rust".into()),
    );
    std::fs::create_dir_all(&directory).expect("Create GPU smoke directory");
    let saved = Arc::new(AtomicBool::new(false));
    let save_error = Arc::new(Mutex::new(None));
    let output = directory.join("gpu-cube.png");
    let capture_saved = saved.clone();
    let capture_error = save_error.clone();
    eframe::run_native(
        "Cube Lab · isolated GPU check",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([960., 1070.])
                .with_app_id("lab.cube.RenderCheck"),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            let gl = cc
                .gl
                .as_ref()
                .ok_or_else(|| std::io::Error::other("Native Glow context unavailable"))?;
            let renderer = gpu::Renderer::new(gl.clone()).map_err(std::io::Error::other)?;
            cc.egui_ctx
                .data_mut(|d| d.insert_temp(egui::Id::new("cube.native.gl"), renderer));
            Ok(Box::new(Capture {
                ctx: cc.egui_ctx.clone(),
                frames: 0,
                requested: false,
                output,
                saved: capture_saved,
                error: capture_error,
            }))
        }),
    )?;
    if let Some(error) = save_error.lock().unwrap().take() {
        panic!("GPU screenshot failed: {error}");
    }
    assert!(
        saved.load(Ordering::Relaxed),
        "GPU screenshot was not captured"
    );
    println!(
        "Captured real native OpenGL geometry to {}",
        directory.join("gpu-cube.png").display()
    );
    Ok(())
}

struct Capture {
    ctx: egui::Context,
    frames: usize,
    requested: bool,
    output: std::path::PathBuf,
    saved: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}
impl eframe::App for Capture {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                let bytes = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let capture =
                    image::RgbaImage::from_raw(image.width() as u32, image.height() as u32, bytes)
                        .unwrap();
                match capture.save(&self.output) {
                    Ok(()) => self.saved.store(true, Ordering::Relaxed),
                    Err(error) => *self.error.lock().unwrap() = Some(error.to_string()),
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(30, 34, 51))
                    .inner_margin(20),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Cube Lab · native OpenGL · isolated geometry check")
                        .size(17.)
                        .color(Color32::from_rgb(223, 222, 236)),
                );
                ui.add_space(5.);
                egui::Grid::new("poses").spacing([20., 4.]).show(ui, |ui| {
                    for (index, (label, yaw, pitch, turn)) in [
                        ("Stickerless shell", HOME_YAW, HOME_PITCH, None),
                        (
                            "Halfway through R",
                            HOME_YAW,
                            HOME_PITCH,
                            Some((Move::parse("R").unwrap(), 0.5)),
                        ),
                        ("Shallow perspective", -0.58, 0.08, None),
                        (
                            "Top / quarter U",
                            -0.25,
                            1.1,
                            Some((Move::parse("U").unwrap(), 0.25)),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(label)
                                    .size(12.)
                                    .color(Color32::from_rgb(173, 177, 199)),
                            );
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(440., 300.), egui::Sense::hover());
                            paint_cube(
                                ui.painter_at(rect),
                                rect,
                                Cube::default(),
                                yaw,
                                pitch,
                                turn,
                            );
                        });
                        if index % 2 == 1 {
                            ui.end_row();
                        }
                    }
                    for (label, elapsed) in [
                        (
                            "Paired R + L' · halfway · gyro Y90",
                            Duration::from_micros(57_500),
                        ),
                        (
                            "Paired R + L' · committed endpoint · gyro Y90",
                            Duration::from_millis(250),
                        ),
                    ] {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(label)
                                    .size(12.)
                                    .color(Color32::from_rgb(173, 177, 199)),
                            );
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(440., 300.), egui::Sense::hover());
                            // Exercise the same timestamp grouping and calibrated
                            // gyro used by live/replay animation, without hardware.
                            let mut animator = Animator::default();
                            let s = std::f32::consts::FRAC_1_SQRT_2;
                            animator.set_orientation(Some([s, 0., 0., s]));
                            animator.set_orientation(Some([0.5, 0.5, 0.5, 0.5]));
                            let start = Instant::now() + Duration::from_secs(1);
                            animator.advance_at(start);
                            animator.push_timed_at(
                                Move::parse("R").unwrap(),
                                Some(0.),
                                false,
                                start,
                            );
                            animator.push_timed_at(
                                Move::parse("L'").unwrap(),
                                Some(40.),
                                false,
                                start,
                            );
                            animator.advance_at(start);
                            let motion = animator.advance_at(start + elapsed);
                            paint_oriented(
                                ui.painter_at(rect),
                                rect,
                                animator.cube,
                                HOME_YAW,
                                HOME_PITCH,
                                motion,
                                animator.orientation,
                                animator.mark_turns,
                            );
                        });
                    }
                    ui.end_row();
                });
            });
        self.frames += 1;
        if self.frames >= 3 && !self.requested {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint_after(Duration::from_millis(30));
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        destroy_glow(&self.ctx);
    }
}
