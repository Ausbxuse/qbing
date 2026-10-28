//! Keep the single cube model alive when Wayland withholds window redraws.
use crate::ui::App;
use eframe::egui;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

type Shared = Rc<RefCell<Option<App>>>;
struct View(Shared);
impl eframe::App for View {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(app) = self.0.borrow_mut().as_mut() {
            eframe::App::update(app, ctx, frame);
        }
    }
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        if let Some(app) = self.0.borrow_mut().as_mut() {
            eframe::App::on_exit(app, gl);
        }
    }
}
struct Runtime<'a> {
    gui: eframe::EframeWinitApplication<'a>,
    state: Shared,
    root: Option<WindowId>,
    next_tick: Instant,
    last_pass: u64,
    last_render: Instant,
}
impl Runtime<'_> {
    fn tick(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now < self.next_tick {
            return;
        }
        let mut redraw = false;
        let mut quit = false;
        let mut interval = Duration::from_millis(250);
        if let Ok(mut state) = self.state.try_borrow_mut() {
            if let Some(app) = state.as_mut() {
                app.handle_cube_controls();
                app.model.capture_view_frame(app.animator.live_view_frame());
                app.model.tick();
                app.animator.set_orientation(app.model.orientation);
                app.model.capture_view_frame(app.animator.live_view_frame());
                let pass = app.ctx.cumulative_pass_nr();
                if pass != self.last_pass {
                    self.last_pass = pass;
                    self.last_render = now;
                }
                let connected = app.model.connected;
                if connected {
                    app.keyboard.cancel();
                }
                let active = app.model.active.is_some();
                if active {
                    interval = Duration::from_millis(33);
                } else if connected {
                    interval = Duration::from_millis(100);
                }
                if let Some(panel) = &app.panel {
                    quit = panel.take_quit();
                    let show = panel.take_show();
                    let hide = panel.take_hide();
                    let widget = panel.take_widget();
                    let top_bar = panel.take_panel_request();
                    if show {
                        app.window_visible = true;
                        app.ctx
                            .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        app.ctx
                            .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        app.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        redraw = true;
                    }
                    if hide {
                        let ctx = app.ctx.clone();
                        app.keep_in_background(&ctx);
                        redraw = true;
                    }
                    if widget {
                        app.shell.widget = true;
                        redraw = true;
                    }
                    if top_bar {
                        let ctx = app.ctx.clone();
                        crate::native_ui::open_panel(app, &ctx);
                        redraw = true;
                    }
                }
                if !app.window_visible
                    || now.duration_since(self.last_render) > Duration::from_millis(500)
                {
                    if let Some(cube) = app.model.state {
                        app.animator.snap(cube);
                    }
                    app.model.animation_moves.clear();
                    app.model.visual_reset = None;
                }
                if let Some(panel) = &app.panel {
                    panel.publish(crate::native_ui::companion_snapshot(app));
                }
            }
        }
        self.next_tick = now + interval;
        if quit {
            event_loop.exit();
            return;
        }
        // Window-control commands need one UI pass even if no frame callback arrives.
        if redraw {
            if let Some(root) = self.root {
                self.gui
                    .window_event(event_loop, root, WindowEvent::RedrawRequested);
            }
        }
    }
}
impl ApplicationHandler<eframe::UserEvent> for Runtime<'_> {
    fn resumed(&mut self, e: &ActiveEventLoop) {
        self.gui.resumed(e);
    }
    fn suspended(&mut self, e: &ActiveEventLoop) {
        self.gui.suspended(e);
    }
    fn exiting(&mut self, e: &ActiveEventLoop) {
        self.gui.exiting(e);
    }
    fn memory_warning(&mut self, e: &ActiveEventLoop) {
        self.gui.memory_warning(e);
    }
    fn device_event(&mut self, e: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        self.gui.device_event(e, id, event);
    }
    fn user_event(&mut self, e: &ActiveEventLoop, event: eframe::UserEvent) {
        self.gui.user_event(e, event);
        self.tick(e);
    }
    fn window_event(&mut self, e: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.root.get_or_insert(id);
        self.gui.window_event(e, id, event);
    }
    fn new_events(&mut self, e: &ActiveEventLoop, cause: StartCause) {
        self.gui.new_events(e, cause);
        self.tick(e);
    }
    fn about_to_wait(&mut self, e: &ActiveEventLoop) {
        self.tick(e);
        self.gui.about_to_wait(e);
        match e.control_flow() {
            ControlFlow::Wait => e.set_control_flow(ControlFlow::WaitUntil(self.next_tick)),
            ControlFlow::WaitUntil(t) => {
                e.set_control_flow(ControlFlow::WaitUntil(t.min(self.next_tick)))
            }
            ControlFlow::Poll => {}
        }
    }
}
pub fn run(
    options: eframe::NativeOptions,
    creator: impl FnOnce(&eframe::CreationContext<'_>) -> App + 'static,
) -> eframe::Result {
    let state: Shared = Rc::new(RefCell::new(None));
    let shared = state.clone();
    let event_loop = EventLoop::<eframe::UserEvent>::with_user_event().build()?;
    let gui = eframe::create_native(
        "org.cstimer.Native",
        options,
        Box::new(move |cc| {
            *shared.borrow_mut() = Some(creator(cc));
            Ok(Box::new(View(shared.clone())))
        }),
        &event_loop,
    );
    let mut runtime = Runtime {
        gui,
        state,
        root: None,
        next_tick: Instant::now(),
        last_pass: 0,
        last_render: Instant::now(),
    };
    event_loop.run_app(&mut runtime)?;
    Ok(())
}
