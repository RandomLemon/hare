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
            Page::Monitor => 'm',
            Page::Control => 'c',
            Page::Preset => 'p',
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
    Cluster,
}

impl MonitorTab {
    /// Display name shown in the sidebar tab.
    pub fn name(&self) -> &'static str {
        match self {
            MonitorTab::Overview => "Overview",
            MonitorTab::Governor => "Governor",
            MonitorTab::Cluster => "Cluster",
        }
    }

    /// Metrics whose id starts with this prefix are shown on list-style
    /// sub-pages. `None` means the page renders custom content (e.g. a table
    /// aggregating several metrics by core index).
    pub fn metric_prefix(&self) -> Option<&'static str> {
        match self {
            MonitorTab::Overview => None,
            MonitorTab::Governor => Some("cpu.governor"),
            MonitorTab::Cluster => None,
        }
    }

    /// Sub-tabs in display order.
    pub const ALL: [MonitorTab; 3] = [
        MonitorTab::Overview,
        MonitorTab::Governor,
        MonitorTab::Cluster,
    ];

    /// Next sub-tab in display order (wraps around).
    pub fn next(self) -> Self {
        let idx = Self::ALL
            .as_slice()
            .iter()
            .position(|t| *t == self)
            .unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Previous sub-tab in display order (wraps around).
    pub fn prev(self) -> Self {
        let idx = Self::ALL
            .as_slice()
            .iter()
            .position(|t| *t == self)
            .unwrap_or(0);
        let len = Self::ALL.len();
        Self::ALL[(idx + len - 1) % len]
    }
}

/// A sub-tab within the Control page, selectable from the left sidebar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlTab {
    /// Toggle per-core online/offline status.
    Online,
    /// Set per-core minimum/maximum scaling frequency.
    FreqLimit,
}

impl ControlTab {
    pub fn name(&self) -> &'static str {
        match self {
            ControlTab::Online => "Online",
            ControlTab::FreqLimit => "Freq Limit",
        }
    }

    pub const ALL: [ControlTab; 2] = [ControlTab::Online, ControlTab::FreqLimit];

    pub fn next(self) -> Self {
        let idx = Self::ALL
            .as_slice()
            .iter()
            .position(|t| *t == self)
            .unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL
            .as_slice()
            .iter()
            .position(|t| *t == self)
            .unwrap_or(0);
        let len = Self::ALL.len();
        Self::ALL[(idx + len - 1) % len]
    }
}

