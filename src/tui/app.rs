use std::time::Instant;

pub struct App {
    pub frequencies: Vec<f64>,
    pub last_refresh: Option<Instant>,
    pub should_quit: bool,
    /// Scroll offset as (x, y). The UI renders content larger than the
    /// terminal area and uses this offset to pan around it.
    pub scroll_offset: (u16, u16),
}

impl App {
    pub fn new() -> Self {
        Self {
            frequencies: Vec::new(),
            last_refresh: None,
            should_quit: false,
            scroll_offset: (0, 0),
        }
    }

    pub fn refresh(&mut self) {
        self.frequencies = crate::hardware::cpu::freq::current_frequencies_mhz()
            .unwrap_or_default();
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
