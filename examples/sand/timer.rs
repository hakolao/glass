use std::time::Instant;

const NUM_TIME_SAMPLES: usize = 100;

/// A simple performance timer, averaging frame time over the last [`NUM_TIME_SAMPLES`] frames.
#[derive(Debug, Copy, Clone)]
pub struct Timer {
    time: Instant,
    time_sum: f64,
    avg_ms: f64,
    num_updates: usize,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            time: Instant::now(),
            time_sum: 0.0,
            avg_ms: 0.0,
            num_updates: 0,
        }
    }

    /// Records the time since the previous call. Call once per frame.
    pub fn update(&mut self) {
        let diff = self.time.elapsed().as_nanos() as f64 * 1.0e-6;
        if self.num_updates >= NUM_TIME_SAMPLES {
            self.avg_ms = self.time_sum / self.num_updates as f64;
            self.time_sum = 0.0;
            self.num_updates = 0;
        }
        self.time_sum += diff;
        self.num_updates += 1;
        self.time = Instant::now();
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
