use std::time::Instant;

const NUM_TIME_SAMPLES: usize = 100;

/// A simple performance timer
#[derive(Debug, Copy, Clone)]
pub struct Timer {
    time: Instant,
    time_sum: f64,
    avg_ms: f64,
    num_updates: usize,
    delta: f64,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            time: Instant::now(),
            time_sum: 0.0,
            avg_ms: 0.0,
            num_updates: 0,
            delta: 0.0,
        }
    }

    #[allow(unused)]
    pub fn start(&mut self) {
        self.time = Instant::now()
    }

    #[allow(unused)]
    pub fn end(&self) -> f64 {
        self.time.elapsed().as_nanos() as f64 * 1.0e-6
    }

    #[allow(unused)]
    pub fn time_since_last_update_sec(&self) -> f64 {
        self.end() / 1000.0
    }

    pub fn update_with_diff(&mut self, diff: f64) {
        self.delta = diff;
        if self.num_updates >= NUM_TIME_SAMPLES {
            self.avg_ms = self.time_sum / self.num_updates as f64;
            // reset
            self.time_sum = 0.0;
            self.num_updates = 0;
        }
        self.time_sum += diff;
        self.num_updates += 1;
    }

    pub fn update(&mut self) {
        let diff = self.time.elapsed().as_nanos() as f64 * 1.0e-6;
        self.update_with_diff(diff);
        self.time = Instant::now();
    }

    #[allow(unused)]
    pub fn dt(&self) -> f64 {
        self.delta
    }

    #[allow(unused)]
    pub fn dt_sec(&self) -> f64 {
        self.delta / 1000.0
    }

    #[allow(unused)]
    pub fn avg_ms(&self) -> f64 {
        self.avg_ms
    }

    pub fn avg_fps(&self) -> f64 {
        1000.0 / self.avg_ms
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer::new()
    }
}
