use crate::hardware::metric::Metric;

/// Central catalogue of available metrics.
///
/// Adding a new CPU parameter only requires registering it here (typically in
/// [`Registry::default`]); the CLI and TUI discover it automatically without
/// needing changes to dispatchers or UI code.
#[derive(Default)]
pub struct Registry {
    metrics: Vec<Box<dyn Metric>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, metric: Box<dyn Metric>) {
        self.metrics.push(metric);
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Metric> {
        self.metrics.iter().map(|m| m.as_ref())
    }

    pub fn len(&self) -> usize {
        self.metrics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }

    /// Build a registry populated with the default set of CPU metrics.
    pub fn default_cpu() -> Self {
        let mut reg = Self::new();
        for m in crate::hardware::cpu::default_metrics() {
            reg.register(m);
        }
        reg
    }
}
