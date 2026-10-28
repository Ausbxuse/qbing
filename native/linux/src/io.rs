use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub endpoint: String,
    pub token: String,
    pub mac: String,
    pub cross: String,
    pub session: String,
    pub theme: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: "https://zhenyuzhao.com/cube-api".into(),
            token: String::new(),
            mac: String::new(),
            cross: "U".into(),
            session: "Practice".into(),
            theme: "paper".into(),
        }
    }
}
#[derive(Clone, Deserialize)]
pub struct Catalog {
    pub cases: Vec<Case>,
}
#[derive(Clone, Deserialize)]
pub struct Case {
    pub id: String,
    pub stage: String,
    pub name: String,
    pub variants: Vec<Variant>,
}
#[derive(Clone, Deserialize)]
pub struct Variant {
    pub id: String,
    pub text: String,
    pub positions: std::collections::HashMap<String, Position>,
}
#[derive(Clone, Deserialize)]
pub struct Position {
    pub state: String,
    #[allow(dead_code)] // Reference solutions are exercised by the catalog validation tests.
    pub solution: Vec<String>,
    pub setup: Vec<String>,
}

pub fn check_connection() -> Result<usize> {
    let settings = Storage::new()?.settings();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let data = request(&client, &settings, "bootstrap", "GET", None)?;
    request(&client, &settings, "algorithm-stats", "GET", None)?;
    Ok(data["records"].as_array().map(Vec::len).unwrap_or(0))
}
pub fn catalog() -> Catalog {
    serde_json::from_str(include_str!(
        "../../shared/src/commonMain/composeResources/files/catalog.json"
    ))
    .expect("Validated bundled catalog")
}

pub struct Storage {
    pub directory: PathBuf,
}
impl Storage {
    pub fn new() -> Result<Self> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap()).join(".config"));
        let s = Self {
            directory: base.join("cstimer-native"),
        };
        fs::create_dir_all(&s.directory)?;
        permissions(&s.directory, 0o700)?;
        Ok(s)
    }
    pub fn read(&self, key: &str) -> Option<Value> {
        fs::read(self.directory.join(format!("{key}.json")))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    }
    pub fn write(&self, key: &str, value: &Value) -> Result<()> {
        let target = self.directory.join(format!("{key}.json"));
        atomic_write(&target, &serde_json::to_vec(value)?)
    }
    pub fn settings(&self) -> Settings {
        let mut s = self
            .read("settings")
            .and_then(|v| serde_json::from_value::<Settings>(v).ok())
            .unwrap_or_default();
        if s.token.is_empty() {
            let path = std::env::var_os("CSTIMER_NATIVE_CONFIG")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(".data/native-client-config.json"));
            if let Ok(bytes) = fs::read(path)
                && let Ok(config) = serde_json::from_slice::<Settings>(&bytes)
            {
                s = config;
                let _ = self.write("settings", &serde_json::to_value(&s).unwrap());
            }
        }
        s
    }
}
fn permissions(p: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(mode))?;
    Ok(())
}
pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(p) = target.parent() {
        fs::create_dir_all(p)?;
    }
    let temporary = target.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temporary, bytes)?;
    permissions(&temporary, 0o600)?;
    fs::rename(temporary, target)?;
    Ok(())
}

