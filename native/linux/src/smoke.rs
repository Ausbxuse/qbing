//! Render the actual egui draw output to PNG without a display, browser, or hardware.
use crate::{
    cube::{Cube, sequence},
    model::Target,
    ui::App,
};
use eframe::egui::{
    self, ColorImage, Pos2, Rect, TextureId, Vec2,
    epaint::{Mesh, Primitive},
};
use std::{collections::HashMap, path::Path};

#[path = "smoke_capture.rs"]
mod capture;
pub use capture::run as capture_native;

pub fn render_all(directory: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(directory)?;
    let fixture = std::env::var("CUBE_LAB_RENDER_FIXTURE")
        .ok()
        .map(std::fs::read)
        .transpose()?
        .map(|v| serde_json::from_slice::<serde_json::Value>(&v))
        .transpose()?;
    let mut count = 0;
    for theme in ["graphite", "snappy", "paper"] {
        for (width, height, label) in [(1180, 900, "desktop"), (480, 900, "compact")] {
            let ctx = egui::Context::default();
            let mut app = App::new(ctx.clone(), true);
            app.model.settings.theme = theme.into();
            if let Some(data) = &fixture {
                app.model.records = data["records"].as_array().cloned().unwrap_or_default();
                app.model.preferences = data["preferences"].clone();
                app.model.stats = data["stats"].as_array().cloned().unwrap_or_default();
                app.model.settings.session = "Preview session".into();
                app.selected = "pll-t".into();
                app.model.latest = app
                    .model
                    .records
                    .iter()
                    .rev()
                    .find(|r| r["kind"] == "solve")
                    .cloned();
                app.model.sync_status = "Preview · synthetic records".into();
            }
            let moves = sequence("D2 B2 U F2 D R2 F2 L2 U' L2 D2 F' R2 U' B R' U' L' D R2")?;
            app.model.target = Some(Target {
                cube: Cube::default().apply(&moves),
                moves,
                stage: "solve".into(),
                case: None,
                algorithm: None,
            });
            let mut textures = HashMap::<TextureId, ColorImage>::new();
            for page in [
                "Practice",
                "Scramble",
                "Training",
                "Library",
                "Case",
                "Insights",
                "History",
                "Settings",
                "Keyboard",
                "Half",
                "Correction",
                "Corrected",
                "Review",
            ] {
                if theme != "snappy" && !["Practice", "Library", "Settings"].contains(&page) {
                    continue;
                }
                app.keyboard.cancel();
                if page == "Keyboard" {
                    let now = std::time::Instant::now();
                    app.keyboard
                        .press(now - std::time::Duration::from_millis(1200));
                    app.keyboard
                        .release(now - std::time::Duration::from_millis(600));
                }
                if page == "Review" {
                    let Some(record) = app.model.latest.clone() else {
                        continue;
                    };
                    app.open_review(record);
                }
                app.page = if [
                    "Scramble",
                    "Keyboard",
                    "Review",
                    "Half",
                    "Correction",
                    "Corrected",
                ]
                .contains(&page)
                {
                    "Practice"
                } else {
                    page
                }
                .into();
                let tracking = ["Scramble", "Half", "Correction", "Corrected"].contains(&page);
                app.model.connected = tracking;
                app.model.powered = tracking;
                app.model.device_name = "Render fixture".into();
                app.model.recovery.clear();
                app.model.state = if page == "Half" {
                    Some(Cube::default().apply(&sequence("D2 B2 U F2 D R")?))
                } else if page == "Correction" {
                    app.model.recovery = sequence("B' U' R")?;
                    Some(Cube::default().apply(&sequence("D2 B2 U F2 D R U B")?))
                } else if page == "Corrected" {
                    Some(Cube::default().apply(&sequence("D2 B2 U F2 D R2")?))
                } else if page == "Scramble" {
                    Some(Cube::default().apply(&app.model.target.as_ref().unwrap().moves[..8]))
                } else {
                    None
                };
                app.animator.snap(app.model.state.unwrap_or_default());
                let mut output = None;
                for n in 0..3 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(width as f32, height as f32),
                        )),
                        time: Some(n as f64 / 10.),
                        ..Default::default()
                    };
                    let o = ctx.run(input, |ctx| app.draw(ctx));
                    for (id, delta) in &o.textures_delta.set {
                        let egui::ImageData::Color(image) = &delta.image;
                        if let Some([x, y]) = delta.pos {
                            let target = textures.get_mut(id).expect("Existing font atlas");
                            for row in 0..image.size[1] {
                                let start = (y + row) * target.size[0] + x;
                                target.pixels[start..start + image.size[0]].copy_from_slice(
                                    &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                                );
                            }
                        } else {
                            textures.insert(*id, (**image).clone());
                        }
                    }
                    output = Some(o);
                }
                let output = output.unwrap();
                let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
                let mut image = image::RgbaImage::from_pixel(width, height, {
                    let bg = crate::design::palette(&ctx).bg;
                    image::Rgba([bg.r(), bg.g(), bg.b(), 255])
                });
                for p in primitives {
                    if let Primitive::Mesh(mesh) = p.primitive {
                        paint(&mut image, &mesh, p.clip_rect, &textures);
                    }
                }
                let theme_label = if theme == "graphite" {
                    String::new()
                } else {
                    format!("-{theme}")
                };
                image.save(
                    directory.join(format!("{label}{theme_label}-{}.png", page.to_lowercase())),
                )?;
                count += 1;
            }
        }
    }
    println!(
        "Rendered {count} native Rust views to {}",
        directory.display()
    );
    render_cubes(directory)?;
    Ok(())
}

