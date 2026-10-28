use anyhow::{Context, Result, ensure};
use bluer::{Adapter, Device, gatt::remote::Characteristic};
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

#[derive(Clone)]
pub struct DeviceInfo {
    pub address: String,
    pub name: String,
}
pub enum Command {
    Scan,
    Connect(String),
    Write(Vec<u8>),
    Disconnect,
    PowerOn,
}
pub enum Event {
    Adapter { powered: bool, reason: String },
    Devices(Vec<DeviceInfo>),
    Connected(String),
    Disconnected(String),
    Bytes(Vec<u8>, Instant),
    Error(String),
}
fn emit(tx: &mpsc::Sender<Event>, ctx: &eframe::egui::Context, event: Event) {
    match &event {
        Event::Adapter { reason, .. } => eprintln!("Bluetooth: {reason}"),
        Event::Connected(_) => eprintln!("Bluetooth: GAN notifications ready"),
        Event::Disconnected(reason) | Event::Error(reason) => eprintln!("Bluetooth: {reason}"),
        _ => {}
    }
    let _ = tx.send(event);
    ctx.request_repaint();
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AdapterStatus {
    name: Option<String>,
    powered: bool,
}
impl AdapterStatus {
    fn invalidates(&self, previous: Option<&Self>) -> bool {
        !self.powered || previous.is_some_and(|old| old.name != self.name)
    }
    fn reason(&self) -> &'static str {
        if self.powered {
            "Bluetooth ready"
        } else if self.name.is_some() {
            "Bluetooth is off"
        } else {
            "No Bluetooth adapter found"
        }
    }
    fn accepts(&self, adapter_name: &str) -> bool {
        self.powered && self.name.as_deref() == Some(adapter_name)
    }
}

struct Link {
    device: Device,
    write: Characteristic,
    notifications: BoxStream<'static, Vec<u8>>,
    name: String,
    owned: bool,
}
// The slot is filled *before* Device1.Connect is sent. Dropping its future does
// not cancel that D-Bus operation: Disconnect must cancel the pending request.
// A pre-existing shared BlueZ link is never put into this slot.
type OwnedConnection = Arc<Mutex<Option<Device>>>;
struct Attempt {
    future: BoxFuture<'static, Result<Link>>,
    owned: OwnedConnection,
}
impl Attempt {
    fn new(adapter: Adapter, address: String) -> Self {
        let owned = Arc::new(Mutex::new(None));
        let cleanup = owned.clone();
        let future = async move {
            tokio::time::timeout(
                Duration::from_secs(18),
                connect(&adapter, &address, &cleanup),
            )
            .await
            .context("Cube connection timed out. Wake the cube and close other cube apps.")?
        }
        .boxed();
        Self { future, owned }
    }
    fn cancel(self) -> Option<Device> {
        let Self { future, owned } = self;
        drop(future);
        owned.lock().unwrap().take()
    }
}
struct Scan {
    adapter_name: String,
    future: BoxFuture<'static, Result<Vec<DeviceInfo>>>,
}
#[derive(Default)]
struct Worker {
    adapter: Option<Adapter>,
    status: Option<AdapterStatus>,
    filter_for: Option<String>,
    link: Option<Link>,
    attempt: Option<Attempt>,
    scan: Option<Scan>,
}
impl Worker {
    async fn release(&mut self) {
        self.scan = None;
        if let Some(attempt) = self.attempt.take() {
            disconnect_owned(attempt.cancel()).await;
        }
        if let Some(link) = self.link.take() {
            release_link(link).await;
        }
    }
    async fn refresh(
        &mut self,
        session: &bluer::Session,
        tx: &mpsc::Sender<Event>,
        ctx: &eframe::egui::Context,
    ) {
        // Adapters are removed and recreated with a new hci index on resume.
        // Refresh even when the cached adapter used to be valid.
        let adapter = session.default_adapter().await.ok();
        let status = AdapterStatus {
            name: adapter.as_ref().map(|a| a.name().to_owned()),
            powered: if let Some(a) = &adapter {
                a.is_powered().await.unwrap_or(false)
            } else {
                false
            },
        };
        let invalidates = status.invalidates(self.status.as_ref());
        self.adapter = adapter;
        if self.status.as_ref() != Some(&status) {
            emit(
                tx,
                ctx,
                Event::Adapter {
                    powered: status.powered,
                    reason: status.reason().into(),
                },
            );
        }
        self.status = Some(status);
        if invalidates {
            self.filter_for = None;
            let had_connection = self.link.is_some() || self.attempt.is_some();
            // Clear the subscription before any subsequent select can deliver
            // bytes or a completed connection from the old adapter.
            if had_connection {
                emit(
                    tx,
                    ctx,
                    Event::Disconnected(
                        "Bluetooth adapter changed or switched off; reconnect your cube".into(),
                    ),
                );
            }
            self.release().await;
        }
    }
    async fn prepare_discovery(&mut self) -> Result<Adapter> {
        let adapter = self
            .adapter
            .as_ref()
            .context("No Bluetooth adapter found")?;
        ensure!(
            self.status.as_ref().is_some_and(|s| s.powered),
            "Bluetooth is off"
        );
        // BlueZ rejects changing our discovery filter during an active scan.
        // Reuse one filter for this adapter and cancel old scans before reuse.
        if self.filter_for.as_deref() != Some(adapter.name()) {
            adapter
                .set_discovery_filter(bluer::DiscoveryFilter {
                    transport: bluer::DiscoveryTransport::Le,
                    ..Default::default()
                })
                .await?;
            self.filter_for = Some(adapter.name().to_owned());
        }
        Ok(adapter.clone())
    }
}

