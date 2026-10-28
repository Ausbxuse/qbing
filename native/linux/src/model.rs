mod cfop;

use crate::{
    bluetooth::{self, Command, DeviceInfo, Event},
    cube::{Cube, Move, sequence},
    gan::{Cipher, Packet, Stream, StreamEvent},
    io::{self, ApiEvent, Catalog, Position, Settings, Storage, Work, number, value_str},
};
use serde_json::{Value, json};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Target {
    pub cube: Cube,
    pub moves: Vec<Move>,
    pub stage: String,
    pub case: Option<String>,
    pub algorithm: Option<String>,
}
pub struct Active {
    pub id: String,
    pub created: String,
    pub initial: Cube,
    pub target: Target,
    pub ready: Instant,
    pub started: Option<Instant>,
    /// First device timestamp, or a provisional origin when leading turns
    /// were recovered. The raw first event still qualifies endpoint accuracy.
    pub clock: Option<f64>,
    /// Largest observed device/local offset. This keeps the live timer moving
    /// when a later Bluetooth notification has less latency than the first.
    live_clock_offset: f64,
    pub moves: Vec<Value>,
    pub quality: Vec<String>,
    pub cross: String,
    pub session: String,
    pub scramble_moves: Vec<Value>,
    pub scramble_quality: Vec<String>,
    rotations: Vec<Value>,
    gyro_recorder: crate::render::orientation::Recorder,
}
impl Active {
    fn new(cube: Cube, target: Target, s: &Settings) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created: chrono::Utc::now().to_rfc3339(),
            initial: cube,
            target,
            ready: Instant::now(),
            started: None,
            clock: None,
            live_clock_offset: 0.,
            moves: vec![],
            quality: vec![],
            cross: s.cross.clone(),
            session: s.session.clone(),
            scramble_moves: vec![],
            scramble_quality: vec![],
            rotations: vec![],
            gyro_recorder: Default::default(),
        }
    }
    fn record(&self, status: &str) -> Value {
        let recognition = self
            .started
            .map(|t| t.saturating_duration_since(self.ready).as_secs_f64() * 1000.);
        let mut r = json!({"id":self.id,"createdAt":self.created,"kind":if self.target.case.is_some(){"drill"}else{"solve"},"source":"bluetooth","status":status,"penalty":if status=="complete"{""}else{"DNF"},"duration":self.moves.last().map(|v|number(v,"t")).unwrap_or(0.),"initialState":self.initial.to_string(),"cross":self.cross,"session":self.session,"stage":self.target.stage,"caseId":self.target.case,"selectedAlgorithmId":self.target.algorithm,"scramble":self.target.moves.iter().map(ToString::to_string).collect::<Vec<_>>().join(" "),"moves":self.moves,"quality":self.quality,"inspection":recognition,"notes":""});
        if self.target.case.is_some() {
            r["recognitionSource"] = json!("cue");
            r["recognitionMs"] = json!(recognition);
        }
        if let Some(pose) = self.gyro_recorder.initial_pose() {
            r["rotationFrame"] = json!("live-v1");
            r["initialOrientation"] = json!(pose);
        }
        r["analysis"] = analysis(&r);
        r["rotations"] = json!(
            self.rotations
                .iter()
                .filter(|v| number(v, "t") <= number(&r, "duration"))
                .collect::<Vec<_>>()
        );
        r["scrambleMoves"] = json!(self.scramble_moves);
        r["scrambleQuality"] = json!(self.scramble_quality);
        r["scrambleStats"] = crate::scramble::stats(&self.scramble_moves, &self.scramble_quality);
        r
    }
}

const COMPLETION_SETTLE: Duration = Duration::from_millis(120);
const AXIS_CONTINUATION_MS: f64 = 100.;

