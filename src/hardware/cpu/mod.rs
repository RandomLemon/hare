pub mod freq;
pub mod governor;
pub mod topology;

use crate::hardware::metric::Metric;

/// Instantiate the default set of CPU metrics.
///
/// Each new CPU parameter should be constructed here (or in its own submodule)
/// and returned from this function so [`crate::hardware::Registry::default_cpu`]
/// picks it up automatically.
pub fn default_metrics() -> Vec<Box<dyn Metric>> {
    vec![
        Box::new(freq::CpuCurrentFreqMetric::new()),
        Box::new(freq::CpuMinFreqMetric::new()),
        Box::new(freq::CpuMaxFreqMetric::new()),
        Box::new(governor::CpuGovernorMetric::new()),
        Box::new(governor::CpuAvailableGovernorsMetric::new()),
        Box::new(topology::CpuOnlineMetric::new()),
    ]
}
