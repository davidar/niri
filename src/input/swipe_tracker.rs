use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug)]
pub struct SwipeTracker {
    history: VecDeque<Event>,
    pos: f64,
    /// Per-millisecond velocity retention for the fling projection.
    deceleration: f64,
    /// How much recent history feeds the velocity estimate.
    history_limit: Duration,
}

#[derive(Debug, Clone, Copy)]
struct Event {
    delta: f64,
    timestamp: Duration,
}

impl SwipeTracker {
    pub fn new(config: &niri_config::TouchpadSwipe) -> Self {
        Self {
            history: VecDeque::new(),
            pos: 0.,
            deceleration: config.deceleration,
            history_limit: Duration::from_millis(u64::from(config.velocity_window_ms)),
        }
    }

    /// Pushes a new reading into the tracker.
    pub fn push(&mut self, delta: f64, timestamp: Duration) {
        // For the events that we care about, timestamps should always increase
        // monotonically.
        if let Some(last) = self.history.back() {
            if timestamp < last.timestamp {
                trace!(
                    "ignoring event with timestamp {timestamp:?} earlier than last {:?}",
                    last.timestamp
                );
                return;
            }
        }

        self.history.push_back(Event { delta, timestamp });
        self.pos += delta;

        self.trim_history();
    }

    /// Returns the current gesture position.
    pub fn pos(&self) -> f64 {
        self.pos
    }

    /// Computes the current gesture velocity.
    pub fn velocity(&self) -> f64 {
        let (Some(first), Some(last)) = (self.history.front(), self.history.back()) else {
            return 0.;
        };

        let total_time = (last.timestamp - first.timestamp).as_secs_f64();
        if total_time == 0. {
            return 0.;
        }

        let total_delta = self.history.iter().map(|event| event.delta).sum::<f64>();
        total_delta / total_time
    }

    /// Computes the gesture end position after decelerating to a halt.
    pub fn projected_end_pos(&self) -> f64 {
        let vel = self.velocity();
        if !(0. < self.deceleration && self.deceleration < 1.) {
            // No usable deceleration curve: stop where the fingers are.
            return self.pos;
        }
        self.pos - vel / (1000. * self.deceleration.ln())
    }

    fn trim_history(&mut self) {
        let Some(&Event { timestamp, .. }) = self.history.back() else {
            return;
        };

        while let Some(first) = self.history.front() {
            if timestamp <= first.timestamp + self.history_limit {
                break;
            }

            let _ = self.history.pop_front();
        }
    }
}
