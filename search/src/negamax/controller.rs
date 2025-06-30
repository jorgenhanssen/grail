use uci::commands::GoParams;

pub struct SearchController {
    start_time: std::time::Instant,
    allocated_time: Option<u64>,
    max_depth: Option<u64>,

    // Iteration timing tracking
    current_iteration_start: Option<std::time::Instant>,
    previous_iteration_duration: Option<std::time::Duration>,
}

impl SearchController {
    pub fn new(params: &GoParams) -> Self {
        Self {
            // allocated_time: params.move_time,
            allocated_time: Some(5_000), // hardcoded for now
            max_depth: params.depth,

            start_time: std::time::Instant::now(),
            current_iteration_start: None,
            previous_iteration_duration: None,
        }
    }

    #[inline(always)]
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    #[inline(always)]
    pub fn continue_search(&mut self, depth: u64) -> bool {
        if self.exceeds_depth_limit(depth) {
            return false;
        }

        let allocated_time_ms = self.allocated_time.unwrap_or(10_000);
        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;

        // Already over time
        if elapsed_ms >= allocated_time_ms {
            return false;
        }

        // Predict if we have enough time for next iteration
        if !self.has_time_for_next_iteration(allocated_time_ms, elapsed_ms) {
            return false;
        }

        // We can continue - start timing this iteration
        self.start_iteration();
        true
    }

    #[inline(always)]
    fn exceeds_depth_limit(&self, depth: u64) -> bool {
        self.max_depth.map_or(false, |max_depth| depth > max_depth)
    }

    #[inline(always)]
    fn has_time_for_next_iteration(&self, allocated_time_ms: u64, elapsed_ms: u64) -> bool {
        let Some(prev_duration) = self.previous_iteration_duration else {
            return true; // No data yet, optimistically continue
        };

        let remaining_time_ms = allocated_time_ms - elapsed_ms;
        let estimated_next_ms = prev_duration.as_millis() as u64 * 3; // 200% buffer

        estimated_next_ms <= remaining_time_ms
    }

    #[inline(always)]
    fn start_iteration(&mut self) {
        if let Some(iteration_start) = self.current_iteration_start {
            self.previous_iteration_duration = Some(iteration_start.elapsed());
        }
        self.current_iteration_start = Some(std::time::Instant::now());
    }
}
