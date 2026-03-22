use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct GovernorConfig {
    pub idle_rss_limit_bytes: u64,
    pub hard_rss_limit_bytes: u64,
    pub cpu_limit_percent: f32,
    pub poll_interval: Duration,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            idle_rss_limit_bytes: 50 * 1024 * 1024,
            hard_rss_limit_bytes: 128 * 1024 * 1024,
            cpu_limit_percent: 25.0,
            poll_interval: Duration::from_millis(250),
        }
    }
}

impl GovernorConfig {
    pub fn recommended_ast_cache_budget_bytes(&self) -> usize {
        ((self.hard_rss_limit_bytes / 5) as usize).min(16 * 1024 * 1024)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Allow,
    Delay,
}

pub struct ResourceGovernor {
    config: GovernorConfig,
    system: System,
    pid: Pid,
    last_throttle: Instant,
    primed: bool,
}

impl ResourceGovernor {
    pub fn new(config: GovernorConfig) -> Result<Self> {
        let pid = Pid::from_u32(std::process::id());
        Ok(Self {
            config,
            system: System::new(),
            pid,
            last_throttle: Instant::now(),
            primed: false,
        })
    }

    pub fn admission(&mut self) -> Admission {
        self.refresh();

        let Some(process) = self.system.process(self.pid) else {
            return Admission::Allow;
        };

        let memory_bytes = process.memory();
        if memory_bytes >= self.config.hard_rss_limit_bytes {
            warn!(
                rss_bytes = memory_bytes,
                hard_limit_bytes = self.config.hard_rss_limit_bytes,
                "livingdocs governor paused dequeue due to RSS ceiling"
            );
            return Admission::Delay;
        }

        if process.cpu_usage() >= self.config.cpu_limit_percent {
            debug!(
                cpu = process.cpu_usage(),
                limit = self.config.cpu_limit_percent,
                "livingdocs governor delaying dequeue due to CPU pressure"
            );
            return Admission::Delay;
        }

        Admission::Allow
    }

    pub fn wait_for_budget(&mut self) {
        while self.admission() == Admission::Delay {
            thread::sleep(self.config.poll_interval);
        }
    }

    pub fn cooperative_yield(&mut self) {
        self.refresh();
        let Some(process) = self.system.process(self.pid) else {
            return;
        };

        if process.cpu_usage() >= self.config.cpu_limit_percent {
            thread::sleep(self.config.poll_interval);
            self.last_throttle = Instant::now();
            return;
        }

        if process.memory() >= self.config.idle_rss_limit_bytes
            && self.last_throttle.elapsed() >= Duration::from_millis(500)
        {
            thread::sleep(Duration::from_millis(50));
            self.last_throttle = Instant::now();
        }
    }

    pub fn hard_rss_limit_bytes(&self) -> u64 {
        self.config.hard_rss_limit_bytes
    }

    pub fn idle_rss_limit_bytes(&self) -> u64 {
        self.config.idle_rss_limit_bytes
    }

    pub fn current_rss_bytes(&mut self) -> Option<u64> {
        self.refresh();
        self.system.process(self.pid).map(|process| process.memory())
    }

    fn refresh(&mut self) {
        if !self.primed {
            self.system
                .refresh_processes(ProcessesToUpdate::Some(&[self.pid]), true);
            thread::sleep(Duration::from_millis(120));
            self.primed = true;
        }
        self.system
            .refresh_processes(ProcessesToUpdate::Some(&[self.pid]), true);
    }
}
