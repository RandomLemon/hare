pub mod cpu;
pub mod metric;
pub mod registry;
pub mod source;

pub use metric::{Metric, Value};
pub use registry::Registry;
pub use source::{Source, SysfsSource};

use anyhow::Result;

/// Legacy trait retained for the existing CLI dispatcher.
///
/// New code should implement [`Metric`] and register it with [`Registry`]
/// instead of `HardwareInfo`.
pub trait HardwareInfo {
    /// Run the module's default action.
    fn run() -> Result<()>;
}