async fn disconnect_owned(device: Option<Device>) {
    if let Some(device) = device {
        let result = tokio::time::timeout(Duration::from_secs(2), device.disconnect()).await;
        if !matches!(result, Ok(Ok(()))) {
            eprintln!("Bluetooth: connection cleanup did not complete");
        }
    }
}
async fn release_link(link: Link) {
    let Link {
        device,
        notifications,
        owned,
        ..
    } = link;
    drop(notifications);
    disconnect_owned(owned.then_some(device)).await;
}

pub fn worker(ctx: eframe::egui::Context) -> (UnboundedSender<Command>, mpsc::Receiver<Event>) {
    let (tx, mut rx) = unbounded_channel();
    let (otx, orx) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let session = match bluer::Session::new().await {
                Ok(session) => session,
                Err(e) => {
                    emit(&otx, &ctx, Event::Adapter { powered: false, reason: format!("Bluetooth service unavailable: {e}") });
                    return;
                }
            };
            let mut state = Worker::default();
            let mut tick = tokio::time::interval(Duration::from_millis(400));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    // Power/adapter changes take precedence over queued bytes
                    // and connection results. One observer owns their ordering.
                    biased;
                    _ = tick.tick() => {
                        state.refresh(&session, &otx, &ctx).await;
                        if let Some(link) = &state.link
                            && !link.device.is_connected().await.unwrap_or(false)
                        {
                            state.release().await;
                            emit(&otx, &ctx, Event::Disconnected("Cube disconnected; wake it and reconnect".into()));
                        }
                    }
                    command = rx.recv() => {
                        let Some(command) = command else {
                            state.release().await;
                            break;
                        };
                        match command {
                            Command::Disconnect => {
                                state.release().await;
                                emit(&otx, &ctx, Event::Disconnected("Disconnected".into()));
                            }
                            Command::Write(bytes) => {
                                if let Some(link) = &state.link {
                                    let result = tokio::time::timeout(Duration::from_secs(2), link.write.write(&bytes)).await;
                                    let error = match result {
                                        Ok(Ok(())) => None,
                                        Ok(Err(e)) => Some(format!("Bluetooth write failed: {e}")),
                                        Err(_) => Some("Bluetooth write timed out; reconnect your cube".into()),
                                    };
                                    if let Some(reason) = error {
                                        state.release().await;
                                        emit(&otx, &ctx, Event::Disconnected(reason));
                                    }
                                }
                            }
                            Command::PowerOn => {
                                state.refresh(&session, &otx, &ctx).await;
                                if let Some(adapter) = &state.adapter {
                                    if let Err(e) = adapter.set_powered(true).await {
                                        emit(&otx, &ctx, Event::Error(format!("Could not enable Bluetooth: {e}. Enable it in system settings.")));
                                    }
                                } else {
                                    emit(&otx, &ctx, Event::Error("No Bluetooth adapter found".into()));
                                }
                                state.refresh(&session, &otx, &ctx).await;
                            }
                            Command::Scan => {
                                state.refresh(&session, &otx, &ctx).await;
                                if state.attempt.is_some() {
                                    continue;
                                }
                                state.scan = None;
                                match state.prepare_discovery().await {
                                    Ok(adapter) => state.scan = Some(Scan {
                                        adapter_name: adapter.name().to_owned(),
                                        future: async move { scan(&adapter).await }.boxed(),
                                    }),
                                    Err(e) => emit(&otx, &ctx, Event::Error(format!("{e:#}"))),
                                }
                            }
                            Command::Connect(address) => {
                                state.refresh(&session, &otx, &ctx).await;
                                state.release().await;
                                match state.prepare_discovery().await {
                                    Ok(adapter) => {
                                        eprintln!("Bluetooth: connecting through {}", adapter.name());
                                        state.attempt = Some(Attempt::new(adapter, address));
                                    }
                                    Err(e) => emit(&otx, &ctx, Event::Error(format!("{e:#}"))),
                                }
                            }
                        }
                    }
                    result = async { state.attempt.as_mut().unwrap().future.as_mut().await }, if state.attempt.is_some() => {
                        let attempt = state.attempt.take().unwrap();
                        match result {
                            Ok(link) => {
                                // Transfer ownership before dropping the attempt;
                                // refresh again before publishing a successful link.
                                attempt.owned.lock().unwrap().take();
                                drop(attempt);
                                let adapter_name = link.device.adapter_name().to_owned();
                                state.link = Some(link);
                                state.refresh(&session, &otx, &ctx).await;
                                if state.status.as_ref().is_some_and(|s| s.accepts(&adapter_name)) {
                                    if let Some(link) = &state.link {
                                        emit(&otx, &ctx, Event::Connected(link.name.clone()));
                                    }
                                } else {
                                    state.release().await;
                                }
                            }
                            Err(error) => {
                                disconnect_owned(attempt.cancel()).await;
                                emit(&otx, &ctx, Event::Error(format!("{error:#}")));
                            }
                        }
                    }
                    result = async { state.scan.as_mut().unwrap().future.as_mut().await }, if state.scan.is_some() => {
                        let scan = state.scan.take().unwrap();
                        state.refresh(&session, &otx, &ctx).await;
                        if state.status.as_ref().is_some_and(|s| s.accepts(&scan.adapter_name)) {
                            match result {
                                Ok(devices) => emit(&otx, &ctx, Event::Devices(devices)),
                                Err(error) => emit(&otx, &ctx, Event::Error(format!("{error:#}"))),
                            }
                        }
                    }
                    bytes = async { state.link.as_mut().unwrap().notifications.next().await }, if state.link.is_some() => {
                        match bytes {
                            Some(bytes) => emit(&otx, &ctx, Event::Bytes(bytes, Instant::now())),
                            None => {
                                state.release().await;
                                emit(&otx, &ctx, Event::Disconnected("Cube notifications ended; reconnect your cube".into()));
                            }
                        }
                    }
                }
            }
        });
    });
    (tx, orx)
}

