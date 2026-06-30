use crate::hardware::metric::{Metric, Value};
use crate::hardware::source::Source;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const SYS_CPU: &str = "/sys/devices/system/cpu";

/// Metric: scaling governor of every CPU core.
///
/// Writable: writing an `Value::Enum(gov)` sets the governor on every core.
pub struct CpuGovernorMetric;

impl CpuGovernorMetric {
    pub fn new() -> Self {
        Self
    }
}

impl Metric for CpuGovernorMetric {
    fn id(&self) -> &str {
        "cpu.governor"
    }

    fn label(&self) -> &str {
        "Scaling Governor"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        Ok(Value::Series(
            per_core_string(source, "cpufreq/scaling_governor")?
                .into_iter()
                .map(Value::Enum)
                .collect(),
        ))
    }

    fn is_writable(&self) -> bool {
        true
    }

    fn write(&self, source: &dyn Source, value: &Value) -> Result<()> {
        let gov = match value {
            Value::Enum(s) | Value::Raw(s) => s.trim().to_string(),
            _ => anyhow::bail!("expected governor name (Enum/Raw), got {:?}", value),
        };
        if gov.is_empty() {
            anyhow::bail!("governor name must not be empty");
        }

        let entries = source.list_dir(Path::new(SYS_CPU))?;
        for path in entries {
            if !is_cpu_dir(&path) {
                continue;
            }
            let gov_path: PathBuf = path.join("cpufreq/scaling_governor");
            if source.exists(&gov_path) {
                source
                    .write(&gov_path, &gov)
                    .with_context(|| format!("failed to set governor on {}", path.display()))?;
            }
        }
        Ok(())
    }
}

/// Metric: governors available on the system (read from the first core).
pub struct CpuAvailableGovernorsMetric;

impl CpuAvailableGovernorsMetric {
    pub fn new() -> Self {
        Self
    }
}

impl Metric for CpuAvailableGovernorsMetric {
    fn id(&self) -> &str {
        "cpu.governor.available"
    }

    fn label(&self) -> &str {
        "Available Governors"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        let entries = source.list_dir(Path::new(SYS_CPU))?;
        for path in entries {
            if !is_cpu_dir(&path) {
                continue;
            }
            let gov_path: PathBuf = path.join("cpufreq/scaling_available_governors");
            if source.exists(&gov_path) {
                let content = source.read_to_string(&gov_path)?;
                let govs: Vec<Value> = content
                    .split_whitespace()
                    .map(|s| Value::Enum(s.to_string()))
                    .collect();
                return Ok(Value::Series(govs));
            }
        }
        Ok(Value::Series(Vec::new()))
    }
}

fn per_core_string(source: &dyn Source, rel: &str) -> Result<Vec<String>> {
    let entries = source.list_dir(Path::new(SYS_CPU))?;
    let mut values: Vec<(usize, String)> = Vec::new();

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
        let file_path: PathBuf = path.join(rel);
        let v = source.read_to_string(&file_path)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        values.push((index, v));
    }

    values.sort_by_key(|(index, _)| *index);
    Ok(values.into_iter().map(|(_, v)| v).collect())
}

fn is_cpu_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("cpu"))
        .map(|s| s.parse::<usize>().is_ok())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct FakeSource {
        files: HashMap<PathBuf, String>,
        dirs: HashMap<PathBuf, Vec<PathBuf>>,
        writes: std::sync::Mutex<Vec<(PathBuf, String)>>,
    }

    impl Source for FakeSource {
        fn read_to_string(&self, path: &Path) -> Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("not found: {}", path.display()))
        }
        fn write(&self, path: &Path, content: &str) -> Result<()> {
            self.writes
                .lock()
                .unwrap()
                .push((path.to_path_buf(), content.to_string()));
            Ok(())
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
        let root = PathBuf::from(SYS_CPU);
        let cpu0 = root.join("cpu0");
        let cpu1 = root.join("cpu1");
        let mut dirs = HashMap::new();
        dirs.insert(root, vec![cpu0.clone(), cpu1.clone()]);
        let mut files = HashMap::new();
        files.insert(
            cpu0.join("cpufreq/scaling_governor"),
            "powersave\n".to_string(),
        );
        files.insert(
            cpu1.join("cpufreq/scaling_governor"),
            "performance\n".to_string(),
        );
        files.insert(
            cpu0.join("cpufreq/scaling_available_governors"),
            "performance powersave\n".to_string(),
        );
        FakeSource {
            files,
            dirs,
            writes: std::sync::Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn reads_per_core_governors() {
        let src = fake_source();
        let v = CpuGovernorMetric::new().read(&src).unwrap();
        let Value::Series(vs) = v else { panic!("series") };
        assert_eq!(vs, vec![Value::Enum("powersave".into()), Value::Enum("performance".into())]);
    }

    #[test]
    fn available_governors_split_into_series() {
        let src = fake_source();
        let v = CpuAvailableGovernorsMetric::new().read(&src).unwrap();
        let Value::Series(vs) = v else { panic!("series") };
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn write_sets_governor_on_all_cores() {
        let src = fake_source();
        CpuGovernorMetric::new()
            .write(&src, &Value::Enum("powersave".into()))
            .unwrap();
        let writes = src.writes.lock().unwrap();
        assert_eq!(writes.len(), 2);
        assert!(writes.iter().all(|(_, v)| v == "powersave"));
    }

    #[test]
    fn is_writable_flag() {
        assert!(CpuGovernorMetric::new().is_writable());
        assert!(!CpuAvailableGovernorsMetric::new().is_writable());
    }
}
