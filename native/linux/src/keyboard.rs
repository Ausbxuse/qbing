use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Timer {
    held: Option<Instant>,
    started: Option<Instant>,
}
impl Timer {
    pub fn press(&mut self, now: Instant) -> Option<f64> {
        if let Some(start) = self.started.take() {
            self.held = None;
            return Some(now.duration_since(start).as_secs_f64());
        }
        self.held.get_or_insert(now);
        None
    }
    pub fn release(&mut self, now: Instant) {
        if let Some(held) = self.held.take()
            && now.duration_since(held) >= Duration::from_millis(500)
        {
            self.started = Some(now);
        }
    }
    pub fn cancel(&mut self) {
        self.held = None;
        self.started = None;
    }
    pub fn running(&self) -> bool {
        self.started.is_some()
    }
    pub fn holding(&self) -> bool {
        self.held.is_some()
    }
    pub fn ready(&self) -> bool {
        self.held
            .is_some_and(|t| t.elapsed() >= Duration::from_millis(500))
    }
    pub fn elapsed(&self) -> Option<f64> {
        self.started.map(|t| t.elapsed().as_secs_f64() * 1000.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hold_release_stop_does_not_restart_on_stop_release() {
        let now = Instant::now();
        let mut timer = Timer::default();
        timer.press(now);
        timer.release(now + Duration::from_millis(100));
        assert!(!timer.running());
        timer.press(now);
        timer.release(now + Duration::from_millis(600));
        assert!(timer.running());
        assert_eq!(timer.press(now + Duration::from_millis(2600)), Some(2.));
        timer.release(now + Duration::from_millis(2700));
        assert!(!timer.running());
    }
    #[test]
    fn cancel_discards_hold_and_active_timing() {
        let now = Instant::now();
        let mut timer = Timer::default();
        timer.press(now);
        timer.cancel();
        timer.release(now + Duration::from_secs(1));
        assert!(!timer.running());
        timer.press(now);
        timer.release(now + Duration::from_secs(1));
        timer.cancel();
        assert!(!timer.running());
    }
}
