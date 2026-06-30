pub mod freq;

use crate::hardware::metric::Metric;
use crate::hardware::HardwareInfo;
use anyhow::Result;

pub struct CpuInfo;

impl HardwareInfo for CpuInfo {
    fn run() -> Result<()> {
        let frequencies = freq::current_frequencies_mhz()?;

        for (index, mhz) in frequencies.iter().enumerate() {
            if mhz.is_nan() {
                println!("CPU {}: NaN", index);
            } else {
                println!("CPU {}: {:.2} MHz", index, mhz);
            }
        }

        Ok(())
    }
}

/// Convenience function used by the command dispatcher.
pub fn run() -> Result<()> {
    CpuInfo::run()
}

/// Instantiate the default set of CPU metrics.
///
/// Each new CPU parameter should be constructed here (or in its own submodule)
/// and returned from this function so [`crate::hardware::Registry::default_cpu`]
/// picks it up automatically.
pub fn default_metrics() -> Vec<Box<dyn Metric>> {
    vec![Box::new(freq::CpuCurrentFreqMetric::new())]
}