fn render_cubes(directory: &Path) -> anyhow::Result<()> {
    let solved = Cube::default();
    let r = crate::cube::Move::parse("R")?;
    let u = crate::cube::Move::parse("U")?;
    let save = |name: &str, cube, pitch, turn| -> anyhow::Result<()> {
        let pixels =
            crate::render::cube_image(cube, crate::render::HOME_YAW, pitch, turn, None, 480, 480);
        let mut image = image::RgbaImage::new(480, 480);
        for (target, source) in image.pixels_mut().zip(&pixels.pixels) {
            let rgba = source.to_array();
            let alpha = rgba[3] as f32 / 255.;
            let bg = [30., 34., 51.];
            *target = image::Rgba([
                (rgba[0] as f32 + bg[0] * (1. - alpha)).min(255.) as u8,
                (rgba[1] as f32 + bg[1] * (1. - alpha)).min(255.) as u8,
                (rgba[2] as f32 + bg[2] * (1. - alpha)).min(255.) as u8,
                255,
            ]);
        }
        image.save(directory.join(name))?;
        Ok(())
    };
    save("cube-home.png", solved, crate::render::HOME_PITCH, None)?;
    save(
        "cube-r-turn.png",
        solved,
        crate::render::HOME_PITCH,
        Some((r, 0.5)),
    )?;
    save(
        "cube-u-turn.png",
        solved.turn(r),
        crate::render::HOME_PITCH,
        Some((u, 0.5)),
    )?;
    save("cube-shallow.png", solved, 0.10, None)?;
    save("cube-top.png", solved, 1.25, None)?;
    if std::env::var_os("CUBE_LAB_RENDER_MOTION").is_some() {
        let moves = sequence("R U R' U' U R U' R'")?;
        std::fs::create_dir_all(directory.join("motion"))?;
        for frame in 0..144 {
            let time = frame as f32 / 30.;
            let step = ((time / 0.5).floor() as usize).min(moves.len());
            let cube = solved.apply(&moves[..step]);
            let fraction = ((time % 0.5) / 0.32).min(1.);
            let eased = fraction * fraction * fraction * (fraction * (fraction * 6. - 15.) + 10.);
            let turn = moves.get(step).map(|mv| (*mv, eased));
            save(
                &format!("motion/{frame:04}.png"),
                cube,
                crate::render::HOME_PITCH,
                turn,
            )?;
        }
    }
    Ok(())
}
fn paint(
    image: &mut image::RgbaImage,
    mesh: &Mesh,
    clip: Rect,
    textures: &HashMap<TextureId, ColorImage>,
) {
    let Some(texture) = textures.get(&mesh.texture_id) else {
        return;
    };
    for tri in mesh.indices.as_chunks::<3>().0 {
        let a = mesh.vertices[tri[0] as usize];
        let b = mesh.vertices[tri[1] as usize];
        let c = mesh.vertices[tri[2] as usize];
        let min_x = a
            .pos
            .x
            .min(b.pos.x)
            .min(c.pos.x)
            .max(clip.left())
            .max(0.)
            .floor() as u32;
        let max_x = a
            .pos
            .x
            .max(b.pos.x)
            .max(c.pos.x)
            .min(clip.right())
            .min(image.width() as f32)
            .ceil() as u32;
        let min_y = a
            .pos
            .y
            .min(b.pos.y)
            .min(c.pos.y)
            .max(clip.top())
            .max(0.)
            .floor() as u32;
        let max_y = a
            .pos
            .y
            .max(b.pos.y)
            .max(c.pos.y)
            .min(clip.bottom())
            .min(image.height() as f32)
            .ceil() as u32;
        let area =
            (b.pos.y - c.pos.y) * (a.pos.x - c.pos.x) + (c.pos.x - b.pos.x) * (a.pos.y - c.pos.y);
        if area.abs() < 0.0001 {
            continue;
        }
        for y in min_y..max_y {
            for x in min_x..max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let u = ((b.pos.y - c.pos.y) * (px - c.pos.x)
                    + (c.pos.x - b.pos.x) * (py - c.pos.y))
                    / area;
                let v = ((c.pos.y - a.pos.y) * (px - c.pos.x)
                    + (a.pos.x - c.pos.x) * (py - c.pos.y))
                    / area;
                let w = 1. - u - v;
                if u < 0. || v < 0. || w < 0. {
                    continue;
                }
                let uv = a.uv.to_vec2() * u + b.uv.to_vec2() * v + c.uv.to_vec2() * w;
                let tx =
                    (uv.x * texture.size[0] as f32 - 0.5).clamp(0., (texture.size[0] - 1) as f32);
                let ty =
                    (uv.y * texture.size[1] as f32 - 0.5).clamp(0., (texture.size[1] - 1) as f32);
                let ix = tx.floor() as usize;
                let iy = ty.floor() as usize;
                let fx = tx - ix as f32;
                let fy = ty - iy as f32;
                let pix = |dx: usize, dy: usize| {
                    texture.pixels[(iy + dy).min(texture.size[1] - 1) * texture.size[0]
                        + (ix + dx).min(texture.size[0] - 1)]
                    .to_array()
                };
                let p00 = pix(0, 0);
                let p10 = pix(1, 0);
                let p01 = pix(0, 1);
                let p11 = pix(1, 1);
                let ac = a.color.to_array();
                let bc = b.color.to_array();
                let cc = c.color.to_array();
                let mut source = [0.; 4];
                for k in 0..4 {
                    let tex = p00[k] as f32 * (1. - fx) * (1. - fy)
                        + p10[k] as f32 * fx * (1. - fy)
                        + p01[k] as f32 * (1. - fx) * fy
                        + p11[k] as f32 * fx * fy;
                    source[k] =
                        tex * (ac[k] as f32 * u + bc[k] as f32 * v + cc[k] as f32 * w) / 255.;
                }
                let target = image.get_pixel_mut(x, y);
                let alpha = source[3] / 255.;
                for (k, color) in source.iter().enumerate().take(3) {
                    target.0[k] = (color + target.0[k] as f32 * (1. - alpha)).clamp(0., 255.) as u8;
                }
            }
        }
    }
}