async fn scan(a: &Adapter) -> Result<Vec<DeviceInfo>> {
    ensure!(
        a.is_powered().await?,
        "Bluetooth is off. Turn it on before scanning."
    );
    let discovery = a.discover_devices().await?;
    futures::pin_mut!(discovery);
    let deadline = tokio::time::sleep(Duration::from_secs(4));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            _ = &mut deadline => break,
            event = discovery.next() => { ensure!(event.is_some(), "Bluetooth discovery ended"); }
        }
    }
    ensure!(
        a.is_powered().await?,
        "Bluetooth was switched off during scanning"
    );
    let mut found = vec![];
    for address in a.device_addresses().await? {
        let d = a.device(address)?;
        // Nearby devices can disappear while enumerating. One stale object
        // must not discard the entire scan, including the user's cube.
        let name = d.name().await.ok().flatten().unwrap_or_default();
        if name.starts_with("GAN") || name.starts_with("MG") || name.starts_with("AiCube") {
            found.push(DeviceInfo {
                address: address.to_string(),
                name,
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

async fn connect(a: &Adapter, address: &str, owned: &OwnedConnection) -> Result<Link> {
    ensure!(a.is_powered().await?, "Bluetooth is off");
    let address = address.parse().context("Invalid cube address")?;
    // Keep LE discovery active while BlueZ establishes the link. Device cache
    // presence alone does not mean the cube is currently awake/advertising.
    let discovery = a.discover_devices().await?;
    futures::pin_mut!(discovery);
    let deadline = tokio::time::sleep(Duration::from_secs(8));
    tokio::pin!(deadline);
    while !a.device_addresses().await?.contains(&address) {
        tokio::select! {
            _ = &mut deadline => anyhow::bail!("The saved cube is not advertising. Wake it with a turn, and disconnect it from other cube apps."),
            event = discovery.next() => { ensure!(event.is_some(), "Bluetooth discovery ended before the cube appeared"); },
            _ = tokio::time::sleep(Duration::from_millis(150)) => { ensure!(a.is_powered().await?, "Bluetooth was switched off"); }
        }
    }
    let device = a.device(address)?;
    let initiated = !device
        .is_connected()
        .await
        .context("Could not read cube connection status")?;
    if initiated {
        *owned.lock().unwrap() = Some(device.clone());
        device
            .connect()
            .await
            .context("Could not connect. Wake the cube and close other cube apps")?;
    }
    eprintln!("Bluetooth: reading GAN services");
    let mut write = None;
    let mut notify = None;
    for service in device
        .services()
        .await
        .context("Could not read cube services")?
    {
        for characteristic in service.characteristics().await? {
            match characteristic.uuid().await?.to_string().as_str() {
                "0000fff5-0000-1000-8000-00805f9b34fb" => write = Some(characteristic),
                "0000fff6-0000-1000-8000-00805f9b34fb" => notify = Some(characteristic),
                _ => {}
            }
        }
    }
    let write = write.context("This cube does not expose GAN Gen4 services")?;
    let notify = notify.context("GAN notifications are unavailable")?;
    let notifications = notify
        .notify()
        .await
        .context("Could not subscribe to GAN notifications; close other cube apps and retry")?
        .boxed();
    let name = device.name().await?.unwrap_or_else(|| "GAN cube".into());
    Ok(Link {
        device,
        write,
        notifications,
        name,
        owned: initiated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn status(name: Option<&str>, powered: bool) -> AdapterStatus {
        AdapterStatus {
            name: name.map(str::to_owned),
            powered,
        }
    }
    #[test]
    fn adapter_replacement_invalidates_even_when_both_are_powered() {
        let before = status(Some("hci0"), true);
        let after = status(Some("hci1"), true);
        assert!(after.invalidates(Some(&before)));
        assert!(!after.accepts("hci0"));
        assert!(after.accepts("hci1"));
        assert!(!after.invalidates(Some(&after)));
    }
    #[test]
    fn power_off_and_removal_reject_late_connections() {
        let before = status(Some("hci0"), true);
        for after in [status(Some("hci0"), false), status(None, false)] {
            assert!(after.invalidates(Some(&before)));
            assert!(!after.accepts("hci0"));
        }
        let resumed = status(Some("hci0"), true);
        assert!(resumed.accepts("hci0"));
        assert!(!resumed.invalidates(Some(&status(Some("hci0"), false))));
    }
    #[tokio::test]
    async fn cancelling_attempt_drops_pending_connect_future() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct Dropped(Arc<AtomicBool>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let guard = Dropped(dropped.clone());
        let future = async move {
            let _guard = guard;
            futures::future::pending::<Result<Link>>().await
        }
        .boxed();
        let mut attempt = Attempt {
            future,
            owned: Arc::new(Mutex::new(None)),
        };
        assert!(futures::poll!(attempt.future.as_mut()).is_pending());
        assert!(attempt.cancel().is_none());
        assert!(dropped.load(Ordering::SeqCst));
    }
}
