use std::sync::Arc;
use std::thread;
use std::time::Duration;
use uci::commands::GoParams;

pub struct SearchController {
    start_time: std::time::Instant,
    allocated_time: Option<u64>,
    max_depth: Option<u8>,
    _timer_handle: Option<thread::JoinHandle<()>>,
    on_stop_callback: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl SearchController {
    pub fn new(params: &GoParams) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            allocated_time: params.move_time,
            max_depth: params.depth,
            _timer_handle: None,
            on_stop_callback: None,
        }
    }

    pub fn on_stop<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_stop_callback = Some(Arc::new(callback));
    }

    pub fn start_timer(&mut self) {
        let Some(allocated_time) = self.allocated_time else {
            return;
        };
        let Some(callback) = &self.on_stop_callback else {
            return;
        };

        let duration = Duration::from_millis(allocated_time);
        let callback = Arc::clone(callback);

        let handle = thread::spawn(move || {
            thread::sleep(duration);
            callback();
        });

        self._timer_handle = Some(handle);
    }

    pub fn check_depth(&self, current_depth: u8) {
        let Some(max_depth) = self.max_depth else {
            return;
        };
        if current_depth <= max_depth {
            return;
        }
        let Some(ref callback) = self.on_stop_callback else {
            return;
        };

        callback();
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}
