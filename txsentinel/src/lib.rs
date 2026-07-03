#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! TXSentinel behavior analysis primitives.

/// Event category observed by TXSentinel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    /// File content was written.
    FileWrite,
    /// File was renamed.
    FileRename,
    /// Process was spawned.
    ProcessSpawn,
    /// Security policy denied an access request.
    PermissionDenied,
}

/// A security-relevant event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    /// Event kind.
    pub kind: EventKind,
    /// Subject or process identifier.
    pub subject: u64,
    /// Monotonic timestamp supplied by the kernel.
    pub tick: u64,
}

/// Threat level derived from behavior.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ThreatLevel {
    /// No suspicious behavior.
    Clean,
    /// Worth recording for audit.
    Suspicious,
    /// Active intervention is recommended.
    Dangerous,
}

/// Fixed-window behavior analyzer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BehaviorAnalyzer {
    window_start_tick: u64,
    file_mutations: u16,
    process_spawns: u16,
    denials: u16,
    window_ticks: u64,
}

impl BehaviorAnalyzer {
    /// Creates an analyzer with a time window.
    pub const fn new(window_ticks: u64) -> Self {
        Self {
            window_start_tick: 0,
            file_mutations: 0,
            process_spawns: 0,
            denials: 0,
            window_ticks,
        }
    }

    /// Observes an event and returns the current threat level.
    pub fn observe(&mut self, event: Event) -> ThreatLevel {
        if event.tick.saturating_sub(self.window_start_tick) > self.window_ticks {
            self.window_start_tick = event.tick;
            self.file_mutations = 0;
            self.process_spawns = 0;
            self.denials = 0;
        }

        match event.kind {
            EventKind::FileWrite | EventKind::FileRename => {
                self.file_mutations = self.file_mutations.saturating_add(1);
            }
            EventKind::ProcessSpawn => {
                self.process_spawns = self.process_spawns.saturating_add(1);
            }
            EventKind::PermissionDenied => {
                self.denials = self.denials.saturating_add(1);
            }
        }

        self.level()
    }

    /// Returns the current level without changing counters.
    pub const fn level(&self) -> ThreatLevel {
        if self.file_mutations >= 10 || self.denials >= 5 {
            ThreatLevel::Dangerous
        } else if self.file_mutations >= 4 || self.process_spawns >= 8 || self.denials >= 2 {
            ThreatLevel::Suspicious
        } else {
            ThreatLevel::Clean
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzer_escalates_repeated_file_mutation() {
        let mut analyzer = BehaviorAnalyzer::new(100);
        let mut level = ThreatLevel::Clean;

        for tick in 0..10 {
            level = analyzer.observe(Event {
                kind: EventKind::FileWrite,
                subject: 1,
                tick,
            });
        }

        assert_eq!(level, ThreatLevel::Dangerous);
    }

    #[test]
    fn analyzer_resets_after_window() {
        let mut analyzer = BehaviorAnalyzer::new(5);
        for tick in 0..4 {
            analyzer.observe(Event {
                kind: EventKind::PermissionDenied,
                subject: 1,
                tick,
            });
        }

        let level = analyzer.observe(Event {
            kind: EventKind::FileWrite,
            subject: 1,
            tick: 20,
        });

        assert_eq!(level, ThreatLevel::Clean);
    }
}
