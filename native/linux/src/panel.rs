//! Local session-bus companion. Only live presentation data leaves the app.
use eframe::egui;
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[derive(Debug, PartialEq)]
pub(crate) enum CubeControl {
    Scan,
    Connect(String),
    Disconnect,
    PowerOn,
}

pub(crate) fn device_key(address: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    address.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

const NAME: &str = "org.cstimer.Native.Companion";
const PATH: &str = "/org/cstimer/Native/Companion";
struct Shared {
    snapshot: Mutex<String>,
    widget: AtomicBool,
    panel: AtomicBool,
    show: AtomicBool,
    hide: AtomicBool,
    quit: AtomicBool,
    cube_controls: Mutex<VecDeque<CubeControl>>,
    ctx: egui::Context,
}
struct Service(Arc<Shared>);
impl Service {
    fn enqueue(&self, command: CubeControl) -> zbus::fdo::Result<()> {
        let mut commands = self
            .0
            .cube_controls
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if commands.len() >= 8 {
            return Err(zbus::fdo::Error::LimitsExceeded(
                "Wait for the previous cube action".into(),
            ));
        }
        commands.push_back(command);
        self.0.ctx.request_repaint();
        Ok(())
    }
}
#[zbus::interface(name = "org.cstimer.Native.Companion")]
impl Service {
    fn scan_cubes(&self) -> zbus::fdo::Result<()> {
        self.enqueue(CubeControl::Scan)
    }
    fn connect_cube(&self, device: String) -> zbus::fdo::Result<()> {
        if device.len() > 32 {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Choose a cube from the list".into(),
            ));
        }
        self.enqueue(CubeControl::Connect(device))
    }
    fn disconnect_cube(&self) -> zbus::fdo::Result<()> {
        self.enqueue(CubeControl::Disconnect)
    }
    fn power_on_bluetooth(&self) -> zbus::fdo::Result<()> {
        self.enqueue(CubeControl::PowerOn)
    }
    fn get_snapshot(&self) -> String {
        // The event-loop model pump publishes independently of window frames.
        // Reading the panel must not redraw the entire dashboard.
        self.0
            .snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    fn show_main(&self) {
        self.0.show.store(true, Ordering::Relaxed);
        self.0.ctx.request_repaint();
    }
    fn hide_main(&self) {
        self.0.hide.store(true, Ordering::Relaxed);
        self.0.ctx.request_repaint();
    }
    fn quit(&self) {
        self.0.quit.store(true, Ordering::Relaxed);
        self.0.ctx.request_repaint();
    }
    fn show_panel_companion(&self) {
        self.0.panel.store(true, Ordering::Relaxed);
        self.0.ctx.request_repaint();
    }
    fn show_companion(&self) {
        self.0.widget.store(true, Ordering::Relaxed);
        self.0.ctx.request_repaint();
    }
}
pub struct Bridge {
    shared: Arc<Shared>,
    _stop: mpsc::Sender<()>,
}
impl Bridge {
    pub fn new(ctx: egui::Context) -> Self {
        let shared=Arc::new(Shared{snapshot:Mutex::new("{\"connected\":false,\"running\":false,\"time\":\"—\",\"tpsLabel\":\"—\",\"moves\":null,\"tokens\":[]}".into()),widget:AtomicBool::new(false),panel:AtomicBool::new(false),show:AtomicBool::new(false),hide:AtomicBool::new(false),quit:AtomicBool::new(false),cube_controls:Mutex::new(VecDeque::new()),ctx});
        let (tx, rx) = mpsc::channel();
        let service = Service(shared.clone());
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<()> {
                let _connection = zbus::blocking::connection::Builder::session()?
                    .name(NAME)?
                    .serve_at(PATH, service)?
                    .build()?;
                let _ = rx.recv();
                Ok(())
            })();
            if let Err(error) = result {
                eprintln!("GNOME companion bus: {error}");
            }
        });
        Self { shared, _stop: tx }
    }
    pub fn publish(&self, value: serde_json::Value) {
        *self
            .shared
            .snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = value.to_string();
    }
    pub fn take_show(&self) -> bool {
        self.shared.show.swap(false, Ordering::Relaxed)
    }
    pub(crate) fn take_cube_controls(&self) -> Vec<CubeControl> {
        self.shared
            .cube_controls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect()
    }
    pub fn take_hide(&self) -> bool {
        self.shared.hide.swap(false, Ordering::Relaxed)
    }
    pub fn take_quit(&self) -> bool {
        self.shared.quit.swap(false, Ordering::Relaxed)
    }
    pub fn take_panel_request(&self) -> bool {
        self.shared.panel.swap(false, Ordering::Relaxed)
    }
    pub fn take_widget(&self) -> bool {
        self.shared.widget.swap(false, Ordering::Relaxed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_and_control_requests_are_consumed_once() {
        let b = Bridge {
            shared: Arc::new(Shared {
                snapshot: Mutex::new("{}".into()),
                widget: AtomicBool::new(false),
                panel: AtomicBool::new(false),
                show: AtomicBool::new(false),
                hide: AtomicBool::new(false),
                quit: AtomicBool::new(false),
                cube_controls: Mutex::new(VecDeque::new()),
                ctx: egui::Context::default(),
            }),
            _stop: mpsc::channel().0,
        };
        b.publish(serde_json::json!({"time":"13.73","tpsLabel":"1.89","moves":26}));
        let s = Service(b.shared.clone());
        assert!(s.get_snapshot().contains("26"));
        s.show_main();
        assert!(b.take_show());
        assert!(!b.take_show());
        s.hide_main();
        assert!(b.take_hide());
        assert!(!b.take_hide());
        s.quit();
        assert!(b.take_quit());
        assert!(!b.take_quit());
        s.show_panel_companion();
        assert!(b.take_panel_request());
        assert!(!b.take_panel_request());
        assert!(
            !b.take_widget(),
            "Top-bar requests must not open a floating window"
        );
        s.show_companion();
        assert!(b.take_widget());
        assert!(!b.take_widget());
        s.scan_cubes().unwrap();
        s.connect_cube("saved".into()).unwrap();
        s.disconnect_cube().unwrap();
        s.power_on_bluetooth().unwrap();
        assert_eq!(
            b.take_cube_controls(),
            vec![
                CubeControl::Scan,
                CubeControl::Connect("saved".into()),
                CubeControl::Disconnect,
                CubeControl::PowerOn
            ]
        );
        assert!(b.take_cube_controls().is_empty());
        assert!(s.connect_cube("x".repeat(33)).is_err());
        for _ in 0..8 {
            s.scan_cubes().unwrap();
        }
        assert!(s.scan_cubes().is_err());
    }
}

/// Reuse the running Rust application, including its Bluetooth connection.
pub fn show_existing_companion() -> bool {
    (|| -> zbus::Result<()> {
        let connection = zbus::blocking::Connection::session()?;
        let proxy = zbus::blocking::Proxy::new(&connection, NAME, PATH, NAME)?;
        proxy.call("ShowCompanion", &())
    })()
    .is_ok()
}

/// Default companion entry point: a native Shell panel item, never a new window.
pub fn show_existing_panel() -> bool {
    (|| -> zbus::Result<()> {
        let connection = zbus::blocking::Connection::session()?;
        let proxy = zbus::blocking::Proxy::new(&connection, NAME, PATH, NAME)?;
        proxy.call("ShowPanelCompanion", &())
    })()
    .is_ok()
}

pub fn enable_top_bar(ctx: egui::Context) -> mpsc::Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let enabled = (|| -> zbus::Result<bool> {
            let connection = zbus::blocking::Connection::session()?;
            let proxy = zbus::blocking::Proxy::new(
                &connection,
                "org.gnome.Shell.Extensions",
                "/org/gnome/Shell/Extensions",
                "org.gnome.Shell.Extensions",
            )?;
            proxy.call("EnableExtension", &("cstimer-companion@local",))
        })();
        let result = match enabled {
            Ok(true) => Ok(()),
            Ok(false) => Err("GNOME has not discovered the installed companion yet. Sign out and back in once to load it.".into()),
            Err(_) => Err("GNOME top bar is unavailable. The floating companion is available with qbing --floating-companion.".into()),
        };
        let _ = tx.send(result);
        ctx.request_repaint();
    });
    rx
}

/// Presentation commands always reuse the one model and Bluetooth connection.
pub fn call_existing(method: &str) -> bool {
    (|| -> zbus::Result<()> {
        let connection = zbus::blocking::Connection::session()?;
        let proxy = zbus::blocking::Proxy::new(&connection, NAME, PATH, NAME)?;
        proxy.call(method, &())
    })()
    .is_ok()
}
