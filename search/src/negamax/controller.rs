use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use uci::commands::GoParams;

pub struct SearchController {
    start_time: std::time::Instant,
    allocated_time: Option<u64>,
    max_depth: Option<u64>,
    _timer_handle: Option<thread::JoinHandle<()>>,
}

impl SearchController {
    pub fn new(params: &GoParams) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            // allocated_time: params.move_time,
            allocated_time: Some(5_000), // hardcoded for now
            max_depth: params.depth,
            _timer_handle: None,
        }
    }

    /// Start async time management by spawning a timer thread that will call the stop_callback after allocated time
    pub fn start_timer<F>(&mut self, stop_callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(allocated_time) = self.allocated_time {
            let duration = Duration::from_millis(allocated_time);

            let handle = thread::spawn(move || {
                thread::sleep(duration);
                stop_callback();
            });

            self._timer_handle = Some(handle);
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}
