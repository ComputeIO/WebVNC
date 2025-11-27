use std::time::{Duration, Instant};

/// A tiny event-loop abstraction suitable for driving periodic server ticks.
///
/// For the PoC this provides a `tick` method that returns immediately and
/// records last-run time. Eventually this will integrate with file descriptors
/// and the `libvncserver` event loop.
#[derive(Debug)]
pub struct EventLoop {
    tick_ms: u64,
    last_run: Option<Instant>,
}

impl EventLoop {
    /// Create a new event loop which intends to run `tick` every `tick_ms` ms.
    pub fn new(tick_ms: u64) -> Self {
        EventLoop {
            tick_ms,
            last_run: None,
        }
    }

    /// Run a single tick. Returns true if the loop would process any work;
    /// the PoC just updates last_run and returns Ok.
    pub fn tick(&mut self) -> Result<bool, String> {
        self.last_run = Some(Instant::now());
        Ok(true)
    }

    /// Get the configured tick interval.
    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(self.tick_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tick_works() {
        let mut e = EventLoop::new(50);
        assert_eq!(e.tick_interval().as_millis(), 50);
        assert!(e.tick().is_ok());
        assert!(e.last_run.is_some());
    }
}
