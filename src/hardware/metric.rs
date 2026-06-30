use crate::hardware::source::Source;
use anyhow::Result;

/// A single observed value for a [`Metric`].
///
/// Variants carry both the data and a hint about how to format/interpret it.
/// `Series` is used for per-core (or otherwise indexed) readings.
#[derive(Debug, Clone)]
pub enum Value {
    Freq(f64),
    Temp(f64),
    Percent(f64),
    Bool(bool),
    Enum(String),
    Raw(String),
    Series(Vec<Value>),
}

impl Value {
    pub fn nan_freq() -> Self {
        Self::Freq(f64::NAN)
    }
}

/// A readable (and optionally writable) hardware parameter.
///
/// Implementations are stateless: they receive the [`Source`] at call time so
/// the same struct can be exercised against a fake backend in tests. Register
/// instances via [`crate::hardware::registry::Registry`] to expose them to the
/// CLI and TUI without touching dispatch code.
pub trait Metric: Send + Sync {
    /// Stable identifier, e.g. `"cpu.freq.cur"`.
    fn id(&self) -> &str;

    /// Human-readable name shown in UIs.
    fn label(&self) -> &str;

    /// Unit suffix, e.g. `"MHz"`. Empty string when not applicable.
    fn unit(&self) -> &str {
        ""
    }

    /// Coarse grouping used to organise CLI/TUI views, e.g. `"cpu"`.
    fn category(&self) -> &str {
        "cpu"
    }

    /// Read the current value.
    fn read(&self, source: &dyn Source) -> Result<Value>;

    /// Whether this parameter can be set. Defaults to `false`.
    fn is_writable(&self) -> bool {
        false
    }

    /// Write a new value. Only called when [`is_writable`] returns `true`.
    fn write(&self, _source: &dyn Source, _value: &Value) -> Result<()> {
        anyhow::bail!("metric {} is read-only", self.id())
    }
}
