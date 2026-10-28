mod bluetooth;
mod case_analysis;
mod cube;
mod design;
mod gan;
mod graph;
mod import;
mod io;
mod keyboard;
mod model;
mod native_stats;
mod native_ui;
mod notation;
mod panel;
mod phase_time;
mod practice_ui;
mod recording;
mod render;
mod review_analysis;
mod review_navigation;
mod review_ui;
mod runtime;
mod scramble;
mod smoke;
mod solver;
mod stats_ui;
#[cfg(test)]
mod tests;
mod timing;
mod training_ui;
mod ui;
mod workspace_layout;
fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--capture-native") {
        return smoke::capture_native();
    }
    if std::env::args().any(|a| a == "--render-gpu-smoke") {
        return render::gpu_smoke();
    }
    if std::env::args().any(|a| a == "--check-api") {
        let n = io::check_connection().expect("API connection failed");
        println!("Authenticated HTTPS connection to milkyway verified; {n} personal records.");
        return Ok(());
    }
    if std::env::args().any(|a| a == "--bluetooth-status") {
        let result = tokio::runtime::Runtime::new().unwrap().block_on(async {
            let session = bluer::Session::new().await?;
            let adapter = session.default_adapter().await?;
            adapter.is_powered().await
        });
        match result {
            Ok(true) => println!("BlueZ: Bluetooth is on"),
            Ok(false) => println!("BlueZ: Bluetooth is off"),
            Err(e) => println!("BlueZ unavailable: {e}"),
        };
        return Ok(());
    }
    if std::env::args().any(|a| a == "--probe") {
        if let Err(e) = probe() {
            eprintln!("Cube probe: {e:#}");
            std::process::exit(2);
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--render-smoke") {
        let output = std::env::var("CUBE_LAB_RENDER_DIR").unwrap_or("test-results/rust".into());
        smoke::render_all(std::path::Path::new(&output)).expect("Native render smoke check");
        return Ok(());
    }
    if std::env::args().any(|a| a == "--quit" || a == "--hide") {
        let method = if std::env::args().any(|a| a == "--quit") {
            "Quit"
        } else {
            "HideMain"
        };
        if !panel::call_existing(method) {
            eprintln!("Qbing is not running.");
        }
        return Ok(());
    }
    let daemon = std::env::args().any(|a| a == "--daemon");
    let companion = std::env::args().any(|a| a == "--companion" || a == "--floating-companion");
    if !daemon && !companion && panel::call_existing("ShowMain") {
        return Ok(());
    }
    if std::env::args().any(|a| a == "--floating-companion") && panel::show_existing_companion() {
        return Ok(());
    }
    if std::env::args().any(|a| a == "--companion") && panel::show_existing_panel() {
        return Ok(());
    }
    let options = eframe::NativeOptions {
        // A Wayland EGL swap interval of one can block the model thread while
        // the compositor withholds frame callbacks for a covered window.
        vsync: std::env::var_os("WAYLAND_DISPLAY").is_none(),
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Qbing")
            .with_visible(!daemon)
            .with_icon(design::app_icon())
            .with_inner_size([1440., 1000.])
            .with_min_inner_size([360., 400.])
            .with_app_id("org.cstimer.Native"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    runtime::run(options, move |cc| {
        if let Some(gl) = &cc.gl {
            render::install_glow(&cc.egui_ctx, gl.clone());
        }
        let isolated = std::env::args().any(|a| a == "--isolated");
        let mut app = ui::App::new(cc.egui_ctx.clone(), isolated);
        if isolated {
            app.panel = Some(panel::Bridge::new(cc.egui_ctx.clone()));
        }
        app.background = daemon;
        app.window_visible = !daemon;
        if daemon {
            app.keep_in_background(&cc.egui_ctx);
            native_ui::open_panel(&mut app, &cc.egui_ctx);
        }
        if std::env::args().any(|a| a == "--floating-companion") {
            app.shell.widget = true;
        }
        if std::env::args().any(|a| a == "--companion") {
            native_ui::open_panel(&mut app, &cc.egui_ctx);
        }
        app
    })
}

fn probe() -> anyhow::Result<()> {
    use bluetooth::{Command, Event};
    use std::time::{Duration, Instant};
    let settings = io::Storage::new()?.settings();
    let cipher = gan::Cipher::new(&settings.mac)?;
    let (tx, rx) = bluetooth::worker(eframe::egui::Context::default());
    let deadline = Instant::now() + Duration::from_secs(25);
    let mut started = false;
    let mut verified = false;
    let result = (|| -> anyhow::Result<()> {
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Event::Adapter { powered, reason }) => {
                    anyhow::ensure!(powered, "{reason}");
                    if !started {
                        tx.send(Command::Connect(settings.mac.clone()))?;
                        started = true;
                    }
                }
                Ok(Event::Connected(name)) => {
                    println!("Connected to {name} using native BlueZ");
                    for name in ["state", "battery"] {
                        tx.send(Command::Write(cipher.command(name)))?;
                    }
                }
                Ok(Event::Bytes(bytes, _)) => {
                    for packet in cipher.decode(&bytes)? {
                        match packet {
                            gan::Packet::State { cube, serial } => {
                                anyhow::ensure!(cube.valid(), "Invalid cube state");
                                println!("Verified cube state; serial {serial}");
                                verified = true;
                            }
                            gan::Packet::Battery(b) => println!("Battery: {b}%"),
                            _ => {}
                        }
                    }
                    if verified {
                        break;
                    }
                }
                Ok(Event::Error(e) | Event::Disconnected(e)) => anyhow::bail!(e),
                _ => {}
            }
        }
        anyhow::ensure!(
            verified,
            "No valid state received before timeout. Wake the cube and close other cube apps."
        );
        Ok(())
    })();
    let _ = tx.send(Command::Disconnect);
    let _ = rx.recv_timeout(Duration::from_secs(1));
    result
}
