use crate::hardware::{Registry, SysfsSource, Value};
use std::time::Instant;

/// A TUI page selectable from the title-bar tabs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Monitor,
    Control,
    Preset,
}

impl Page {
    /// Shortcut digit shown in the tab label and handled by the keyboard.
    pub fn digit(&self) -> char {
        match self {
            Page::Monitor => '1',
            Page::Control => '2',
            Page::Preset => '3',
        }
    }

    /// Display name shown in the tab label (without the digit prefix).
    pub fn name(&self) -> &'static str {
        match self {
            Page::Monitor => "Monitor",
            Page::Control => "Control",
            Page::Preset => "Preset",
        }
    }

    /// Pages in display order.
    pub const ALL: [Page; 3] = [Page::Monitor, Page::Control, Page::Preset];
}

/// A sub-tab within the Monitor page, selectable from the left sidebar.
///
/// Adding a new monitor sub-page only requires adding a variant here, an entry
/// in `ALL`, and (for list-style pages) a `metric_prefix` mapping. Pages with
/// custom rendering return `None` from `metric_prefix` and get a dedicated
/// match arm in the content renderer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MonitorTab {
    Overview,
    Governor,
}

impl MonitorTab {
    /// Display name shown in the sidebar tab.
    pub fn name(&self) -> &'static str {
        match self {
            MonitorTab::Overview => "Overview",
            MonitorTab::Governor => "Governor",
        }
    }

    /// Metrics whose id starts with this prefix are shown on list-style
    /// sub-pages. `None` means the page renders custom content (e.g. a table
    /// aggregating several metrics by core index).
    pub fn metric_prefix(&self) -> Option<&'static str> {
        match self {
            MonitorTab::Overview => None,
            MonitorTab::Governor => Some("cpu.governor"),
        }
    }

    /// Sub-tabs in display order.
    pub const ALL: [MonitorTab; 2] = [MonitorTab::Overview, MonitorTab::Governor];

    /// Next sub-tab in display order (wraps around).
    pub fn next(self) -> Self {
        let idx = Self::ALL.as_slice().iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Previous sub-tab in display order (wraps around).
    pub fn prev(self) -> Self {
        let idx = Self::ALL.as_slice().iter().position(|t| *t == self).unwrap_or(0);
        let len = Self::ALL.len();
        Self::ALL[(idx + len - 1) % len]
    }
}

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
    pub current_page: Page,
    /// Active sub-tab within the Monitor page.
    pub monitor_tab: MonitorTab,
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
            current_page: Page::Monitor,
            monitor_tab: MonitorTab::Overview,
            scroll_offset: (0, 0),
        }
    }

    /// Switch to `page`; resets scroll to the top whenever the page changes.
    pub fn set_page(&mut self, page: Page) {
        if self.current_page != page {
            self.current_page = page;
            self.scroll_offset = (0, 0);
        }
    }

    /// Switch to a Monitor sub-tab; resets scroll to the top on change.
    pub fn set_monitor_tab(&mut self, tab: MonitorTab) {
        if self.monitor_tab != tab {
            self.monitor_tab = tab;
            self.scroll_offset = (0, 0);
        }
    }

    /// Cycle to the next Monitor sub-tab (Tab key).
    pub fn next_monitor_tab(&mut self) {
        let next = self.monitor_tab.next();
        self.set_monitor_tab(next);
    }

    /// Cycle to the previous Monitor sub-tab (Shift+Tab).
    pub fn prev_monitor_tab(&mut self) {
        let prev = self.monitor_tab.prev();
        self.set_monitor_tab(prev);
    }

    /// Esc semantics: return to the monitor page, or quit if already there.
    pub fn escape(&mut self) {
        if self.current_page == Page::Monitor {
            self.should_quit = true;
        } else {
            self.set_page(Page::Monitor);
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
