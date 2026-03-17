//! Frame-based execution model
//!
//! The frame system provides a consistent execution model where
//! all agent operations are processed within discrete time frames.
//! This enables deterministic behavior and efficient resource management.

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration, Instant};
use tracing::{debug, info, trace};

/// A frame represents a discrete unit of execution time
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame number (monotonically increasing)
    pub number: u64,
    /// Frame start time
    pub start_time: Instant,
    /// Frame duration
    pub duration: Duration,
    /// Delta time from previous frame
    pub delta_time: Duration,
}

impl Frame {
    /// Create a new frame
    pub fn new(number: u64, start_time: Instant, duration: Duration, delta_time: Duration) -> Self {
        Self {
            number,
            start_time,
            duration,
            delta_time,
        }
    }

    /// Check if the frame has completed
    pub fn is_complete(&self) -> bool {
        Instant::now().duration_since(self.start_time) >= self.duration
    }

    /// Get remaining time in the frame
    pub fn remaining(&self) -> Duration {
        let elapsed = Instant::now().duration_since(self.start_time);
        self.duration.saturating_sub(elapsed)
    }
}

/// Context passed to frame handlers
#[derive(Debug, Clone)]
pub struct FrameContext {
    /// Current frame
    pub frame: Frame,
    /// Shared state
    pub state: Arc<RwLock<FrameState>>,
}

/// Shared frame state
#[derive(Debug, Default)]
pub struct FrameState {
    /// Active agent count
    pub active_agents: usize,
    /// Pending task count
    pub pending_tasks: usize,
    /// Messages processed this frame
    pub messages_processed: usize,
}

/// Frame executor that manages the frame loop
pub struct FrameExecutor {
    target_duration: Duration,
    frame_number: u64,
    last_frame_time: Instant,
    state: Arc<RwLock<FrameState>>,
}

impl FrameExecutor {
    /// Create a new frame executor
    pub fn new(target_fps: u64) -> Self {
        let target_duration = Duration::from_millis(1000 / target_fps);
        Self {
            target_duration,
            frame_number: 0,
            last_frame_time: Instant::now(),
            state: Arc::new(RwLock::new(FrameState::default())),
        }
    }

    /// Start the frame loop
    pub async fn run<F, Fut>(&mut self, mut frame_handler: F)
    where
        F: FnMut(FrameContext) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        info!(
            "Starting frame executor with target FPS: {}",
            1000 / self.target_duration.as_millis() as u64
        );

        let mut ticker = interval(self.target_duration);

        loop {
            ticker.tick().await;

            let now = Instant::now();
            let delta_time = now.duration_since(self.last_frame_time);
            self.last_frame_time = now;

            self.frame_number += 1;

            let frame = Frame::new(self.frame_number, now, self.target_duration, delta_time);

            let context = FrameContext {
                frame,
                state: Arc::clone(&self.state),
            };

            trace!("Frame {} started", self.frame_number);
            frame_handler(context).await;
            debug!("Frame {} completed", self.frame_number);
        }
    }

    /// Get the shared state
    pub fn state(&self) -> Arc<RwLock<FrameState>> {
        Arc::clone(&self.state)
    }

    /// Get current frame number
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let frame = Frame::new(
            1,
            Instant::now(),
            Duration::from_millis(16),
            Duration::from_millis(16),
        );
        assert_eq!(frame.number, 1);
    }

    #[tokio::test]
    async fn test_frame_executor() {
        let mut executor = FrameExecutor::new(60);
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);

        // Run for a few frames
        tokio::spawn(async move {
            executor
                .run(move |_ctx| {
                    let counter = Arc::clone(&counter_clone);
                    async move {
                        let mut c = counter.write().await;
                        *c += 1;
                    }
                })
                .await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let c = counter.read().await;
        assert!(*c >= 1);
    }
}