/// Which frequency column is focused in the Freq Limit control tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FreqCol {
    Min,
    Max,
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
    /// Active sub-tab within the Control page.
    pub control_tab: ControlTab,
    /// Per-core lockability of the `online` file (computed in `refresh`),
    /// indexed by core index. `false` means the core cannot be toggled (e.g.
    /// cpu0 has no `online` file) and is shown as "locked".
    pub online_lockable: Vec<bool>,
    /// Currently highlighted core in the Control pages.
    pub selected_core: usize,
    /// Focused column in the Freq Limit tab.
    pub freq_col: FreqCol,
    /// Active frequency input buffer; `Some` means the Freq Limit tab is in
    /// edit mode for the focused (core, column).
    pub freq_edit: Option<String>,
    /// Last control action result message (success or error), shown inline.
    pub control_message: Option<String>,
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
            control_tab: ControlTab::Online,
            online_lockable: Vec::new(),
            selected_core: 0,
            freq_col: FreqCol::Min,
            freq_edit: None,
            control_message: None,
            scroll_offset: (0, 0),
        }
    }

    /// Switch to `page`; resets scroll/cursor state when the page changes.
    pub fn set_page(&mut self, page: Page) {
        if self.current_page != page {
            self.current_page = page;
            self.scroll_offset = (0, 0);
            self.freq_edit = None;
            self.control_message = None;
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

    /// Switch to a Control sub-tab; resets cursor/edit/message on change.
    pub fn set_control_tab(&mut self, tab: ControlTab) {
        if self.control_tab != tab {
            self.control_tab = tab;
            self.selected_core = 0;
            self.freq_edit = None;
            self.control_message = None;
            self.scroll_offset = (0, 0);
        }
    }

    /// Cycle to the next Control sub-tab (Tab key), unless editing.
    pub fn next_control_tab(&mut self) {
        if self.freq_edit.is_some() {
            return;
        }
        self.set_control_tab(self.control_tab.next());
    }

    /// Cycle to the next Control sub-tab (Shift+Tab), unless editing.
    pub fn prev_control_tab(&mut self) {
        if self.freq_edit.is_some() {
            return;
        }
        self.set_control_tab(self.control_tab.prev());
    }

    /// Cycle the active page's sub-tab forward (Tab key).
    pub fn cycle_sub_tab_next(&mut self) {
        match self.current_page {
            Page::Monitor => self.next_monitor_tab(),
            Page::Control => self.next_control_tab(),
            Page::Preset => {}
        }
    }

    /// Cycle the active page's sub-tab backward (Shift+Tab key).
    pub fn cycle_sub_tab_prev(&mut self) {
        match self.current_page {
            Page::Monitor => self.prev_monitor_tab(),
            Page::Control => self.prev_control_tab(),
            Page::Preset => {}
        }
    }

    /// Esc semantics: cancel an active freq edit first; otherwise return to the
    /// monitor page, or quit if already there.
    pub fn escape(&mut self) {
        if self.freq_edit.take().is_some() {
            return;
        }
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

        // Per-core online file presence, for the Online control tab's "locked"
        // indicator. Aligned to the online series length (core count).
        let core_count = self
            .snapshot
            .iter()
            .find(|e| e.id == "cpu.topology.online")
            .and_then(|e| match &e.value {
                Value::Series(v) => Some(v.len()),
                _ => None,
            })
            .unwrap_or(0);
        self.online_lockable = (0..core_count)
            .map(|i| crate::hardware::cpu::topology::core_has_online_file(&source, i))
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

    // ----- Control actions -----

    /// Number of cores visible in the Control pages (max per-core series len).
    fn control_core_count(&self) -> usize {
        [
            "cpu.topology.online",
            "cpu.freq.min",
            "cpu.freq.max",
            "cpu.topology.cluster",
        ]
        .iter()
        .filter_map(|id| self.snapshot.iter().find(|e| e.id.as_str() == *id))
        .filter_map(|e| match &e.value {
            Value::Series(v) => Some(v.len()),
            _ => None,
        })
        .max()
        .unwrap_or(0)
    }

    /// Move the core cursor vertically (Online + FreqLimit tabs).
    pub fn move_cursor_vertical(&mut self, down: bool) {
        let n = self.control_core_count();
        if n == 0 {
            return;
        }
        let i = self.selected_core;
        self.selected_core = if down {
            (i + 1).min(n - 1)
        } else {
            i.saturating_sub(1)
        };
    }

    /// Move the column cursor horizontally (FreqLimit tab only).
    pub fn move_cursor_horizontal(&mut self, left: bool) {
        // No-op while editing.
        if self.freq_edit.is_some() {
            return;
        }
        self.freq_col = if left { FreqCol::Min } else { FreqCol::Max };
    }

    /// Toggle the selected core's online/offline status (Online tab).
    pub fn toggle_online(&mut self) {
        let core = self.selected_core;
        self.control_message = Some(self.do_toggle_online(core));
        self.refresh();
    }

    fn do_toggle_online(&self, core: usize) -> String {
        let Some(lockable) = self.online_lockable.get(core).copied() else {
            return format!("core {}: unknown", core);
        };
        if !lockable {
            return format!("core {}: locked (cannot offline)", core);
        }

        let current_online = self
            .snapshot
            .iter()
            .find(|e| e.id == "cpu.topology.online")
            .and_then(|e| match &e.value {
                Value::Series(v) => v.get(core),
                _ => None,
            })
            .and_then(|v| match v {
                Value::Bool(b) => Some(*b),
                _ => None,
            });
        let Some(current) = current_online else {
            return format!("core {}: no online data", core);
        };

        let desired = !current;
        let source = SysfsSource::new();
        match self
            .registry
            .iter()
            .find(|m| m.id() == "cpu.topology.online")
        {
            Some(m) => {
                if !m.is_core_writable() {
                    return format!("core {}: not per-core writable", core);
                }
                match m.write_core(&source, core, &Value::Bool(desired)) {
                    Ok(()) => {
                        format!(
                            "core {}: {}",
                            core,
                            if desired { "online" } else { "offline" }
                        )
                    }
                    Err(e) => format!("core {}: {}", core, e),
                }
            }
            None => "metric cpu.topology.online not registered".to_string(),
        }
    }

    /// Append a digit to the freq input buffer, starting an edit if idle.
    pub fn freq_input(&mut self, c: char) {
        if !c.is_ascii_digit() {
            return;
        }
        let buf = self.freq_edit.get_or_insert_with(String::new);
        if buf.len() < 9 {
            buf.push(c);
        }
    }

    pub fn freq_backspace(&mut self) {
        if let Some(buf) = self.freq_edit.as_mut() {
            buf.pop();
            if buf.is_empty() {
                // Keep the edit session open with an empty buffer.
            }
        }
    }

    /// Commit the current freq input buffer to the focused (core, column).
    pub fn freq_commit(&mut self) {
        let Some(buf) = self.freq_edit.clone() else {
            return;
        };
        let msg = self.do_freq_commit(buf);
        self.freq_edit = None;
        self.control_message = Some(msg);
        self.refresh();
    }

    fn do_freq_commit(&self, buf: String) -> String {
        let core = self.selected_core;
        let col = self.freq_col;
        let Ok(mhz) = buf.parse::<f64>() else {
            return format!("core {}: invalid frequency '{}'", core, buf);
        };
        let id = match col {
            FreqCol::Min => "cpu.freq.min",
            FreqCol::Max => "cpu.freq.max",
        };
        let source = SysfsSource::new();
        match self.registry.iter().find(|m| m.id() == id) {
            Some(m) => {
                if !m.is_core_writable() {
                    return format!("core {}: not per-core writable", core);
                }
                match m.write_core(&source, core, &Value::Freq(mhz)) {
                    Ok(()) => format!("core {}: {} = {:.0} MHz", core, col_name(col), mhz),
                    Err(e) => format!("core {}: {}", core, e),
                }
            }
            None => format!("metric {} not registered", id),
        }
    }
}

fn col_name(col: FreqCol) -> &'static str {
    match col {
        FreqCol::Min => "min",
        FreqCol::Max => "max",
    }
}
