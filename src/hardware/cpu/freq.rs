use crate::hardware::metric::{Metric, Value};
use crate::hardware::source::Source;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const SYS_CPU: &str = "/sys/devices/system/cpu";

/// Metric: current frequency of every CPU core (MHz).
pub struct CpuCurrentFreqMetric;

impl CpuCurrentFreqMetric {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CpuCurrentFreqMetric {
    fn default() -> Self {
        Self::new()
    }
}

impl Metric for CpuCurrentFreqMetric {
    fn id(&self) -> &str {
        "cpu.freq.cur"
    }

    fn label(&self) -> &str {
        "Current Frequency"
    }

    fn unit(&self) -> &str {
        "MHz"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        Ok(Value::Series(
            per_core_mhz(source, "cpufreq/scaling_cur_freq")?
                .into_iter()
                .map(Value::Freq)
                .collect(),
        ))
    }
}

/// Metric: minimum scaling frequency of every CPU core (MHz).
pub struct CpuMinFreqMetric;

impl CpuMinFreqMetric {
    pub fn new() -> Self {
        Self
    }
}

impl Metric for CpuMinFreqMetric {
    fn id(&self) -> &str {
        "cpu.freq.min"
    }

    fn label(&self) -> &str {
        "Minimum Frequency"
    }

    fn unit(&self) -> &str {
        "MHz"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        Ok(Value::Series(
            per_core_mhz(source, "cpufreq/scaling_min_freq")?
                .into_iter()
                .map(Value::Freq)
                .collect(),
        ))
    }
}

/// Metric: maximum scaling frequency of every CPU core (MHz).
pub struct CpuMaxFreqMetric;

impl CpuMaxFreqMetric {
    pub fn new() -> Self {
        Self
    }
}

impl Metric for CpuMaxFreqMetric {
    fn id(&self) -> &str {
        "cpu.freq.max"
    }

    fn label(&self) -> &str {
        "Maximum Frequency"
    }

    fn unit(&self) -> &str {
        "MHz"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        Ok(Value::Series(
            per_core_mhz(source, "cpufreq/scaling_max_freq")?
                .into_iter()
                .map(Value::Freq)
                .collect(),
        ))
    }
}

fn per_core_mhz(source: &dyn Source, rel: &str) -> Result<Vec<f64>> {
    let entries = source.list_dir(Path::new(SYS_CPU))?;

    let mut frequencies: Vec<(usize, f64)> = Vec::new();

    for path in entries {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(index_str) = name.strip_prefix("cpu") else {
            continue;
        };
        let Ok(index) = index_str.parse::<usize>() else {
            continue;
        };

        let freq_path: PathBuf = path.join(rel);
        let mhz = read_freq_mhz(source, &freq_path).unwrap_or(f64::NAN);
        frequencies.push((index, mhz));
    }

    frequencies.sort_by_key(|(index, _)| *index);
    Ok(frequencies.into_iter().map(|(_, mhz)| mhz).collect())
}

fn read_freq_mhz(source: &dyn Source, path: &Path) -> Result<f64> {
    let content = source.read_to_string(path)?;
    let khz: f64 = content
        .trim()
        .parse()
        .with_context(|| format!("failed to parse frequency value in {}", path.display()))?;
    Ok(khz / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct FakeSource {
        files: HashMap<PathBuf, String>,
        dirs: HashMap<PathBuf, Vec<PathBuf>>,
    }

    impl Source for FakeSource {
        fn read_to_string(&self, path: &Path) -> Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not found: {}", path.display()))
        }

        fn write(&self, _path: &Path, _content: &str) -> Result<()> {
            unimplemented!()
        }

        fn exists(&self, path: &Path) -> bool {
            self.files.contains_key(path) || self.dirs.contains_key(path)
        }

        fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
            self.dirs
                .get(path)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not a dir: {}", path.display()))
        }
    }

    fn fake_source() -> FakeSource {
        let cpu_root = PathBuf::from(SYS_CPU);
        let cpu0 = cpu_root.join("cpu0");
        let cpu1 = cpu_root.join("cpu1");
        let cpu3 = cpu_root.join("cpu3");

        let mut dirs = HashMap::new();
        dirs.insert(
            cpu_root.clone(),
            vec![cpu0.clone(), cpu1.clone(), cpu3.clone()],
        );

        let mut files = HashMap::new();
        files.insert(cpu0.join("cpufreq/scaling_cur_freq"), "2400000\n".to_string());
        files.insert(cpu1.join("cpufreq/scaling_cur_freq"), "1800000\n".to_string());
        files.insert(cpu0.join("cpufreq/scaling_min_freq"), "800000\n".to_string());
        files.insert(cpu1.join("cpufreq/scaling_min_freq"), "800000\n".to_string());
        files.insert(cpu0.join("cpufreq/scaling_max_freq"), "3500000\n".to_string());
        files.insert(cpu1.join("cpufreq/scaling_max_freq"), "3500000\n".to_string());

        FakeSource { files, dirs }
    }

    #[test]
    fn reads_cur_frequencies_in_index_order_with_nan_for_missing() {
        let src = fake_source();
        let Value::Series(values) = CpuCurrentFreqMetric::new().read(&src).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(values.len(), 3);
        assert!(matches!(values[0], Value::Freq(v) if (v - 2400.0).abs() < 1e-6));
        assert!(matches!(values[1], Value::Freq(v) if (v - 1800.0).abs() < 1e-6));
        assert!(matches!(values[2], Value::Freq(v) if v.is_nan()));
    }

    #[test]
    fn metric_read_returns_series() {
        let src = fake_source();
        let m = CpuCurrentFreqMetric::new();
        let Value::Series(values) = m.read(&src).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(values.len(), 3);
        assert!(matches!(values[2], Value::Freq(v) if v.is_nan()));
    }

    #[test]
    fn min_max_metrics_read_expected_values() {
        let src = fake_source();
        let min = CpuMinFreqMetric::new().read(&src).unwrap();
        let max = CpuMaxFreqMetric::new().read(&src).unwrap();
        let Value::Series(mins) = min else { panic!("min series") };
        let Value::Series(maxs) = max else { panic!("max series") };
        assert_eq!(mins[0], Value::Freq(800.0));
        assert_eq!(maxs[0], Value::Freq(3500.0));
    }

    #[test]
    fn metric_metadata() {
        let m = CpuCurrentFreqMetric::new();
        assert_eq!(m.id(), "cpu.freq.cur");
        assert_eq!(m.unit(), "MHz");
        assert!(!m.is_writable());
    }
}
