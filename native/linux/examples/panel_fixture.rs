// Synthetic session-bus fixture; run only under dbus-run-session.
#[path = "../src/panel.rs"]
mod panel;
fn main() {
    assert!(std::env::var("CSTIMER_PANEL_TEST").as_deref() == Ok("1"));
    let bridge = panel::Bridge::new(eframe::egui::Context::default());
    let mut connected = std::env::var("CSTIMER_FIXTURE_DISCONNECTED").as_deref() != Ok("1");
    for _ in 0..400 {
        bridge.publish(serde_json::json!({"version":1,"updatedAt":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,"connected":connected,"connection":{"powered":true,"connecting":false,"scanning":false,"saved":true,"name":"Synthetic cube","message":"","devices":[{"id":"fixture","name":"Synthetic cube"}]},"running":false,"time":"13.73","tpsLabel":"1.89","moves":26,"tokens":"D2 B2 U F2 D R2 F2 L2 U' L2 D2 F' R2 U' B R' U' L' D R2".split_whitespace().enumerate().map(|(i,m)|serde_json::json!({"text":m,"kind":if i<3{1}else if i==3{2}else{0}})).collect::<Vec<_>>()}));
        for control in bridge.take_cube_controls() {
            match control {
                panel::CubeControl::Disconnect => {
                    connected = false;
                    println!("Disconnect action received");
                }
                panel::CubeControl::Connect(id) => {
                    assert!(id == "saved" || id == "fixture");
                    connected = true;
                    println!("Connect action received");
                }
                panel::CubeControl::Scan => println!("Scan action received"),
                panel::CubeControl::PowerOn => println!("Power action received"),
            }
        }
        if bridge.take_show() {
            println!("Main window requested");
        }
        if bridge.take_panel_request() {
            println!("Panel action received");
        }
        if bridge.take_widget() {
            println!("Companion action received");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
