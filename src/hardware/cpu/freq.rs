use crate::hardware::metric::{Metric, Value};
use crate::hardware::source::Source;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const SYS_CPU: &str = "/sys/devices/system/cpu";

/// Metric: current frequency of every CPU core (MHz).
///
/// Reads `scaling_cur_freq` for each `cpuN`. Missing/unreadable cores produce
/// `Value::Freq(NaN)` so that indexing stays stable across cores.
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
        let frequencies = current_frequencies_mhz_with(source)?;
        Ok(Value::Series(
            frequencies.into_iter().map(Value::Freq).collect(),
        ))
    }
}

/// Reads the current frequency of every CPU core via the provided [`Source`].
///
/// Returns the frequencies in MHz ordered by CPU index. Cores whose frequency
/// cannot be read yield `f64::NAN`.
pub fn current_frequencies_mhz() -> Result<Vec<f64>> {
    current_frequencies_mhz_with(&crate::hardware::source::SysfsSource::new())
}

fn current_frequencies_mhz_with(source: &dyn Source) -> Result<Vec<f64>> {
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

        let freq_path: PathBuf = path.join("cpufreq/scaling_cur_freq");
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

    /// In-memory `Source` for exercising parsing logic without sysfs.
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
        // cpu3 has no scaling_cur_freq -> NaN

        FakeSource { files, dirs }
    }

    #[test]
    fn reads_frequencies_in_index_order_with_nan_for_missing() {
        let src = fake_source();
        let freqs = current_frequencies_mhz_with(&src).unwrap();
        assert_eq!(freqs.len(), 3);
        assert_eq!(freqs[0], 2400.0);
        assert_eq!(freqs[1], 1800.0);
        assert!(freqs[2].is_nan());
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
    fn metric_metadata() {
        let m = CpuCurrentFreqMetric::new();
        assert_eq!(m.id(), "cpu.freq.cur");
        assert_eq!(m.unit(), "MHz");
        assert!(!m.is_writable());
    }
}