#[derive(Clone, Copy)]
struct CompletionCandidate {
    serial: u8,
    mv: Move,
    clock: Option<f64>,
    at: Instant,
    recovered: bool,
    verified: bool,
}
impl CompletionCandidate {
    fn continues_axis_motion(
        self,
        serial: u8,
        mv: Move,
        clock: Option<f64>,
        recovered: bool,
        at: Instant,
    ) -> bool {
        if serial != self.serial.wrapping_add(1) || self.mv.axis().0 != mv.axis().0 {
            return false;
        }
        if !self.recovered
            && !recovered
            && let Some((before, after)) = self.clock.zip(clock)
            && before.is_finite()
            && after.is_finite()
            && after >= before
        {
            return after - before <= AXIS_CONTINUATION_MS;
        }
        // Recovered turns lack reliable device timestamps. Keep plausible
        // continuing axis motion in the attempt, including R R L' L' bursts,
        // rather than ending halfway through a physical half/slice turn.
        at.saturating_duration_since(self.at).as_secs_f64() * 1000. <= AXIS_CONTINUATION_MS
    }
}
pub struct Model {
    pub revision: u64,
    pub settings: Settings,
    pub catalog: Catalog,
    pub records: Vec<Value>,
    pub pending: Vec<Value>,
    pub stats: Vec<Value>,
    pub preferences: Value,
    pub powered: bool,
    pub adapter_status: String,
    pub connected: bool,
    pub connecting: bool,
    pub scanning: bool,
    pub devices: Vec<DeviceInfo>,
    pub device_name: String,
    pub battery: Option<u8>,
    pub state: Option<Cube>,
    pub target: Option<Target>,
    pub active: Option<Active>,
    pub cue_ready: bool,
    pub countdown: Option<Instant>,
    pub latest: Option<Value>,
    pub message: String,
    pub sync_status: String,
    pub syncing: bool,
    pub animation_moves: Vec<(Move, Option<f64>, bool)>,
    pub visual_reset: Option<Cube>,
    pub scramble: crate::scramble::Tracker,
    pub recovery: Vec<Move>,
    pub guidance_checkpoint: Option<Cube>,
    pub recovery_busy: bool,
    pub recovery_error: Option<String>,
    pub scramble_loading: bool,
    pub orientation: Option<[f32; 4]>,
    view_frame: Option<([f32; 4], [f32; 4])>,
    pub gyro_live: bool,
    orientation_at: Option<Instant>,
    target_generation: u64,
    recovery_revision: u64,
    recovery_due: Option<Instant>,
    recovery_request: Option<(u64, Cube, Cube)>,
    storage: Option<Storage>,
    api_tx: mpsc::Sender<Work>,
    api_rx: mpsc::Receiver<ApiEvent>,
    ble_tx: Option<tokio::sync::mpsc::UnboundedSender<Command>>,
    ble_rx: Option<mpsc::Receiver<Event>>,
    cipher: Option<Cipher>,
    stream: Stream,
    completion: Option<CompletionCandidate>,
    verification: Option<(String, u8)>,
    connection_started: Option<Instant>,
    state_requests: u8,
    disconnect_reason: Option<String>,
    last_sync: Instant,
    last_draft: Instant,
}
impl Model {
    pub fn new(ctx: eframe::egui::Context, isolated: bool) -> Self {
        let storage = if isolated { None } else { Storage::new().ok() };
        let settings = storage.as_ref().map(Storage::settings).unwrap_or_default();
        let read = |key| storage.as_ref().and_then(|s| s.read(key));
        let cache = read("cache").unwrap_or(json!({}));
        let mut pending: Vec<Value> = read("pending")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let pending_refreshed = crate::recording::refresh_records(&mut pending);
        let mut records = cache["records"].as_array().cloned().unwrap_or_default();
        let records_refreshed = crate::recording::refresh_records(&mut records);
        for r in &pending {
            records.retain(|v| v["id"] != r["id"]);
            records.push(r.clone());
        }
        records.sort_by(|a, b| value_str(a, "createdAt").cmp(value_str(b, "createdAt")));
        let stats = read("analytics")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        let draft = read("draft");
        let (api_tx, api_rx) = io::api_worker(ctx.clone());
        let (ble_tx, ble_rx) = if isolated {
            (None, None)
        } else {
            let (tx, rx) = bluetooth::worker(ctx);
            (Some(tx), Some(rx))
        };
        let mut result = Self {
            revision: 0,
            scramble: Default::default(),
            recovery: vec![],
            guidance_checkpoint: None,
            recovery_busy: false,
            recovery_error: None,
            scramble_loading: false,
            orientation: None,
            view_frame: None,
            gyro_live: false,
            orientation_at: None,
            target_generation: 0,
            recovery_revision: 0,
            recovery_due: None,
            recovery_request: None,
            settings,
            catalog: io::catalog(),
            records,
            pending,
            stats,
            preferences: cache["preferences"]
                .as_object()
                .cloned()
                .map(Value::Object)
                .unwrap_or(json!({})),
            powered: false,
            adapter_status: if isolated {
                "Bluetooth is off".into()
            } else {
                "Checking Bluetooth…".into()
            },
            connected: false,
            connecting: false,
            scanning: false,
            devices: vec![],
            device_name: String::new(),
            battery: None,
            state: None,
            target: None,
            active: None,
            cue_ready: false,
            countdown: None,
            latest: None,
            message: String::new(),
            sync_status: "Offline".into(),
            syncing: false,
            animation_moves: vec![],
            visual_reset: None,
            storage,
            api_tx,
            api_rx,
            ble_tx,
            ble_rx,
            cipher: None,
            stream: Stream::default(),
            completion: None,
            verification: None,
            connection_started: None,
            state_requests: 0,
            disconnect_reason: None,
            last_sync: Instant::now(),
            last_draft: Instant::now(),
        };
        if pending_refreshed || records_refreshed {
            result.persist_history();
        }
        if let Some(mut r) = draft
            && r.is_object()
            && r["moves"].as_array().is_some_and(|m| !m.is_empty())
        {
            r["status"] = json!("interrupted");
            r["penalty"] = json!("DNF");
            if let Some(q) = r["quality"].as_array_mut() {
                q.push(json!("app-closed"));
            }
            result.save(r);
            result.persist("draft", &Value::Null);
        }
        if !isolated {
            result.sync();
            result.new_scramble();
        }
        result
    }
    fn persist(&mut self, key: &str, v: &Value) {
        if let Some(s) = &self.storage
            && let Err(e) = s.write(key, v)
        {
            self.message = format!("Could not save {key}: {e}");
        }
    }
    fn persist_history(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.persist("pending", &json!(self.pending));
        self.persist(
            "cache",
            &json!({"schema":1,"records":self.records,"preferences":self.preferences}),
        );
    }
    pub fn capture_view_frame(&mut self, frame: Option<([f32; 4], [f32; 4])>) {
        self.view_frame = frame;
        let elapsed = self.elapsed_at(Instant::now());
        if let (Some((basis, pose)), Some(active)) = (frame, self.active.as_mut()) {
            // Never mix an already-recorded legacy relative sequence with live poses.
            if active.rotations.is_empty() || active.gyro_recorder.has_live_frame() {
                if let Some((notation, pose)) =
                    active
                        .gyro_recorder
                        .set_view_frame(basis, pose, active.started.is_some())
                {
                    active.rotations.push(
                        json!({"notation":notation,"pose":pose,"t":elapsed,"timing":"local-gyro"}),
                    );
                }
            }
        }
    }
    pub fn command(&self, c: Command) {
        if let Some(tx) = &self.ble_tx {
            let _ = tx.send(c);
        }
    }
    fn request(&self, route: &str, method: &str, body: Option<Value>, tag: &str) {
        let _ = self.api_tx.send(Work::Request {
            settings: self.settings.clone(),
            route: route.into(),
            method: method.into(),
            body,
            tag: tag.into(),
        });
    }
    pub fn sync(&mut self) {
        if self.syncing {
            return;
        }
        if self.settings.token.is_empty() {
            self.sync_status = "Local".into();
            return;
        }
        self.syncing = true;
        self.sync_status = "Syncing".into();
        self.last_sync = Instant::now();
        let _ = self.api_tx.send(Work::Sync {
            settings: self.settings.clone(),
            pending: self.pending.clone(),
        });
    }
    pub fn save(&mut self, r: Value) {
        let r = crate::recording::refresh_analysis(r);
        self.pending.retain(|v| v["id"] != r["id"]);
        self.pending.push(r.clone());
        self.records.retain(|v| v["id"] != r["id"]);
        self.records.push(r.clone());
        self.records
            .sort_by(|a, b| value_str(a, "createdAt").cmp(value_str(b, "createdAt")));
        if self.latest.as_ref().is_some_and(|v| v["id"] == r["id"]) {
            self.latest = Some(r);
        }
        self.persist_history();
        self.sync();
    }
    pub fn scan(&mut self) {
        if !self.powered {
            self.message = self.adapter_status.clone();
            return;
        }
        self.scanning = true;
        self.devices.clear();
        self.command(Command::Scan);
    }
    pub fn connect(&mut self, address: String) {
        if !self.powered {
            self.message = "Bluetooth is off. Turn it on before connecting.".into();
            return;
        }
        if self.connecting || self.connected {
            return;
        }
        match Cipher::new(&address) {
            Ok(cipher) => self.cipher = Some(cipher),
            Err(e) => {
                self.message = e.to_string();
                return;
            }
        }
        self.settings.mac = address.clone();
        self.scanning = false;
        self.connecting = true;
        self.message.clear();
        self.disconnect_reason = None;
        self.connection_started = None;
        self.state_requests = 0;
        self.stream.reset();
        self.state = None;
        self.command(Command::Connect(address));
    }
    pub fn disconnect(&mut self) {
        self.interrupt("connection-lost");
        self.connected = false;
        self.connecting = false;
        self.connection_started = None;
        self.state_requests = 0;
        self.state = None;
        self.orientation = None;
        self.gyro_live = false;
        self.stream.reset();
        self.command(Command::Disconnect);
    }
    pub fn read_state(&self) {
        if let Some(c) = &self.cipher {
            self.command(Command::Write(c.command("state")));
        }
    }
    fn request_initial_state(&mut self, at: Instant) {
        if self.connected && self.connection_started.is_none() {
            self.connection_started = Some(at);
            self.state_requests = 1;
            self.read_state();
        }
    }
    fn poll_initial_state(&mut self, at: Instant) {
        if !self.connected || self.state.is_some() {
            self.connection_started = None;
            self.state_requests = 0;
            return;
        }
        let Some(started) = self.connection_started else {
            return;
        };
        let elapsed = at.saturating_duration_since(started);
        if elapsed >= Duration::from_secs(6) {
            self.disconnect();
            let reason =
                "Cube did not return a valid state. Wake it, close other cube apps, and reconnect."
                    .to_string();
            self.message = reason.clone();
            self.disconnect_reason = Some(reason);
        } else if self.state_requests < 3
            && elapsed >= Duration::from_secs(if self.state_requests == 1 { 1 } else { 3 })
        {
            self.state_requests += 1;
            self.read_state();
        }
    }
    fn clear_guidance(&mut self) {
        self.recovery_revision = self.recovery_revision.wrapping_add(1);
        self.recovery.clear();
        self.recovery_busy = false;
        self.recovery_error = None;
        self.recovery_due = None;
        self.recovery_request = None;
    }
    fn refresh_guidance(&mut self) {
        let (Some(state), Some(target)) = (self.state, self.target.as_ref()) else {
            return;
        };
        if self.active.is_some() {
            self.clear_guidance();
            return;
        }
        if crate::cube::scramble_progress(state, &target.moves).matched {
            self.guidance_checkpoint = Some(state);
            self.clear_guidance();
            return;
        }
        // With no known prefix, repair to the solved origin and leave every
        // original scramble token pending. Never disguise a direct-to-target
        // solution as an undo at the beginning of the original notation.
        self.guidance_checkpoint.get_or_insert(Cube::default());
        if !self.recovery.is_empty()
            || self.recovery_request.is_some()
            || self.recovery_error.is_some()
        {
            return;
        }
        self.recovery_busy = true;
        self.recovery_due.get_or_insert(Instant::now());
    }
    pub fn calibrate(&mut self) {
        if self.active.is_some() {
            self.message = "Finish or cancel the attempt first".into();
            return;
        }
        self.target = None;
        self.guidance_checkpoint = None;
        self.scramble_loading = false;
        self.clear_guidance();
        self.target_generation += 1;
        self.stream.reset();
        if let Some(c) = &self.cipher {
            self.command(Command::Write(c.command("reset")));
            self.read_state();
        }
    }
    pub fn tick(&mut self) {
        let events = self
            .ble_rx
            .as_ref()
            .map(|rx| rx.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for event in events {
            self.bluetooth_event(event);
        }
        for event in self.api_rx.try_iter().collect::<Vec<_>>() {
            self.api_event(event);
        }
        self.gyro_live = self.connected
            && self.orientation.is_some()
            && self
                .orientation_at
                .is_some_and(|at| at.elapsed() < Duration::from_secs(2));
        if self
            .recovery_due
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(180))
        {
            self.recovery_due = None;
            if let (Some(state), Some(_)) = (self.state, self.target.as_ref()) {
                let target = self.guidance_checkpoint.unwrap_or_default();
                if self.active.is_none() {
                    let revision = self.recovery_revision;
                    self.recovery_request = Some((revision, state, target));
                    self.request(
                        "recovery",
                        "POST",
                        Some(json!({"state":state.to_string(),"target":target.to_string()})),
                        &format!("recovery:{revision}"),
                    );
                }
            }
        }
        let at = Instant::now();
        for event in self.stream.poll(at) {
            self.stream_event(event, at);
        }
        if self.stream.expire() {
            self.fault("missing-moves");
            self.request_initial_state(at);
        }
        self.settle_completion(at);
        self.poll_initial_state(at);
        if let Some(start) = self.countdown
            && start.elapsed() > Duration::from_secs(3)
        {
            self.countdown = None;
            if let (Some(state), Some(target)) = (self.state, self.target.clone())
                && state == target.cube
            {
                let mut a = Active::new(state, target, &self.settings);
                a.scramble_moves = self.scramble.moves.clone();
                a.scramble_quality = self.scramble.quality.clone();
                self.active = Some(a);
                self.cue_ready = false;
                self.message.clear();
            }
        }
        if !self.pending.is_empty() && self.last_sync.elapsed() > Duration::from_secs(15) {
            self.sync();
        }
    }
    fn bluetooth_event(&mut self, e: Event) {
        match e {
            Event::Adapter { powered, reason } => {
                self.powered = powered;
                self.adapter_status = reason;
                if !powered {
                    self.connection_started = None;
                    self.state_requests = 0;
                    self.scanning = false;
                    self.connecting = false;
                    if self.connected || self.active.is_some() {
                        self.fault("bluetooth-off");
                    }
                    self.connected = false;
                    self.state = None;
                    self.orientation = None;
                    self.gyro_live = false;
                    self.stream.reset();
                }
            }
            Event::Devices(d) => {
                self.scanning = false;
                self.devices = d;
                if self.devices.is_empty() {
                    self.message =
                        "No GAN cube found. Wake it with a turn, then scan again.".into();
                }
            }
            Event::Connected(name) => {
                if !self.powered || !self.connecting {
                    self.command(Command::Disconnect);
                    return;
                }
                self.connected = true;
                self.connecting = false;
                self.device_name = name;
                self.message.clear();
                self.request_initial_state(Instant::now());
                self.persist("settings", &serde_json::to_value(&self.settings).unwrap());
                if let Some(c) = &self.cipher {
                    for name in ["battery", "hardware"] {
                        self.command(Command::Write(c.command(name)));
                    }
                }
            }
            Event::Disconnected(reason) => {
                if self.connected || self.active.is_some() {
                    self.fault("connection-lost");
                }
                self.connected = false;
                self.connecting = false;
                self.connection_started = None;
                self.state_requests = 0;
                self.state = None;
                self.orientation = None;
                self.gyro_live = false;
                self.stream.reset();
                self.message = self.disconnect_reason.take().unwrap_or(reason);
            }
            Event::Error(reason) => {
                self.connecting = false;
                self.scanning = false;
                if self.active.is_some() {
                    self.fault("connection-lost");
                    // A fault clears the visible state. Reset its serial base
                    // too, otherwise the next snapshot only emits Verified.
                    self.stream.reset();
                    self.request_initial_state(Instant::now());
                }
                self.message = reason;
            }
            Event::Bytes(bytes, at) => {
                if !self.connected {
                    return;
                }
                let decoded = self.cipher.as_ref().map(|c| c.decode(&bytes));
                if let Some(result) = decoded {
                    match result {
                        Ok(packets) => {
                            for p in packets {
                                match p {
                                    Packet::Battery(b) => self.battery = Some(b),
                                    Packet::Orientation(q) => {
                                        let elapsed = self.elapsed_at(at);
                                        if let Some(active) = self.active.as_mut() {
                                            if let Some((basis, pose)) = self.view_frame {
                                                if active.rotations.is_empty()
                                                    || active.gyro_recorder.has_live_frame()
                                                {
                                                    active.gyro_recorder.set_view_frame(
                                                        basis,
                                                        pose,
                                                        active.started.is_some(),
                                                    );
                                                }
                                            }
                                            if let Some((notation, pose)) = active
                                                .gyro_recorder
                                                .observe(q, active.started.is_some())
                                            {
                                                active.rotations.push(json!({"notation":notation,"t":elapsed,"pose":pose,"timing":"local-gyro"}));
                                            }
                                        }
                                        self.orientation = Some(q);
                                        self.orientation_at = Some(at);
                                        self.gyro_live = true;
                                    }
                                    Packet::Disconnected => self.disconnect(),
                                    _ => {
                                        for e in self.stream.accept_at(p, at) {
                                            self.stream_event(e, at);
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            self.stream.reset();
                            self.fault("invalid-packet");
                            self.request_initial_state(at);
                            self.message =
                                "Could not decode a cube packet. Reading its state again…".into();
                        }
                    }
                }
            }
        }
    }
    fn stream_event(&mut self, e: StreamEvent, at: Instant) {
        match e {
            StreamEvent::State(cube) => {
                self.connection_started = None;
                self.state_requests = 0;
                self.clear_guidance();
                self.state = Some(cube);
                self.visual_reset = Some(cube);
                self.animation_moves.clear();
                self.message.clear();
                self.maybe_arm();
                self.refresh_guidance();
            }
            StreamEvent::History(serial, count) => {
                if let Some(c) = &self.cipher {
                    self.command(Command::Write(c.history(serial, count)));
                }
            }
            StreamEvent::Fault(reason) => {
                self.fault(reason);
                // Some faults are followed by a trusted replacement snapshot;
                // others clear the stream completely and need a fresh read.
                if self.stream.serial.is_none() {
                    self.request_initial_state(at);
                }
            }
            StreamEvent::Verified(serial) => {
                if let Some(candidate) = &mut self.completion
                    && candidate.serial == serial
                {
                    candidate.verified = true;
                }
                if let Some((id, expected)) = &self.verification
                    && *expected == serial
                {
                    if let Some(mut r) = self
                        .records
                        .iter()
                        .find(|r| value_str(r, "id") == id)
                        .cloned()
                    {
                        if let Some(q) = r["quality"].as_array_mut() {
                            q.retain(|v| v.as_str() != Some("unverified-final-state"));
                        }
                        self.save(r);
                    }
                    self.verification = None;
                }
            }
            StreamEvent::Turn(p, cube) => {
                if let Some(candidate) = self.completion
                    && let Packet::Turn {
                        serial,
                        mv,
                        clock,
                        recovered,
                    } = &p
                {
                    if candidate.continues_axis_motion(*serial, *mv, *clock, *recovered, at) {
                        self.completion = None;
                    } else {
                        // A distinct-axis or later turn belongs to the next scramble,
                        // even when the user starts it before the quiet period.
                        self.finish_completion();
                    }
                }
                let before = self.state;
                self.state = Some(cube);
                if let Packet::Turn {
                    serial,
                    mv,
                    clock,
                    recovered,
                } = p
                {
                    self.animation_moves.push((mv, clock, recovered));
                    if let Some(a) = &mut self.active {
                        if let Some((basis, pose)) = self.view_frame {
                            if a.rotations.is_empty() || a.gyro_recorder.has_live_frame() {
                                a.gyro_recorder
                                    .set_view_frame(basis, pose, a.started.is_some());
                            }
                        }
                        let first = *a.started.get_or_insert(at);
                        if a.moves.is_empty() {
                            a.clock = clock;
                        } else if a.clock.is_none() && !recovered {
                            a.clock = crate::timing::estimated_origin(&a.moves, clock);
                        }
                        let local = at.saturating_duration_since(first).as_secs_f64() * 1000.;
                        let mut t = clock.zip(a.clock).map(|(c, s)| c - s).unwrap_or_else(|| {
                            if recovered && clock.is_none() {
                                crate::timing::estimated_recovered_elapsed(&a.moves)
                            } else {
                                local
                            }
                        });
                        let previous_clock = a
                            .moves
                            .iter()
                            .rev()
                            .find_map(|m| m["raw"]["cubeTimestamp"].as_f64());
                        let clock_reset = t < 0.
                            || clock
                                .zip(previous_clock)
                                .is_some_and(|(current, previous)| current < previous);
                        if clock_reset {
                            if !a.quality.iter().any(|q| q == "clock-reset") {
                                a.quality.push("clock-reset".into());
                            }
                            t = local;
                        } else if clock.is_some() && a.clock.is_some() && !recovered {
                            a.live_clock_offset = a.live_clock_offset.max(t - local);
                        }
                        if recovered && !a.quality.iter().any(|q| q == "estimated-timing") {
                            a.quality.push("estimated-timing".into());
                        }
                        if !recovered && !a.quality.iter().any(|q| q == "clock-reset") {
                            crate::timing::settle_recovered(&mut a.moves, a.clock, clock);
                        }
                        t = t.max(a.moves.last().map(|m| number(m, "t")).unwrap_or(0.));
                        a.moves.push(json!({"move":mv.to_string(),"t":t,"recovered":recovered,"raw":{"serial":serial,"cubeTimestamp":clock,"localTimestamp":local,"receivedAt":local}}));
                        let done = cube.goal(&a.target.stage, a.cross.as_bytes()[0]);
                        if done {
                            let draft = a.record("interrupted");
                            self.completion = Some(CompletionCandidate {
                                serial,
                                mv,
                                clock,
                                at,
                                recovered,
                                verified: false,
                            });
                            self.persist("draft", &draft);
                            self.read_state();
                        } else if self.last_draft.elapsed() > Duration::from_millis(200) {
                            let r = a.record("interrupted");
                            self.persist("draft", &r);
                            self.last_draft = Instant::now();
                        }
                    } else {
                        if self.target.is_some() || self.scramble_loading {
                            self.scramble
                                .observe(before, cube, mv, clock, recovered, at);
                        }
                        if let Some(target) = &self.target {
                            if let Some(before) = before
                                && crate::cube::scramble_progress(before, &target.moves).matched
                            {
                                self.guidance_checkpoint = Some(before);
                            }
                            let plan = crate::scramble::correction(
                                before,
                                cube,
                                mv,
                                &target.moves,
                                &self.recovery,
                            );
                            self.clear_guidance();
                            if let Some(plan) = plan {
                                self.recovery = plan;
                            }
                        }
                        if self.target.as_ref().is_some_and(|t| t.cube != cube) {
                            self.cue_ready = false;
                            self.countdown = None;
                        }
                        self.maybe_arm();
                        self.refresh_guidance();
                    }
                }
            }
        }
    }
    /// The candidate's device time is frozen while waiting for a possible
    /// continuation of the same physical half turn or slice-axis motion.
    pub(crate) fn settle_completion(&mut self, at: Instant) {
        if self.completion.is_some_and(|candidate| {
            at.saturating_duration_since(candidate.at) >= COMPLETION_SETTLE
        }) {
            self.finish_completion();
        }
    }
    fn finish_completion(&mut self) {
        let Some(candidate) = self.completion.take() else {
            return;
        };
        let Some(active) = self.active.as_ref() else {
            return;
        };
        if !self
            .state
            .is_some_and(|cube| cube.goal(&active.target.stage, active.cross.as_bytes()[0]))
        {
            return;
        }
        let mut record = active.record("complete");
        if !candidate.verified {
            record["quality"]
                .as_array_mut()
                .unwrap()
                .push(json!("unverified-final-state"));
        }
        let id = value_str(&record, "id").to_owned();
        let initial = record["initialState"].clone();
        let cross = record["analysis"]["cross"]
            .as_str()
            .unwrap_or(&active.cross)
            .to_owned();
        let is_cross = active.target.stage == "Cross" || active.target.stage == "solve";
        let completed_target = active.target.clone();
        self.active = None;
        self.target = None;
        self.latest = Some(record.clone());
        self.verification = (!candidate.verified).then(|| (id.clone(), candidate.serial));
        self.persist("draft", &Value::Null);
        self.save(record);
        if !candidate.verified {
            self.read_state();
        }
        if is_cross {
            self.request(
                "cross",
                "POST",
                Some(json!({"state":initial,"cross":cross})),
                &format!("cross:{id}:{cross}"),
            );
        }
        if completed_target.case.is_none() {
            self.new_scramble();
        } else {
            self.target = Some(completed_target);
            self.guidance_checkpoint = None;
            self.scramble = Default::default();
            self.clear_guidance();
            self.refresh_guidance();
        }
    }
    fn maybe_arm(&mut self) {
        if self.active.is_some() {
            return;
        }
        if let (Some(cube), Some(target)) = (self.state, self.target.clone())
            && cube == target.cube
        {
            self.guidance_checkpoint = Some(cube);
            self.scramble.freeze();
            self.clear_guidance();
            if target.case.is_some() {
                self.cue_ready = true;
            } else {
                let mut a = Active::new(cube, target, &self.settings);
                a.scramble_moves = self.scramble.moves.clone();
                a.scramble_quality = self.scramble.quality.clone();
                self.active = Some(a);
            }
        }
    }
    pub fn start_cue(&mut self) {
        if self.cue_ready && self.countdown.is_none() {
            self.countdown = Some(Instant::now());
        }
    }
    pub fn new_scramble(&mut self) {
        if self.active.is_some() {
            return;
        }
        self.target_generation += 1;
        self.target = None;
        self.guidance_checkpoint = None;
        self.scramble = Default::default();
        self.scramble_loading = true;
        self.clear_guidance();
        self.request(
            "scramble",
            "GET",
            None,
            &format!("scramble:{}", self.target_generation),
        );
    }
    pub fn prepare_case(&mut self, id: &str, variant: usize) {
        self.interrupt("user-stopped");
        self.scramble = Default::default();
        let Some(case) = self.catalog.cases.iter().find(|c| c.id == id) else {
            return;
        };
        let Some(v) = case.variants.get(variant) else {
            return;
        };
        let Some(p) = v.positions.get(&self.settings.cross) else {
            return;
        };
        self.target = Some(target(
            p,
            case.stage.clone(),
            Some(case.id.clone()),
            Some(v.id.clone()),
        ));
        self.maybe_arm();
        self.refresh_guidance();
    }
    pub fn arm_current(&mut self) {
        if let Some(cube) = self.state {
            if cube == Cube::default() {
                self.message = "Scramble the physical cube first".into();
                return;
            }
            self.interrupt("user-stopped");
            let target = Target {
                cube,
                moves: vec![],
                stage: "solve".into(),
                case: None,
                algorithm: None,
            };
            self.active = Some(Active::new(cube, target, &self.settings));
            self.latest = None;
        }
    }
    pub fn interrupt(&mut self, reason: &str) {
        self.completion = None;
        self.guidance_checkpoint = None;
        self.target_generation += 1;
        self.scramble_loading = false;
        self.clear_guidance();
        self.countdown = None;
        self.cue_ready = false;
        if let Some(mut a) = self.active.take()
            && !a.moves.is_empty()
        {
            a.quality.push(reason.into());
            let r = a.record("interrupted");
            self.latest = Some(r.clone());
            self.save(r);
        }
        self.target = None;
        self.persist("draft", &Value::Null);
    }
    fn fault(&mut self, reason: &str) {
        // Turn/state recovery is independent of the gyro reference. Clearing
        // orientation here silently recalibrates the user's current hold as
        // home on the next gyro packet, making repeated recovery look like drift.
        // Transport disconnect and adapter-off paths clear it explicitly.
        self.scramble = Default::default();
        self.interrupt(reason);
        if let Some((id, _)) = self.verification.take()
            && let Some(mut r) = self
                .records
                .iter()
                .find(|r| value_str(r, "id") == id)
                .cloned()
        {
            r["status"] = json!("interrupted");
            r["penalty"] = json!("DNF");
            if let Some(q) = r["quality"].as_array_mut() {
                q.push(json!(reason));
            }
            self.save(r);
        }
        self.state = None;
        self.message = format!(
            "Tracking interrupted: {}. Read the cube state before another attempt.",
            reason.replace('-', " ")
        );
    }
    pub fn display_snapshot(&self) -> (f64, Value) {
        let now = Instant::now();
        (self.elapsed_at(now), self.live_analysis_at(now))
    }
    pub fn elapsed(&self) -> f64 {
        self.elapsed_at(Instant::now())
    }
    fn elapsed_at(&self, now: Instant) -> f64 {
        if let Some(a) = &self.active
            && a.started.is_some()
        {
            if self.completion.is_some() {
                return a.moves.last().map(|event| number(event, "t")).unwrap_or(0.);
            }
            a.started
                .map(|s| {
                    now.saturating_duration_since(s).as_secs_f64() * 1000. + a.live_clock_offset
                })
                .unwrap_or(0.)
                .max(a.moves.last().map(|v| number(v, "t")).unwrap_or(0.))
        } else {
            self.latest
                .as_ref()
                .map(|r| number(r, "duration"))
                .unwrap_or(0.)
        }
    }
    pub fn live_analysis(&self) -> Value {
        self.live_analysis_at(Instant::now())
    }
    fn live_analysis_at(&self, now: Instant) -> Value {
        if let Some(a) = &self.active
            && a.started.is_some()
        {
            let mut value = a.record("interrupted")["analysis"].clone();
            let duration = self.elapsed_at(now);
            let timing_reliable = !a.quality.iter().any(|q| {
                matches!(
                    q.as_str(),
                    "missing-moves" | "state-mismatch" | "clock-reset"
                )
            });
            value["duration"] = json!(duration);
            // Interior recovery cannot change the live clock's original
            // anchor. Only an estimated start makes the total rate estimated.
            let estimated_start = a
                .moves
                .first()
                .is_some_and(|event| event["recovered"] == true)
                || a.clock.is_none();
            value["estimated"] = json!(estimated_start);
            value["tps"] = json!(if timing_reliable && duration > 0. {
                Some(crate::recording::count_moves(&a.moves) as f64 * 1000. / duration)
            } else {
                None
            });
            value["pauseFraction"] = json!(if duration > 0. {
                number(&value, "pauseTime") / duration
            } else {
                0.
            });
            if let Some(splits) = value["splits"].as_array_mut() {
                let record = a.record("interrupted");
                for split in splits.iter_mut() {
                    split["estimated"] = json!(crate::recording::split_timing_estimated(
                        &record,
                        value_str(split, "stage"),
                    ));
                }
                if let Some(current) = splits.iter_mut().find(|split| split["end"].is_null()) {
                    let time = (duration - number(current, "start")).max(0.);
                    current["time"] = json!(time);
                    current["tps"] = json!(if timing_reliable && time > 0. {
                        Some(number(current, "moves") * 1000. / time)
                    } else {
                        None
                    });
                    current["live"] = json!(true);
                    current["estimated"] = json!(estimated_start || current["estimated"] == true);
                }
                if !timing_reliable {
                    for split in splits {
                        split["tps"] = Value::Null;
                    }
                }
            }
            value
        } else {
            self.latest
                .as_ref()
                .map(|r| r["analysis"].clone())
                .unwrap_or(Value::Null)
        }
    }
    pub fn manual(&mut self, seconds: f64) {
        if !(0. < seconds && seconds <= 86400.) {
            return;
        }
        let r = json!({"id":uuid::Uuid::new_v4().to_string(),"createdAt":chrono::Utc::now().to_rfc3339(),"kind":"solve","source":"manual","status":"complete","penalty":"","duration":seconds*1000.,"initialState":Cube::default().to_string(),"cross":self.settings.cross,"session":self.settings.session,"scramble":self.target.as_ref().map(|t|t.moves.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")),"moves":[],"quality":[],"notes":""});
        self.latest = Some(r.clone());
        self.save(r);
        self.new_scramble();
    }
    pub fn save_settings(&mut self) {
        if self.active.is_some() {
            self.message = "Finish the attempt before changing settings".into();
            return;
        }
        self.target = None;
        self.scramble_loading = false;
        self.target_generation += 1;
        self.clear_guidance();
        self.cue_ready = false;
        self.countdown = None;
        self.persist("settings", &serde_json::to_value(&self.settings).unwrap());
        self.sync();
    }
    pub fn set_theme(&mut self, theme: &str) {
        self.settings.theme = theme.into();
        self.persist("settings", &serde_json::to_value(&self.settings).unwrap());
    }
    pub fn set_session(&mut self, session: &str) {
        if self.active.is_some() {
            return;
        }
        self.settings.session = session.into();
        self.persist("settings", &serde_json::to_value(&self.settings).unwrap());
    }
    pub fn update_preferences(&mut self) {
        self.persist_history();
        self.request(
            "preferences",
            "PUT",
            Some(self.preferences.clone()),
            "preferences",
        );
    }
    pub fn delete(&mut self, id: &str) {
        if self.settings.token.is_empty() {
            self.records.retain(|r| r["id"] != id);
            self.pending.retain(|r| r["id"] != id);
            if self.latest.as_ref().is_some_and(|r| r["id"] == id) {
                self.latest = None;
            }
            self.persist_history();
        } else {
            self.request(
                &format!("records/{id}"),
                "DELETE",
                Some(json!({})),
                &format!("delete:{id}"),
            );
        }
    }
    pub fn backup(&self) -> Value {
        json!({"schema":1,"records":self.records,"preferences":self.preferences,"exportedAt":chrono::Utc::now().to_rfc3339()})
    }
    pub fn import_backup(&mut self, value: Value) {
        match crate::import::records(&value) {
            Ok(records) => {
                let incoming = records
                    .into_iter()
                    .map(crate::recording::refresh_analysis)
                    .collect::<Vec<_>>();
                let ids = incoming
                    .iter()
                    .map(|r| value_str(r, "id").to_string())
                    .collect::<std::collections::HashSet<_>>();
                self.records.retain(|r| !ids.contains(value_str(r, "id")));
                self.pending.retain(|r| !ids.contains(value_str(r, "id")));
                self.records.extend(incoming.clone());
                self.pending.extend(incoming);
                self.records
                    .sort_by(|a, b| value_str(a, "createdAt").cmp(value_str(b, "createdAt")));
                self.persist_history();
                self.sync();
                self.message = "Backup imported and saved locally".into();
            }
            Err(e) => self.message = format!("Import failed: {e}"),
        }
    }
    pub fn shutdown(&mut self) {
        self.interrupt("app-closed");
        self.persist_history();
        self.command(Command::Disconnect);
    }
    fn api_event(&mut self, e: ApiEvent) {
        if let Some(revision) = e
            .tag
            .strip_prefix("recovery:")
            .and_then(|s| s.parse::<u64>().ok())
        {
            let Some((expected, state, target)) = self.recovery_request else {
                return;
            };
            if revision != expected
                || revision != self.recovery_revision
                || self.state != Some(state)
                || self.target.is_none()
                || self.guidance_checkpoint.unwrap_or_default() != target
                || self.active.is_some()
            {
                return;
            }
            self.recovery_request = None;
            self.recovery_busy = false;
            match e.result {
                Ok(value) => {
                    let parsed = value["moves"].as_array().map(|list| {
                        list.iter()
                            .map(|v| Move::parse(v.as_str().unwrap_or("")))
                            .collect::<anyhow::Result<Vec<_>>>()
                    });
                    if let Some(Ok(plan)) = parsed
                        && state.apply(&plan) == target
                    {
                        self.recovery = plan;
                        self.recovery_error = None;
                    } else {
                        self.recovery_error =
                            Some("Could not verify correction. Turn the cube to retry.".into());
                    }
                }
                Err(_) => {
                    self.recovery_error =
                        Some("Correction unavailable offline. Reconnect to retry.".into())
                }
            }
            return;
        }
        if let Some(generation) = e
            .tag
            .strip_prefix("scramble:")
            .and_then(|s| s.parse::<u64>().ok())
        {
            if generation != self.target_generation {
                return;
            }
            self.scramble_loading = false;
        }
        if e.tag == "sync" {
            self.syncing = false;
            match e.result {
                Ok(value) => {
                    for sent in e.sent {
                        self.pending.retain(|v| *v != sent);
                    }
                    let previous = self
                        .records
                        .iter()
                        .map(|r| (value_str(r, "id").to_owned(), r.clone()))
                        .collect::<std::collections::HashMap<_, _>>();
                    self.records = value["bootstrap"]["records"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    for record in &mut self.records {
                        if let Some(local) = previous.get(value_str(record, "id")) {
                            *record =
                                crate::recording::retain_replay_metadata(record.clone(), local);
                        }
                    }
                    crate::recording::refresh_records(&mut self.records);
                    for r in &self.pending {
                        self.records.retain(|v| v["id"] != r["id"]);
                        self.records.push(r.clone());
                    }
                    self.records
                        .sort_by(|a, b| value_str(a, "createdAt").cmp(value_str(b, "createdAt")));
                    self.preferences = value["bootstrap"]["preferences"].clone();
                    self.stats = value["stats"].as_array().cloned().unwrap_or_default();
                    if let Some(last) = &self.latest {
                        self.latest = self.records.iter().find(|r| r["id"] == last["id"]).cloned();
                    }
                    self.persist_history();
                    self.persist("analytics", &json!(self.stats));
                    self.sync_status = "Synced to milkyway".into();
                    if !self.pending.is_empty() {
                        self.sync();
                    }
                }
                Err(reason) => {
                    self.sync_status = if self.pending.is_empty() {
                        "Offline · cached history".into()
                    } else {
                        format!("Offline · {} saved locally", self.pending.len())
                    };
                    if reason.contains("key") || reason.contains("connection") {
                        self.message = reason;
                    }
                }
            }
            return;
        }
        match e.result {
            Err(reason) => self.message = reason,
            Ok(v) => {
                if e.tag.starts_with("scramble:") {
                    if self.active.is_none() {
                        if let Ok(moves) = sequence(value_str(&v, "scramble")) {
                            self.target = Some(Target {
                                cube: Cube::default().apply(&moves),
                                moves,
                                stage: "solve".into(),
                                case: None,
                                algorithm: None,
                            });
                            self.countdown = None;
                            self.cue_ready = false;
                            self.maybe_arm();
                            self.refresh_guidance();
                        }
                    }
                } else if let Some(request) = e.tag.strip_prefix("cross:") {
                    let (id, requested_cross) = request
                        .rsplit_once(':')
                        .map(|(id, face)| (id, Some(face)))
                        .unwrap_or((request, None));
                    if let Some(mut r) = self
                        .records
                        .iter()
                        .find(|r| value_str(r, "id") == id)
                        .cloned()
                    {
                        let effective = r["analysis"]["cross"]
                            .as_str()
                            .unwrap_or(value_str(&r, "cross"));
                        let requested = requested_cross.unwrap_or(value_str(&r, "cross"));
                        if requested != effective {
                            return;
                        }
                        let mut optimal = v;
                        optimal["cross"] = json!(requested);
                        r["crossOptimal"] = optimal;
                        self.save(r);
                    }
                } else if let Some(id) = e.tag.strip_prefix("delete:") {
                    self.records.retain(|r| value_str(r, "id") != id);
                    self.pending.retain(|r| value_str(r, "id") != id);
                    if self
                        .latest
                        .as_ref()
                        .is_some_and(|r| value_str(r, "id") == id)
                    {
                        self.latest = None;
                    }
                    self.persist_history();
                    self.sync();
                } else {
                    self.sync();
                }
            }
        }
    }
}

#[cfg(test)]
mod controller_tests {
    use super::*;
    #[test]
    fn active_record_keeps_the_live_starting_pose_and_frame() {
        let mut model = Model::new(eframe::egui::Context::default(), true);
        let initial = Cube::default();
        let target = Target {
            cube: initial,
            moves: vec![],
            stage: "solve".into(),
            case: None,
            algorithm: None,
        };
        model.active = Some(Active::new(initial, target, &model.settings));
        let s = std::f32::consts::FRAC_1_SQRT_2;
        model.capture_view_frame(Some(([0., 0., 0., 1.], [0., s, 0., s])));
        let active = model.active.as_ref().unwrap();
        let record = active.record("interrupted");
        assert_eq!(record["rotationFrame"], "live-v1");
        let pose = record["initialOrientation"].as_array().unwrap();
        for (v, expected) in pose.iter().zip([0., s, 0., s]) {
            assert!((v.as_f64().unwrap() - expected as f64).abs() < 1e-6);
        }
        assert_eq!(record["moves"], json!([]));
    }

    #[test]
    fn deletion_removes_only_the_selected_solve_and_pending_upload() {
        let mut model = Model::new(eframe::egui::Context::default(), true);
        let first = json!({"id":"delete-first","source":"manual","duration":12000.,"moves":[]});
        let second = json!({"id":"delete-second","source":"manual","duration":14000.,"moves":[]});
        model.records = vec![first.clone(), second.clone()];
        model.pending = model.records.clone();
        model.latest = Some(second.clone());
        let revision = model.revision;
        model.delete("delete-first");
        assert_eq!(model.records, vec![second.clone()]);
        assert_eq!(model.pending, vec![second.clone()]);
        assert_eq!(model.latest, Some(second));
        assert!(model.revision > revision);
        model.delete("delete-second");
        assert!(model.records.is_empty());
        assert!(model.pending.is_empty());
        assert!(model.latest.is_none());
    }

    #[test]
    fn server_delete_ack_clears_latest_but_failed_delete_preserves_history() {
        let mut model = Model::new(eframe::egui::Context::default(), true);
        let record = json!({"id":"delete-ack","source":"manual","duration":12000.,"moves":[]});
        model.records = vec![record.clone()];
        model.pending = vec![record.clone()];
        model.latest = Some(record.clone());
        model.api_event(ApiEvent {
            tag: "delete:delete-ack".into(),
            result: Err("offline".into()),
            sent: vec![],
        });
        assert_eq!(model.records, vec![record]);
        assert!(model.latest.is_some());
        model.api_event(ApiEvent {
            tag: "delete:delete-ack".into(),
            result: Ok(json!({})),
            sent: vec![],
        });
        assert!(model.records.is_empty());
        assert!(model.pending.is_empty());
        assert!(model.latest.is_none());
    }

    fn snapshot(cube: Cube, serial: u8) -> Vec<u8> {
        let (cp, co, ep, eo) = cube.cubies().unwrap();
        let mut bytes = vec![0u8; 20];
        bytes[0] = 0xed;
        bytes[1] = 14;
        bytes[2] = serial;
        let mut put = |offset: usize, width: usize, value: usize| {
            for i in 0..width {
                let bit = offset + i;
                if value & (1 << (width - 1 - i)) != 0 {
                    bytes[bit / 8] |= 1 << (7 - bit % 8);
                }
            }
        };
        for i in 0..7 {
            put(32 + i * 3, 3, cp[i]);
            put(53 + i * 2, 2, co[i]);
        }
        for i in 0..11 {
            put(69 + i * 4, 4, ep[i]);
            put(113 + i, 1, eo[i]);
        }
        bytes
    }
    fn emit(model: &mut Model, plain: Vec<u8>, at: Instant) {
        let bytes = model
            .cipher
            .as_ref()
            .unwrap()
            .transform(&plain, true)
            .unwrap();
        model.bluetooth_event(Event::Bytes(bytes, at));
    }
    fn ready() -> Model {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = mpsc::channel().0;
        m.powered = true;
        m.connected = true;
        m.cipher = Some(Cipher::new("12:34:56:78:9A:BC").unwrap());
        m.prepare_case("pll-t", 0);
        let cube = m.target.as_ref().unwrap().cube;
        emit(&mut m, snapshot(cube, 0), Instant::now());
        assert!(m.cue_ready);
        m.countdown = Some(Instant::now() - Duration::from_secs(4));
        m.tick();
        assert!(m.active.is_some());
        m
    }

    #[test]
    fn matching_the_scramble_freezes_its_speed_before_inspection() {
        let mut m = ready();
        m.active = None;
        m.countdown = None;
        m.cue_ready = false;
        let r = Move::parse("R").unwrap();
        let u = Move::parse("U2").unwrap();
        let first = Cube::default().turn(r);
        let goal = first.turn(u);
        let at = Instant::now() - Duration::from_secs(10);
        m.scramble
            .observe(Some(Cube::default()), first, r, Some(1000.), false, at);
        m.scramble.observe(
            Some(first),
            goal,
            u,
            Some(1400.),
            false,
            at + Duration::from_millis(400),
        );
        let t = m.target.as_mut().unwrap();
        t.cube = goal;
        t.case = None;
        t.stage = "solve".into();
        m.state = Some(goal);
        m.maybe_arm();
        assert!(m.active.is_some());
        assert_eq!(m.scramble.live_stats(), m.scramble.stats());
        assert_eq!(m.scramble.live_stats()["duration"], 400.);
    }
    #[test]
    fn initial_state_retries_are_bounded_and_success_cancels_them() {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = mpsc::channel().0;
        m.powered = true;
        m.connect("12:34:56:78:9A:BC".into());
        m.bluetooth_event(Event::Connected("GAN test".into()));
        let start = m.connection_started.unwrap();
        assert_eq!(m.state_requests, 1);
        m.poll_initial_state(start + Duration::from_millis(999));
        assert_eq!(m.state_requests, 1);
        m.poll_initial_state(start + Duration::from_secs(1));
        assert_eq!(m.state_requests, 2);
        m.poll_initial_state(start + Duration::from_secs(3));
        assert_eq!(m.state_requests, 3);
        emit(
            &mut m,
            snapshot(Cube::default(), 5),
            start + Duration::from_secs(4),
        );
        m.poll_initial_state(start + Duration::from_secs(7));
        assert!(m.connected);
        assert_eq!(m.state, Some(Cube::default()));
        assert!(m.connection_started.is_none());
    }

    #[test]
    fn failed_handshake_releases_connection_and_keeps_actionable_error() {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = mpsc::channel().0;
        m.powered = true;
        m.connect("12:34:56:78:9A:BC".into());
        m.bluetooth_event(Event::Connected("GAN test".into()));
        m.poll_initial_state(m.connection_started.unwrap() + Duration::from_secs(6));
        assert!(!m.connected && !m.connecting);
        m.bluetooth_event(Event::Disconnected("Disconnected".into()));
        assert!(m.message.contains("did not return a valid state"));
        m.connect("12:34:56:78:9A:BC".into());
        assert!(m.connecting);
    }

    #[test]
    fn transport_and_decode_errors_resync_without_waiting_for_a_new_serial() {
        let mut m = ready();
        let cube = m.state.unwrap();
        m.bluetooth_event(Event::Error("Bluetooth write failed".into()));
        assert!(m.state.is_none());
        assert!(m.stream.serial.is_none());
        assert_eq!(m.state_requests, 1);
        emit(&mut m, snapshot(cube, 0), Instant::now());
        assert_eq!(m.state, Some(cube));
        m.bluetooth_event(Event::Bytes(vec![0; 3], Instant::now()));
        let started = m.connection_started.unwrap();
        m.bluetooth_event(Event::Bytes(vec![0; 3], started + Duration::from_secs(1)));
        assert_eq!(m.connection_started, Some(started));
        emit(&mut m, snapshot(cube, 0), started + Duration::from_secs(2));
        assert_eq!(m.state, Some(cube));
        assert!(m.connection_started.is_none());
    }

    #[test]
    fn disconnected_transport_cannot_repopulate_cube_from_queued_packets() {
        let mut m = ready();
        m.disconnect();
        emit(&mut m, snapshot(Cube::default(), 1), Instant::now());
        assert!(m.state.is_none());
        assert!(!m.gyro_live);
        m.bluetooth_event(Event::Connected("stale connection".into()));
        assert!(!m.connected);
    }

    #[test]
    fn logical_tracking_fault_does_not_recalibrate_the_gyro() {
        let mut model = ready();
        let q = [0., 0.70710677, 0., 0.70710677];
        let at = Instant::now();
        model.orientation = Some(q);
        model.orientation_at = Some(at);
        model.gyro_live = true;
        model.fault("missing-moves");
        assert_eq!(model.orientation, Some(q));
        assert_eq!(model.orientation_at, Some(at));
        assert!(model.gyro_live);
        model.bluetooth_event(Event::Disconnected("Cube disconnected".into()));
        assert!(model.orientation.is_none());
        assert!(!model.gyro_live);
    }

    #[test]
    fn wrong_turn_guidance_returns_to_the_exact_commuting_partial_checkpoint() {
        let mut model = Model::new(eframe::egui::Context::default(), true);
        model.api_tx = mpsc::channel().0;
        let moves = sequence("R U2 D F").unwrap();
        model.target = Some(Target {
            cube: Cube::default().apply(&moves),
            moves: moves.clone(),
            stage: "solve".into(),
            case: None,
            algorithm: None,
        });
        model.state = Some(Cube::default());
        model.refresh_guidance();
        let at = Instant::now();
        for (index, name) in ["R", "D", "U"].into_iter().enumerate() {
            recorded_turn(
                &mut model,
                name,
                index as u8 + 1,
                Some(index as f64 * 200.),
                false,
                at + Duration::from_millis(index as u64 * 200),
            );
        }
        let checkpoint = model.state.unwrap();
        assert_eq!(model.guidance_checkpoint, Some(checkpoint));
        recorded_turn(
            &mut model,
            "B",
            4,
            Some(600.),
            false,
            at + Duration::from_millis(600),
        );
        assert_eq!(model.guidance_checkpoint, Some(checkpoint));
        assert_eq!(model.recovery, sequence("B'").unwrap());
        assert_eq!(model.state.unwrap().apply(&model.recovery), checkpoint);
        assert_eq!(model.target.as_ref().unwrap().moves, moves);
        recorded_turn(
            &mut model,
            "B'",
            5,
            Some(800.),
            false,
            at + Duration::from_millis(800),
        );
        assert!(model.recovery.is_empty());
        let progress = crate::cube::scramble_progress(model.state.unwrap(), &moves);
        assert_eq!(progress.done, [0, 2]);
        assert_eq!(progress.partial_moves, [1]);
        assert_eq!(progress.completed, 1);
    }

    #[test]
    fn server_repair_is_verified_against_checkpoint_not_scramble_destination() {
        let mut model = Model::new(eframe::egui::Context::default(), true);
        model.api_tx = mpsc::channel().0;
        let moves = sequence("R U F").unwrap();
        let checkpoint = Cube::default().turn(moves[0]);
        let wrong = checkpoint.turn(Move::parse("B").unwrap());
        model.target = Some(Target {
            cube: Cube::default().apply(&moves),
            moves,
            stage: "solve".into(),
            case: None,
            algorithm: None,
        });
        model.state = Some(wrong);
        model.guidance_checkpoint = Some(checkpoint);
        model.recovery_revision = 7;
        model.recovery_request = Some((7, wrong, checkpoint));
        model.api_event(ApiEvent {
            tag: "recovery:7".into(),
            result: Ok(json!({"moves":["B'"]})),
            sent: vec![],
        });
        assert_eq!(model.recovery, sequence("B'").unwrap());
        assert_eq!(wrong.apply(&model.recovery), checkpoint);
    }
    fn turn(model: &mut Model, mv: Move, serial: u8, at: Instant) {
        let mut b = vec![0u8; 20];
        b[0] = 1;
        b[1] = 7;
        b[2..6].copy_from_slice(&(5000u32 + serial as u32 * 100).to_le_bytes());
        b[6] = serial;
        b[8] = match mv.face {
            b'U' => 2,
            b'R' => 32,
            b'F' => 8,
            b'D' => 1,
            b'L' => 16,
            _ => 4,
        } | if mv.turns < 0 { 64 } else { 0 };
        emit(model, b, at);
    }
    #[test]
    fn encrypted_drill_records_cue_execution_and_final_verification() {
        let mut m = ready();
        let start = m.active.as_ref().unwrap().ready + Duration::from_millis(600);
        let solution = m
            .catalog
            .cases
            .iter()
            .find(|c| c.id == "pll-t")
            .unwrap()
            .variants[0]
            .positions["U"]
            .solution
            .clone();
        let moves = solution
            .iter()
            .flat_map(|s| {
                let mv = Move::parse(s).unwrap();
                if mv.turns == 2 {
                    vec![Move { turns: 1, ..mv }; 2]
                } else {
                    vec![mv]
                }
            })
            .collect::<Vec<_>>();
        for (i, mv) in moves.iter().enumerate() {
            turn(
                &mut m,
                *mv,
                (i + 1) as u8,
                start + Duration::from_millis(i as u64 * 100),
            );
        }
        m.settle_completion(
            start + Duration::from_millis((moves.len() - 1) as u64 * 100) + COMPLETION_SETTLE,
        );
        assert!(m.active.is_none());
        let r = m.latest.as_ref().unwrap();
        assert_eq!(r["recognitionSource"], "cue");
        assert_eq!(number(r, "recognitionMs"), 600.);
        assert_eq!(number(r, "duration"), (moves.len() - 1) as f64 * 100.);
        assert!(
            r["quality"]
                .as_array()
                .unwrap()
                .contains(&json!("unverified-final-state"))
        );
        emit(
            &mut m,
            snapshot(Cube::default(), moves.len() as u8),
            Instant::now(),
        );
        assert!(
            m.latest.as_ref().unwrap()["quality"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(m.pending.len(), 1);
    }
    #[test]
    fn bluetooth_off_interrupts_active_attempt_and_never_looks_like_no_devices() {
        let mut m = ready();
        turn(&mut m, Move::parse("R").unwrap(), 1, Instant::now());
        m.bluetooth_event(Event::Adapter {
            powered: false,
            reason: "Bluetooth is off".into(),
        });
        assert!(!m.connected && !m.powered);
        assert!(m.active.is_none());
        let r = m.latest.as_ref().unwrap();
        assert_eq!(r["status"], "interrupted");
        assert_eq!(r["penalty"], "DNF");
        assert!(
            r["quality"]
                .as_array()
                .unwrap()
                .contains(&json!("bluetooth-off"))
        );
        m.scan();
        assert_eq!(m.message, "Bluetooth is off");
        assert!(!m.scanning);
    }
    #[test]
    fn next_scramble_retains_result_and_discards_stale_responses() {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = mpsc::channel().0;
        m.latest = Some(json!({"id":"last", "duration":13320., "analysis":{"tps":3.9}}));
        m.new_scramble();
        let stale = m.target_generation;
        m.new_scramble();
        m.api_event(ApiEvent {
            tag: format!("scramble:{stale}"),
            result: Ok(json!({"scramble":"F"})),
            sent: vec![],
        });
        assert!(m.target.is_none());
        m.api_event(ApiEvent {
            tag: format!("scramble:{}", m.target_generation),
            result: Ok(json!({"scramble":"R U"})),
            sent: vec![],
        });
        assert_eq!(m.latest.as_ref().unwrap()["id"], "last");
        m.state = Some(m.target.as_ref().unwrap().cube);
        m.maybe_arm();
        assert!(m.active.as_ref().unwrap().started.is_none());
        assert_eq!(m.elapsed(), 13320.);
        assert_eq!(m.live_analysis()["tps"], 3.9);
    }
    #[test]
    fn live_tps_and_split_time_use_the_same_running_clock() {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = mpsc::channel().0;
        let setup = sequence("F R U").unwrap();
        let initial = Cube::default().apply(&setup);
        let target = Target {
            cube: initial,
            moves: setup,
            stage: "solve".into(),
            case: None,
            algorithm: None,
        };
        let mut active = Active::new(initial, target, &m.settings);
        let start = Instant::now();
        active.started = Some(start);
        active.moves = vec![json!({"move":"U'","t":0.}), json!({"move":"R'","t":1000.})];
        m.active = Some(active);
        let running = m.live_analysis_at(start + Duration::from_secs(3));
        assert_eq!(running["duration"], 3000.);
        assert!((number(&running, "tps") - 2. / 3.).abs() < 1e-9);
        assert_eq!(running["moves"], 2);
        let current = running["splits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["live"] == true)
            .unwrap();
        assert_eq!(number(current, "time"), 3000. - number(current, "start"));
        assert_eq!(
            number(current, "tps"),
            number(current, "moves") * 1000. / number(current, "time")
        );
        assert_eq!(
            m.active.as_ref().unwrap().record("complete")["duration"],
            1000.
        );
        assert_eq!(m.active.as_ref().unwrap().moves.last().unwrap()["t"], 1000.);
    }
    #[test]
    fn finished_metrics_freeze_and_keep_penalties_out_of_execution_tps() {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.latest = Some(json!({
            "id":"finished", "source":"bluetooth", "status":"complete", "penalty":"+2",
            "duration":12500., "quality":["unverified-final-state"],
            "analysis":{"duration":12500.,"tps":4.,"moves":50}
        }));
        let future = Instant::now() + Duration::from_secs(60);
        let value = m.live_analysis_at(future);
        assert_eq!(value["duration"], 12500.);
        assert_eq!(value["tps"], 4.);
        assert_eq!(value["moves"], 50);
        assert_eq!(m.elapsed_at(future), 12500.);
        assert_eq!(io::adjusted(m.latest.as_ref().unwrap()), 14500.);
        m.latest = Some(json!({"id":"manual","source":"manual","duration":20000.}));
        assert!(m.live_analysis_at(future).is_null());
    }
    #[test]
    fn reduced_bluetooth_latency_does_not_freeze_the_live_timer() {
        let mut m = ready();
        let start = Instant::now();
        let r = Move::parse("R").unwrap();
        let u = Move::parse("U").unwrap();
        let first = m.state.unwrap().turn(r);
        m.stream_event(
            StreamEvent::Turn(
                Packet::Turn {
                    serial: 1,
                    mv: r,
                    clock: Some(1000.),
                    recovered: false,
                },
                first,
            ),
            start,
        );
        m.stream_event(
            StreamEvent::Turn(
                Packet::Turn {
                    serial: 2,
                    mv: u,
                    clock: Some(3000.),
                    recovered: false,
                },
                first.turn(u),
            ),
            start + Duration::from_secs(1),
        );
        assert_eq!(m.elapsed_at(start + Duration::from_millis(1500)), 2500.);
        assert_eq!(
            m.live_analysis_at(start + Duration::from_millis(1500))["duration"],
            2500.
        );
        assert_eq!(
            m.active.as_ref().unwrap().record("complete")["duration"],
            2000.
        );
    }
    #[test]
    fn backwards_device_clock_above_start_is_flagged_without_losing_turns() {
        let mut m = ready();
        let start = Instant::now();
        let mut state = m.state.unwrap();
        for (i, (name, clock, local)) in [("R", 1000., 0), ("U", 2000., 1000), ("R'", 1500., 1100)]
            .into_iter()
            .enumerate()
        {
            let mv = Move::parse(name).unwrap();
            state = state.turn(mv);
            m.stream_event(
                StreamEvent::Turn(
                    Packet::Turn {
                        serial: (i + 1) as u8,
                        mv,
                        clock: Some(clock),
                        recovered: false,
                    },
                    state,
                ),
                start + Duration::from_millis(local),
            );
        }
        let active = m.active.as_ref().unwrap();
        assert_eq!(active.moves.len(), 3);
        assert!(active.quality.iter().any(|q| q == "clock-reset"));
        assert_eq!(active.moves[2]["raw"]["cubeTimestamp"], 1500.);
        assert_eq!(active.moves[2]["t"], 1100.);
        let running = m.live_analysis_at(start + Duration::from_millis(1500));
        assert!(running["tps"].is_null());
        assert_eq!(running["moves"], 3);
        assert!(
            running["splits"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["tps"].is_null())
        );
    }
    #[test]
    fn cancelling_pending_scramble_clears_loading_and_ignores_late_response() {
        for calibrate in [false, true] {
            let mut m = Model::new(eframe::egui::Context::default(), true);
            m.api_tx = mpsc::channel().0;
            m.new_scramble();
            let old_generation = m.target_generation;
            assert!(m.scramble_loading);
            if calibrate {
                m.calibrate();
            } else {
                m.save_settings();
            }
            assert!(!m.scramble_loading);
            m.api_event(ApiEvent {
                tag: format!("scramble:{old_generation}"),
                result: Ok(json!({"scramble":"R U"})),
                sent: vec![],
            });
            assert!(m.target.is_none());
            assert!(!m.scramble_loading);
        }
    }
    #[test]
    fn completed_solve_loads_next_target_without_losing_late_verification() {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = mpsc::channel().0;
        m.powered = true;
        m.connected = true;
        m.cipher = Some(Cipher::new("12:34:56:78:9A:BC").unwrap());
        let moves = sequence("R U").unwrap();
        let cube = Cube::default().apply(&moves);
        m.target = Some(Target {
            cube,
            moves,
            stage: "solve".into(),
            case: None,
            algorithm: None,
        });
        emit(&mut m, snapshot(cube, 0), Instant::now());
        let start = Instant::now();
        turn(&mut m, Move::parse("U'").unwrap(), 1, start);
        turn(
            &mut m,
            Move::parse("R'").unwrap(),
            2,
            start + Duration::from_millis(100),
        );
        m.settle_completion(start + Duration::from_millis(100) + COMPLETION_SETTLE);
        assert!(m.scramble_loading);
        let id = value_str(m.latest.as_ref().unwrap(), "id").to_owned();
        m.api_event(ApiEvent {
            tag: format!("scramble:{}", m.target_generation),
            result: Ok(json!({"scramble":"F2 D"})),
            sent: vec![],
        });
        turn(
            &mut m,
            Move::parse("F").unwrap(),
            3,
            start + Duration::from_millis(300),
        );
        emit(
            &mut m,
            snapshot(Cube::default(), 2),
            start + Duration::from_millis(350),
        );
        let saved = m.records.iter().find(|r| value_str(r, "id") == id).unwrap();
        assert!(saved["quality"].as_array().unwrap().is_empty());
        assert_eq!(value_str(m.latest.as_ref().unwrap(), "id"), id);
        assert_eq!(
            m.state,
            Some(Cube::default().turn(Move::parse("F").unwrap()))
        );
        assert_eq!(m.target.as_ref().unwrap().moves, sequence("F2 D").unwrap());
        assert_eq!(m.elapsed(), 100.);
    }
    fn completion_model(setup: &str) -> Model {
        let mut model = Model::new(eframe::egui::Context::default(), true);
        model.api_tx = mpsc::channel().0;
        model.powered = true;
        model.connected = true;
        model.state = Some(Cube::default().apply(&sequence(setup).unwrap()));
        model.arm_current();
        assert!(model.active.is_some());
        model
    }
    fn recorded_turn(
        model: &mut Model,
        name: &str,
        serial: u8,
        clock: Option<f64>,
        recovered: bool,
        at: Instant,
    ) {
        let mv = Move::parse(name).unwrap();
        let cube = model.state.unwrap().turn(mv);
        model.stream_event(
            StreamEvent::Turn(
                Packet::Turn {
                    serial,
                    mv,
                    clock,
                    recovered,
                },
                cube,
            ),
            at,
        );
    }
    #[test]
    fn recovered_interior_latency_does_not_replace_the_real_finish_time() {
        let mut model = completion_model("B' F' U' R'");
        let at = Instant::now();
        recorded_turn(&mut model, "R", 1, Some(1000.), false, at);
        recorded_turn(
            &mut model,
            "U",
            2,
            Some(1200.),
            false,
            at + Duration::from_millis(200),
        );
        recorded_turn(
            &mut model,
            "F",
            3,
            None,
            true,
            at + Duration::from_millis(550),
        );
        recorded_turn(
            &mut model,
            "B",
            4,
            Some(1400.),
            false,
            at + Duration::from_millis(560),
        );
        model.stream_event(StreamEvent::Verified(4), at + Duration::from_millis(570));
        model.settle_completion(at + Duration::from_millis(690));
        let record = model.latest.as_ref().unwrap();
        assert_eq!(record["duration"], 400.);
        assert_eq!(record["moves"][2]["t"], 300.);
        assert_eq!(record["moves"][2]["raw"]["receivedAt"], 550.);
        assert!(record["moves"][2]["raw"]["cubeTimestamp"].is_null());
        assert_eq!(record["moves"][3]["t"], 400.);
        assert_eq!(record["quality"], json!(["estimated-timing"]));
        assert!(
            !crate::recording::execution_metrics(record)
                .unwrap()
                .estimated
        );
    }
    #[test]
    fn recovered_finish_excludes_history_delivery_and_verification_delay() {
        for delay in [100, 1000] {
            let mut model = completion_model("B' F' U' R'");
            let at = Instant::now();
            recorded_turn(&mut model, "R", 1, Some(1000.), false, at);
            recorded_turn(
                &mut model,
                "U",
                2,
                Some(1200.),
                false,
                at + Duration::from_millis(200),
            );
            let arrival = at + Duration::from_millis(200 + delay);
            recorded_turn(&mut model, "F", 3, None, true, arrival);
            recorded_turn(&mut model, "B", 4, None, true, arrival);
            model.settle_completion(arrival + COMPLETION_SETTLE);
            assert_eq!(model.latest.as_ref().unwrap()["duration"], 300.);
            // State verification can arrive much later without changing the
            // historical turn times or declaring their estimates precise.
            model.stream_event(StreamEvent::Verified(4), arrival + Duration::from_secs(3));
            let record = model.latest.as_ref().unwrap();
            assert_eq!(record["duration"], 300.);
            assert_eq!(record["moves"][2]["t"], 250.);
            assert_eq!(record["moves"][3]["t"], 300.);
            assert_eq!(
                record["moves"][3]["raw"]["receivedAt"],
                (200 + delay) as f64
            );
            assert!(record["moves"][3]["raw"]["cubeTimestamp"].is_null());
            assert_eq!(record["quality"], json!(["estimated-timing"]));
            assert!(
                crate::recording::execution_metrics(record)
                    .unwrap()
                    .estimated
            );
        }
    }
    #[test]
    fn recovered_start_stays_estimated_after_live_device_clocks_resume() {
        for delay in [100, 1000] {
            let mut model = completion_model("B' F' U' R'");
            let at = Instant::now();
            let history_arrival = at + Duration::from_millis(delay);
            recorded_turn(&mut model, "R", 1, None, true, history_arrival);
            recorded_turn(&mut model, "U", 2, None, true, history_arrival);
            // Buffered live turns retain their original receive timestamps,
            // which may precede the response that recovered leading turns.
            recorded_turn(
                &mut model,
                "F",
                3,
                Some(1100.),
                false,
                at + Duration::from_millis(100),
            );
            recorded_turn(
                &mut model,
                "B",
                4,
                Some(1300.),
                false,
                at + Duration::from_millis(300),
            );
            model.stream_event(StreamEvent::Verified(4), history_arrival);
            model.settle_completion(history_arrival + Duration::from_secs(1));
            let record = model.latest.as_ref().unwrap();
            assert_eq!(record["duration"], 300.);
            assert_eq!(record["moves"][0]["t"], 0.);
            assert_eq!(record["moves"][1]["t"], 50.);
            assert_eq!(record["moves"][2]["t"], 100.);
            assert!(record["moves"][0]["raw"]["cubeTimestamp"].is_null());
            assert_eq!(record["quality"], json!(["estimated-timing"]));
            assert!(
                crate::recording::execution_metrics(record)
                    .unwrap()
                    .estimated
            );
        }
    }
    #[test]
    fn genuine_finish_uses_device_duration_despite_delivery_bursts_and_clock_rate_difference() {
        for delay in [100, 1000] {
            let mut model = completion_model("U' R'");
            let at = Instant::now();
            recorded_turn(&mut model, "R", 1, Some(1000.), false, at);
            let arrival = at + Duration::from_millis(500 + delay);
            recorded_turn(&mut model, "U", 2, Some(1400.), false, arrival);
            model.stream_event(StreamEvent::Verified(2), arrival);
            model.settle_completion(arrival + COMPLETION_SETTLE);
            let record = model.latest.as_ref().unwrap();
            assert_eq!(record["duration"], 400.);
            assert_eq!(
                record["moves"][1]["raw"]["receivedAt"],
                (500 + delay) as f64
            );
            assert!(
                !crate::recording::execution_metrics(record)
                    .unwrap()
                    .estimated
            );
        }
    }
    #[test]
    fn slice_partner_cancels_transient_completion_and_keeps_the_same_attempt() {
        let mut model = completion_model("R'");
        let at = Instant::now();
        let id = model.active.as_ref().unwrap().id.clone();
        recorded_turn(&mut model, "R", 1, Some(1000.), false, at);
        assert!(model.completion.is_some());
        assert_eq!(model.elapsed_at(at + Duration::from_millis(100)), 0.);
        assert!(model.records.is_empty());
        // Even a matching snapshot cannot end the partner window early.
        model.stream_event(StreamEvent::Verified(1), at + Duration::from_millis(10));
        assert!(model.active.is_some() && model.records.is_empty());
        recorded_turn(
            &mut model,
            "L'",
            2,
            Some(1020.),
            false,
            at + Duration::from_millis(20),
        );
        assert!(model.completion.is_none());
        assert_eq!(model.active.as_ref().unwrap().id, id);
        assert_eq!(model.active.as_ref().unwrap().moves.len(), 2);
        assert_eq!(
            model.state,
            Some(Cube::default().turn(Move::parse("L'").unwrap()))
        );
        model.settle_completion(at + Duration::from_millis(300));
        assert!(model.records.is_empty());
        recorded_turn(
            &mut model,
            "L",
            3,
            Some(1400.),
            false,
            at + Duration::from_millis(400),
        );
        assert_eq!(
            model.live_analysis_at(at + Duration::from_millis(500))["duration"],
            400.
        );
        model.settle_completion(at + Duration::from_millis(519));
        assert!(model.active.is_some());
        model.settle_completion(at + Duration::from_millis(520));
        assert!(model.active.is_none());
        let record = model.latest.as_ref().unwrap();
        assert_eq!(record["id"], id);
        assert_eq!(record["moves"].as_array().unwrap().len(), 3);
        assert_eq!(record["duration"], 400.);
        assert_eq!(record["quality"], json!(["unverified-final-state"]));
        model.stream_event(StreamEvent::Verified(3), at + Duration::from_millis(530));
        assert!(
            model.latest.as_ref().unwrap()["quality"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(model.records.len(), 1);
    }
    #[test]
    fn settled_goal_keeps_device_time_and_early_verification_without_extra_latency() {
        let mut model = completion_model("R U");
        let at = Instant::now();
        recorded_turn(&mut model, "U'", 1, Some(2000.), false, at);
        recorded_turn(
            &mut model,
            "R'",
            2,
            Some(2100.),
            false,
            at + Duration::from_millis(60),
        );
        assert_eq!(model.elapsed_at(at + Duration::from_millis(150)), 100.);
        model.stream_event(StreamEvent::Verified(2), at + Duration::from_millis(80));
        model.settle_completion(at + Duration::from_millis(179));
        assert!(model.latest.is_none());
        model.settle_completion(at + Duration::from_millis(180));
        let record = model.latest.as_ref().unwrap();
        assert_eq!(record["duration"], 100.);
        assert_eq!(record["analysis"]["tps"], 20.);
        assert!(record["quality"].as_array().unwrap().is_empty());
        assert!(model.verification.is_none());
    }
    #[test]
    fn same_face_first_slice_half_turn_burst_does_not_finish_at_its_first_quarter() {
        let mut model = completion_model("R'");
        let at = Instant::now();
        for (i, name) in ["R", "R", "L'", "L'"].into_iter().enumerate() {
            recorded_turn(
                &mut model,
                name,
                (i + 1) as u8,
                Some(1000. + i as f64 * 20.),
                false,
                at + Duration::from_millis(i as u64 * 20),
            );
        }
        model.settle_completion(at + Duration::from_millis(300));
        assert!(model.active.is_some() && model.records.is_empty());
        assert_eq!(model.active.as_ref().unwrap().moves.len(), 4);
        assert_eq!(
            model.state,
            Some(Cube::default().apply(&sequence("R L2").unwrap()))
        );
        for (i, name) in ["L", "L", "R'"].into_iter().enumerate() {
            recorded_turn(
                &mut model,
                name,
                (i + 5) as u8,
                Some(1400. + i as f64 * 100.),
                false,
                at + Duration::from_millis(400 + i as u64 * 100),
            );
        }
        assert_eq!(model.elapsed_at(at + Duration::from_millis(700)), 600.);
        model.settle_completion(at + Duration::from_millis(720));
        let record = model.latest.as_ref().unwrap();
        assert_eq!(record["duration"], 600.);
        assert_eq!(record["moves"].as_array().unwrap().len(), 7);
        assert_eq!(model.records.len(), 1);
    }
    #[test]
    fn overshooting_with_a_half_turn_continues_until_the_cube_is_solved_again() {
        let mut model = completion_model("R'");
        let at = Instant::now();
        recorded_turn(&mut model, "R", 1, Some(1000.), false, at);
        recorded_turn(
            &mut model,
            "R",
            2,
            Some(1020.),
            false,
            at + Duration::from_millis(20),
        );
        model.settle_completion(at + Duration::from_millis(200));
        assert!(model.active.is_some() && model.records.is_empty());
        assert!(model.completion.is_none());
        recorded_turn(
            &mut model,
            "R'",
            3,
            Some(1400.),
            false,
            at + Duration::from_millis(400),
        );
        model.settle_completion(at + Duration::from_millis(520));
        assert_eq!(model.latest.as_ref().unwrap()["duration"], 400.);
        assert_eq!(
            model.latest.as_ref().unwrap()["moves"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }
    #[test]
    fn immediate_unrelated_turn_starts_scramble_and_preserves_late_verification() {
        let mut model = completion_model("R'");
        let at = Instant::now();
        recorded_turn(&mut model, "R", 1, Some(1000.), false, at);
        recorded_turn(
            &mut model,
            "F",
            2,
            Some(1020.),
            false,
            at + Duration::from_millis(20),
        );
        assert!(model.active.is_none() && model.completion.is_none());
        assert_eq!(
            model.latest.as_ref().unwrap()["moves"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(model.latest.as_ref().unwrap()["duration"], 0.);
        assert_eq!(model.scramble.moves.len(), 1);
        assert_eq!(model.scramble.moves[0]["move"], "F");
        model.stream_event(StreamEvent::Verified(1), at + Duration::from_millis(30));
        assert!(
            model.latest.as_ref().unwrap()["quality"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(model.records.len(), 1);
    }
    #[test]
    fn completion_partner_window_respects_device_clock_and_unknown_timing_flags() {
        for (setup, first, second) in [("R'", "R", "L'"), ("U'", "U", "D'"), ("F", "F'", "B")] {
            let at = Instant::now();
            let mut within = completion_model(setup);
            recorded_turn(&mut within, first, 1, Some(1000.), false, at);
            recorded_turn(
                &mut within,
                second,
                2,
                Some(1100.),
                false,
                at + Duration::from_millis(20),
            );
            assert!(within.active.is_some() && within.records.is_empty());
            let mut outside = completion_model(setup);
            recorded_turn(&mut outside, first, 1, Some(1000.), false, at);
            recorded_turn(
                &mut outside,
                second,
                2,
                Some(1101.),
                false,
                at + Duration::from_millis(20),
            );
            assert!(outside.active.is_none());
            assert_eq!(outside.records.len(), 1);
        }
        let at = Instant::now();
        let mut recovered = completion_model("R'");
        recorded_turn(&mut recovered, "R", 1, Some(1000.), false, at);
        recorded_turn(
            &mut recovered,
            "L'",
            2,
            None,
            true,
            at + Duration::from_millis(20),
        );
        assert!(recovered.records.is_empty());
        assert!(
            recovered
                .active
                .as_ref()
                .unwrap()
                .quality
                .contains(&"estimated-timing".into())
        );
        recorded_turn(
            &mut recovered,
            "L",
            3,
            Some(1200.),
            false,
            at + Duration::from_millis(200),
        );
        recovered.stream_event(StreamEvent::Verified(3), at + Duration::from_millis(210));
        recovered.settle_completion(at + Duration::from_millis(320));
        assert_eq!(
            recovered.latest.as_ref().unwrap()["quality"],
            json!(["estimated-timing"])
        );
    }
    #[test]
    fn cancelling_or_disconnecting_a_candidate_preserves_interrupted_raw_turns() {
        for disconnect in [false, true] {
            let mut model = completion_model("R'");
            let at = Instant::now();
            recorded_turn(&mut model, "R", 1, Some(1000.), false, at);
            if disconnect {
                model.disconnect();
            } else {
                model.interrupt("user-stopped");
            }
            model.settle_completion(at + Duration::from_secs(1));
            assert!(model.active.is_none() && model.completion.is_none());
            assert_eq!(model.records.len(), 1);
            assert_eq!(model.records[0]["status"], "interrupted");
            assert_eq!(model.records[0]["moves"].as_array().unwrap().len(), 1);
        }
    }
    #[test]
    fn stale_recovery_cannot_replace_current_correction() {
        let mut m = Model::new(eframe::egui::Context::default(), true);
        m.api_tx = mpsc::channel().0;
        let state = Cube::default().turn(Move::parse("R").unwrap());
        m.state = Some(state);
        m.target = Some(Target {
            cube: Cube::default(),
            moves: vec![],
            stage: "solve".into(),
            case: None,
            algorithm: None,
        });
        m.recovery_request = Some((1, state, Cube::default()));
        m.recovery_revision = 2;
        m.recovery = sequence("U'").unwrap();
        m.api_event(ApiEvent {
            tag: "recovery:1".into(),
            result: Ok(json!({"moves":["R'"]})),
            sent: vec![],
        });
        assert_eq!(m.recovery, sequence("U'").unwrap());
    }
}
fn target(p: &Position, stage: String, case: Option<String>, algorithm: Option<String>) -> Target {
    Target {
        cube: Cube::parse(&p.state).unwrap(),
        moves: p.setup.iter().map(|m| Move::parse(m).unwrap()).collect(),
        stage,
        case,
        algorithm,
    }
}
pub const ANALYSIS_VERSION: u64 = 4;

pub fn analysis(record: &Value) -> Value {
    cfop::analyze(record)
}
