use crate::hardware::source::Source;
use anyhow::Result;

/// A single observed value for a [`Metric`].
///
/// Variants carry both the data and a hint about how to format/interpret it.
/// `Series` is used for per-core (or otherwise indexed) readings.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // variants are part of the extensible domain model, used as new metrics land
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
    /// Human-readable rendering of a single value (no surrounding context).
    pub fn format(&self) -> String {
        match self {
            Value::Freq(v) => {
                if v.is_nan() {
                    "NaN".to_string()
                } else {
                    format!("{:.2} MHz", v)
                }
            }
            Value::Temp(v) => {
                if v.is_nan() {
                    "NaN".to_string()
                } else {
                    format!("{:.1} C", v)
                }
            }
            Value::Percent(v) => format!("{:.1}%", v),
            Value::Bool(b) => {
                if *b {
                    "online".to_string()
                } else {
                    "offline".to_string()
                }
            }
            Value::Enum(s) => s.clone(),
            Value::Raw(s) => s.trim().to_string(),
            Value::Series(vs) => {
                let parts: Vec<String> = vs.iter().map(|v| v.format()).collect();
                format!("[{}]", parts.join(", "))
            }
        }
    }

    /// Expand a (possibly per-core) value into display lines.
    ///
    /// Scalars yield a single line; `Series` yields one line per element
    /// prefixed with its index.
    pub fn lines(&self) -> Vec<String> {
        match self {
            Value::Series(vs) => vs
                .iter()
                .enumerate()
                .map(|(i, v)| format!("#{}: {}", i, v.format()))
                .collect(),
            other => vec![other.format()],
        }
    }
}

/// A readable (and optionally writable) hardware parameter.
///
/// Implementations are usually stateless: they receive the [`Source`] at call
/// time so the same struct can be exercised against a fake backend in tests.
/// Stateful metrics (e.g. utilization, which needs the previous sample) may
/// hold interior-mutable state behind a `Mutex` while still taking `&self`.
/// Register instances via [`crate::hardware::registry::Registry`] to expose
/// them to the CLI and TUI without touching dispatch code.
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
    #[allow(dead_code)] // consumed once multi-category metrics and TUI tabs land
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

    /// Whether this parameter supports per-core writes (for series metrics
    /// where each core can be controlled independently). Defaults to `false`.
    fn is_core_writable(&self) -> bool {
        false
    }

    /// Write a value to a single core of a per-core series metric. Only called
    /// when [`is_core_writable`] returns `true`. Implementations should isolate
    /// failures: an error for one core must not prevent writes to other cores.
    fn write_core(&self, _source: &dyn Source, _core: usize, _value: &Value) -> Result<()> {
        anyhow::bail!("metric {} is not per-core writable", self.id())
    }
}