pub enum Work {
    Sync {
        settings: Settings,
        pending: Vec<Value>,
    },
    Request {
        settings: Settings,
        route: String,
        method: String,
        body: Option<Value>,
        tag: String,
    },
}
pub struct ApiEvent {
    pub tag: String,
    pub result: std::result::Result<Value, String>,
    pub sent: Vec<Value>,
}
pub fn api_worker(
    repaint: eframe::egui::Context,
) -> (mpsc::Sender<Work>, mpsc::Receiver<ApiEvent>) {
    let (tx, rx) = mpsc::channel();
    let (otx, orx) = mpsc::channel();
    let (serial_tx, serial_rx) = mpsc::channel();
    let (guidance_tx, guidance_rx) = mpsc::channel();
    // Keep scorebook writes ordered, while next-scramble and recovery requests
    // can complete even during a slow/offline history upload. Bound concurrency
    // to two guidance workers; model generation tags discard obsolete results.
    spawn_api_worker(
        Arc::new(Mutex::new(serial_rx)),
        otx.clone(),
        repaint.clone(),
    );
    let guidance = Arc::new(Mutex::new(guidance_rx));
    for _ in 0..2 {
        spawn_api_worker(guidance.clone(), otx.clone(), repaint.clone());
    }
    std::thread::spawn(move || {
        for work in rx {
            let independent = matches!(&work, Work::Request { route, .. } if route == "scramble" || route == "recovery");
            let queue = if independent {
                &guidance_tx
            } else {
                &serial_tx
            };
            if queue.send(work).is_err() {
                break;
            }
        }
    });
    (tx, orx)
}
fn spawn_api_worker(
    rx: Arc<Mutex<mpsc::Receiver<Work>>>,
    otx: mpsc::Sender<ApiEvent>,
    repaint: eframe::egui::Context,
) {
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(25))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        loop {
            let work = { rx.lock().unwrap().recv() };
            let Ok(work) = work else {
                break;
            };
            let event = match work {
                Work::Sync { settings, pending } => {
                    let result = (|| -> Result<Value> {
                        for r in &pending {
                            request(&client, &settings, "records", "POST", Some(r))?;
                        }
                        let bootstrap = request(&client, &settings, "bootstrap", "GET", None)?;
                        let stats = request(&client, &settings, "algorithm-stats", "GET", None)?;
                        Ok(json!({"bootstrap":bootstrap,"stats":stats}))
                    })();
                    ApiEvent {
                        tag: "sync".into(),
                        result: result.map_err(|e| e.to_string()),
                        sent: pending,
                    }
                }
                Work::Request {
                    settings,
                    route,
                    method,
                    body,
                    tag,
                } => ApiEvent {
                    tag,
                    result: request(&client, &settings, &route, &method, body.as_ref())
                        .map_err(|e| e.to_string()),
                    sent: vec![],
                },
            };
            if otx.send(event).is_err() {
                break;
            }
            repaint.request_repaint();
        }
    });
}
fn request(
    client: &reqwest::blocking::Client,
    s: &Settings,
    route: &str,
    method: &str,
    body: Option<&Value>,
) -> Result<Value> {
    if (s.token.is_empty() || route.starts_with("native-"))
        && matches!(
            route,
            "scramble"
                | "recovery"
                | "cross"
                | "native-tools"
                | "native-algorithms"
                | "native-review"
                | "native-technique"
        )
    {
        return crate::solver::request(route, body);
    }
    let url = reqwest::Url::parse(&format!("{}/{}", s.endpoint.trim_end_matches('/'), route))?;
    ensure!(
        url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))),
        "Use HTTPS for your server"
    );
    ensure!(
        !s.token.is_empty(),
        "Import your server connection in Settings"
    );
    let mut r = client.request(method.parse()?, url).bearer_auth(&s.token);
    if let Some(body) = body {
        r = r.json(body)
    }
    let response = r
        .send()
        .context("Cannot reach milkyway; attempts remain saved on this device")?;
    ensure!(
        response.status().as_u16() != 401,
        "Your server key is invalid. Import the connection file in Settings"
    );
    ensure!(
        response.status().is_success(),
        "Server returned {}",
        response.status()
    );
    Ok(response.json()?)
}

pub fn value_str<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn number(v: &Value, key: &str) -> f64 {
    v[key].as_f64().unwrap_or(0.)
}
pub fn seconds(v: f64) -> String {
    if !v.is_finite() {
        return "DNF".into();
    }
    if v >= 60_000. {
        format!("{}:{:05.2}", (v / 60000.) as u64, (v % 60000.) / 1000.)
    } else {
        format!("{:0.2}", v / 1000.)
    }
}
pub fn timed(r: &Value) -> bool {
    value_str(r, "status") == "complete"
        && value_str(r, "penalty") != "DNF"
        && !r["quality"].as_array().is_some_and(|q| {
            q.iter().any(|v| {
                matches!(
                    v.as_str(),
                    Some(
                        "missing-moves"
                            | "state-mismatch"
                            | "estimated-timing"
                            | "clock-reset"
                            | "unverified-final-state"
                    )
                )
            })
        })
}
pub fn adjusted(r: &Value) -> f64 {
    if value_str(r, "status") != "complete" || value_str(r, "penalty") == "DNF" {
        f64::INFINITY
    } else {
        number(r, "duration")
            + if value_str(r, "penalty") == "+2" {
                2000.
            } else {
                0.
            }
    }
}
pub fn average(records: &[Value], n: usize) -> Option<f64> {
    if records.len() < n {
        return None;
    }
    let mut times = records[records.len() - n..]
        .iter()
        .map(adjusted)
        .collect::<Vec<_>>();
    times.sort_by(f64::total_cmp);
    let trim = if n >= 5 {
        (n as f64 * 0.05).ceil() as usize
    } else {
        0
    };
    let a = &times[trim..n - trim];
    Some(a.iter().sum::<f64>() / a.len() as f64)
}
