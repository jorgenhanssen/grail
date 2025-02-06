use uci::commands::GoParams;

pub struct SearchController {
    start_time: std::time::Instant,
    allocated_time: Option<u64>,
    max_depth: Option<u64>,
}

impl SearchController {
    pub fn new(params: &GoParams) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            allocated_time: params.move_time,
            max_depth: params.depth,
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    #[inline(always)]
    pub fn continue_search(&self, depth: u64) -> bool {
        // return depth <= 6;
        // return self.start_time.elapsed().as_millis() < 10_000;

        // Check time limit if it exists
        if let Some(allocated_time) = self.allocated_time {
            if self.start_time.elapsed().as_millis() >= allocated_time as u128 {
                return false;
            }
        }

        // Check depth limit if it exists
        if let Some(max_depth) = self.max_depth {
            if depth > max_depth {
                return false;
            }
        }

        // If neither limit is set, use a default time of 10 seconds
        self.start_time.elapsed().as_millis() < 10_000
    }
}
