//! Private Wayland focus/occlusion fixture, never a real cube client.
fn main() -> eframe::Result {
    assert!(std::env::var("FACET_PRIVATE_TEST").as_deref() == Ok("1"));
    let report = std::env::var("FACET_COVER_REPORT").expect("Private focus report path");
    let mut previous = None;
    eframe::run_simple_native(
        "Other application",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_title("Other application")
                .with_app_id("org.facet.BackgroundTest")
                .with_fullscreen(true),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        move |ctx, _| {
            let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
            if previous != Some(focused) {
                std::fs::write(&report, format!("{{\"active\":{focused}}}")).unwrap();
                previous = Some(focused);
            }
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Other application · isolated focus test");
            });
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        },
    )
}
