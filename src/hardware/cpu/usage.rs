use crate::hardware::metric::{Metric, Value};
use crate::hardware::source::Source;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

const PROC_STAT: &str = "/proc/stat";

/// Metric: per-core CPU utilization (percent), derived from `/proc/stat`.
///
/// This is the first **stateful** metric: utilization is a ratio of deltas
/// between two samples, so the metric keeps the previous `(idle, total)` per
/// core in a `Mutex`. The instance is constructed once (via
/// [`crate::hardware::Registry::default_cpu`]) and persists across reads, so
/// the TUI's periodic refresh naturally yields real deltas after the first
/// sample. The first sample (no prior data) returns `NaN` per core, which the
/// UI renders as `—`.
pub struct CpuUtilizationMetric {
    prev: Mutex<HashMap<usize, (u64, u64)>>,
}

impl CpuUtilizationMetric {
    pub fn new() -> Self {
        Self {
            prev: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for CpuUtilizationMetric {
    fn default() -> Self {
        Self::new()
    }
}

impl Metric for CpuUtilizationMetric {
    fn id(&self) -> &str {
        "cpu.usage"
    }

    fn label(&self) -> &str {
        "Usage"
    }

    fn unit(&self) -> &str {
        "%"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        let content = source
            .read_to_string(Path::new(PROC_STAT))
            .with_context(|| format!("failed to read {}", PROC_STAT))?;
        let samples = parse_proc_stat(&content);

        let mut prev = self.prev.lock().expect("usage state mutex poisoned");
        let mut out: Vec<Value> = Vec::with_capacity(samples.len());

        for (core, idle, total) in &samples {
            let pct = match prev.get(core) {
                Some((prev_idle, prev_total)) => {
                    let dt = total.saturating_sub(*prev_total) as f64;
                    let di = idle.saturating_sub(*prev_idle) as f64;
                    if dt == 0.0 {
                        f64::NAN
                    } else {
                        ((dt - di) / dt) * 100.0
                    }
                }
                None => f64::NAN,
            };
            out.push(Value::Percent(pct));
            prev.insert(*core, (*idle, *total));
        }

        Ok(Value::Series(out))
    }
}

/// Parse `/proc/stat` into `(core, idle, total)` tuples, sorted by core.
///
/// `idle = idle + iowait`; `total` is the sum of the first eight fields
/// (user, nice, system, idle, iowait, irq, softirq, steal). The aggregate
/// `cpu ` line (no numeric suffix) is skipped.
fn parse_proc_stat(content: &str) -> Vec<(usize, u64, u64)> {
    let mut out: Vec<(usize, u64, u64)> = Vec::new();

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let Some(label) = parts.next() else {
            continue;
        };
        let Some(rest) = label.strip_prefix("cpu") else {
            continue;
        };
        let Ok(core) = rest.parse::<usize>() else {
            // Aggregate "cpu" line (no digits) -> skip.
            continue;
        };

        let fields: Vec<u64> = parts.filter_map(|s| s.parse::<u64>().ok()).collect();
        if fields.len() < 4 {
            continue;
        }
        let idle = fields[3] + fields.get(4).copied().unwrap_or(0);
        let total: u64 = fields.iter().take(8).sum();
        out.push((core, idle, total));
    }

    out.sort_by_key(|(core, _, _)| *core);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct FakeSource {
        files: HashMap<PathBuf, String>,
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
            self.files.contains_key(path)
        }
        fn list_dir(&self, _path: &Path) -> Result<Vec<PathBuf>> {
            Ok(Vec::new())
        }
    }

    fn src_with(content: &str) -> FakeSource {
        let mut files = HashMap::new();
        files.insert(PathBuf::from(PROC_STAT), content.to_string());
        FakeSource { files }
    }

    // Sample1: cpu0 total=100 idle=85; cpu1 total=200 idle=170
    const SAMPLE_A: &str = "cpu  100 0 100 1000 0 0 0 0\ncpu0 10 0 5 85 0 0 0 0\ncpu1 20 0 10 170 0 0 0 0\n";
    // Sample2: cpu0 total=200 idle=155 (dt=100, di=70 -> 30%);
    //          cpu1 total=240 idle=180 (dt=40, di=10  -> 75%)
    const SAMPLE_B: &str = "cpu  200 0 200 2000 0 0 0 0\ncpu0 30 0 15 155 0 0 0 0\ncpu1 40 0 20 180 0 0 0 0\n";

    #[test]
    fn first_sample_is_nan_per_core() {
        let src = src_with(SAMPLE_A);
        let m = CpuUtilizationMetric::new();
        let Value::Series(vs) = m.read(&src).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(vs.len(), 2);
        assert!(matches!(vs[0], Value::Percent(v) if v.is_nan()));
        assert!(matches!(vs[1], Value::Percent(v) if v.is_nan()));
    }

    #[test]
    fn second_sample_yields_delta_percent() {
        let m = CpuUtilizationMetric::new();
        // Seed state.
        let _ = m.read(&src_with(SAMPLE_A)).unwrap();
        let Value::Series(vs) = m.read(&src_with(SAMPLE_B)).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(vs.len(), 2);
        match vs[0] {
            Value::Percent(v) => assert!((v - 30.0).abs() < 1e-6),
            _ => panic!("expected Percent"),
        }
        match vs[1] {
            Value::Percent(v) => assert!((v - 75.0).abs() < 1e-6),
            _ => panic!("expected Percent"),
        }
    }

    #[test]
    fn aggregate_cpu_line_is_skipped() {
        let src = src_with("cpu  1 2 3 4 5 6 7 8\ncpu0 1 2 3 4 5 6 7 8\n");
        let m = CpuUtilizationMetric::new();
        let Value::Series(vs) = m.read(&src).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(vs.len(), 1, "aggregate cpu line must not be a core");
    }

    #[test]
    fn parse_handles_iowait_and_eight_field_total() {
        let parsed = parse_proc_stat("cpu0 1 2 3 4 5 6 7 8\n");
        assert_eq!(parsed, vec![(0, 4 + 5, 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8)]);
    }

    #[test]
    fn metric_metadata() {
        let m = CpuUtilizationMetric::new();
        assert_eq!(m.id(), "cpu.usage");
        assert_eq!(m.unit(), "%");
        assert!(!m.is_writable());
    }
}
