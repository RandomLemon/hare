use crate::hardware::{Registry, SysfsSource, Value};
use std::time::Instant;

/// One sampled metric rendered by the TUI.
#[derive(Clone)]
#[allow(dead_code)] // `id`/`unit` reserved for future tabs/detail views
pub struct SnapshotEntry {
    pub id: String,
    pub label: String,
    pub unit: String,
    pub value: Value,
}

pub struct App {
    pub registry: Registry,
    pub snapshot: Vec<SnapshotEntry>,
    pub last_refresh: Option<Instant>,
    pub should_quit: bool,
    /// Scroll offset as (x, y). The UI renders content larger than the
    /// terminal area and uses this offset to pan around it.
    pub scroll_offset: (u16, u16),
}

impl App {
    pub fn new() -> Self {
        let registry = Registry::default_cpu();
        Self {
            registry,
            snapshot: Vec::new(),
            last_refresh: None,
            should_quit: false,
            scroll_offset: (0, 0),
        }
    }

    pub fn refresh(&mut self) {
        let source = SysfsSource::new();
        self.snapshot = self
            .registry
            .iter()
            .filter_map(|m| match m.read(&source) {
                Ok(value) => Some(SnapshotEntry {
                    id: m.id().to_string(),
                    label: m.label().to_string(),
                    unit: m.unit().to_string(),
                    value,
                }),
                // Skip metrics that cannot be read on this machine rather than
                // blanking the whole view.
                Err(_) => None,
            })
            .collect();
        self.last_refresh = Some(Instant::now());
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset.1 = self.scroll_offset.1.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset.1 = self.scroll_offset.1.saturating_add(amount);
    }

    pub fn scroll_left(&mut self, amount: u16) {
        self.scroll_offset.0 = self.scroll_offset.0.saturating_sub(amount);
    }

    pub fn scroll_right(&mut self, amount: u16) {
        self.scroll_offset.0 = self.scroll_offset.0.saturating_add(amount);
    }
}
