use crate::hardware::metric::{Metric, Value};
use crate::hardware::source::Source;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const SYS_CPU: &str = "/sys/devices/system/cpu";

/// List `cpuN` entries under `/sys/devices/system/cpu` as `(core_index, path)`,
/// sorted by index. Non-`cpuN` entries (e.g. `cpufreq`, `cpuidle`, the
/// aggregate `cpu` line in /proc/stat is unrelated) are skipped.
fn cpu_core_paths(source: &dyn Source) -> Result<Vec<(usize, PathBuf)>> {
    let entries = source.list_dir(Path::new(SYS_CPU))?;
    let mut cores: Vec<(usize, PathBuf)> = Vec::new();

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
        cores.push((index, path));
    }

    cores.sort_by_key(|(index, _)| *index);
    Ok(cores)
}

/// Metric: online status of every CPU core.
///
/// Cores without an `online` file (e.g. cpu0 on some kernels) default to
/// `true` (online).
pub struct CpuOnlineMetric;

impl CpuOnlineMetric {
    pub fn new() -> Self {
        Self
    }
}

impl Metric for CpuOnlineMetric {
    fn id(&self) -> &str {
        "cpu.topology.online"
    }

    fn label(&self) -> &str {
        "Online Status"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        let cores = cpu_core_paths(source)?;
        let values = cores
            .into_iter()
            .map(|(index, path)| {
                let online_path: PathBuf = path.join("online");
                let online = if source.exists(&online_path) {
                    source
                        .read_to_string(&online_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u8>().ok())
                        .map(|n| n != 0)
                        .unwrap_or(true)
                } else {
                    // No online file -> core is always online (e.g. cpu0).
                    true
                };
                (index, online)
            })
            .map(|(_, b)| Value::Bool(b))
            .collect();
        Ok(Value::Series(values))
    }

    fn is_core_writable(&self) -> bool {
        true
    }

    fn write_core(&self, source: &dyn Source, core: usize, value: &Value) -> Result<()> {
        let online = match value {
            Value::Bool(b) => *b,
            _ => anyhow::bail!("expected Bool for online status, got {:?}", value),
        };
        let path = Path::new(SYS_CPU)
            .join(format!("cpu{}", core))
            .join("online");
        if !source.exists(&path) {
            anyhow::bail!("core {} cannot be onlined/offlined (no online file)", core);
        }
        source
            .write(&path, if online { "1" } else { "0" })
            .with_context(|| format!("failed to set online status for core {}", core))?;
        Ok(())
    }
}

/// Whether a given core exposes an `online` file (i.e. can be toggled). Boot
/// cores like cpu0 typically have no `online` file and are always online.
pub fn core_has_online_file(source: &dyn Source, core: usize) -> bool {
    let path = Path::new(SYS_CPU)
        .join(format!("cpu{}", core))
        .join("online");
    source.exists(&path)
}

/// Metric: cluster id of every CPU core, from `topology/cluster_id`.
///
/// Cores whose `cluster_id` file is missing (kernels without cluster
/// topology support) yield `"-"` for that slot so the index ordering stays
/// stable across cores.
pub struct CpuClusterMetric;

impl CpuClusterMetric {
    pub fn new() -> Self {
        Self
    }
}

impl Metric for CpuClusterMetric {
    fn id(&self) -> &str {
        "cpu.topology.cluster"
    }

    fn label(&self) -> &str {
        "Cluster"
    }

    fn read(&self, source: &dyn Source) -> Result<Value> {
        let cores = cpu_core_paths(source)?;
        let values: Vec<Value> = cores
            .into_iter()
            .map(|(_, path)| {
                let cluster_path: PathBuf = path.join("topology/cluster_id");
                let cluster = source
                    .read_to_string(&cluster_path)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "-".to_string());
                Value::Raw(cluster)
            })
            .collect();
        Ok(Value::Series(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    use std::sync::Mutex;

    struct FakeSource {
        files: HashMap<PathBuf, String>,
        dirs: HashMap<PathBuf, Vec<PathBuf>>,
        writes: Mutex<Vec<(PathBuf, String)>>,
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

    fn fake_source(with_cluster: bool) -> FakeSource {
        let root = PathBuf::from(SYS_CPU);
        let cpu0 = root.join("cpu0");
        let cpu1 = root.join("cpu1");
        let cpu3 = root.join("cpu3");
        let mut dirs = HashMap::new();
        dirs.insert(root, vec![cpu0.clone(), cpu1.clone(), cpu3.clone()]);
        let mut files = HashMap::new();
        files.insert(cpu0.join("online"), "1\n".to_string());
        files.insert(cpu1.join("online"), "0\n".to_string());
        // cpu3 has no online file -> defaults to online.
        if with_cluster {
            files.insert(cpu0.join("topology/cluster_id"), "0\n".to_string());
            files.insert(cpu1.join("topology/cluster_id"), "4\n".to_string());
            // cpu3 lacks cluster_id -> "-".
        }
        FakeSource {
            files,
            dirs,
            writes: Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn online_reads_per_core_with_default_for_missing() {
        let src = fake_source(false);
        let Value::Series(vs) = CpuOnlineMetric::new().read(&src).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(vs, vec![Value::Bool(true), Value::Bool(false), Value::Bool(true)]);
    }

    #[test]
    fn cluster_reads_per_core_with_dash_for_missing() {
        let src = fake_source(true);
        let Value::Series(vs) = CpuClusterMetric::new().read(&src).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(
            vs,
            vec![
                Value::Raw("0".to_string()),
                Value::Raw("4".to_string()),
                Value::Raw("-".to_string()),
            ]
        );
    }

    #[test]
    fn cluster_all_missing_when_unsupported() {
        // No topology/cluster_id files at all.
        let src = fake_source(false);
        let Value::Series(vs) = CpuClusterMetric::new().read(&src).unwrap() else {
            panic!("expected Series");
        };
        assert_eq!(vs.len(), 3);
        assert!(vs.iter().all(|v| matches!(v, Value::Raw(s) if s == "-")));
    }

    #[test]
    fn cluster_metadata() {
        let m = CpuClusterMetric::new();
        assert_eq!(m.id(), "cpu.topology.cluster");
        assert!(!m.is_writable());
    }

    #[test]
    fn online_write_core_writes_one_or_zero_and_skips_locked() {
        let src = fake_source(true); // cpu0/cpu1 have online, cpu3 does not
        let m = CpuOnlineMetric::new();

        // cpu1 present -> writes "0" to its online file.
        m.write_core(&src, 1, &Value::Bool(false)).unwrap();
        assert_eq!(
            src.writes.lock().unwrap().iter().filter(|(_, v)| v == "0").count(),
            1
        );

        // cpu3 (no online file) -> error, nothing written for it.
        let err = m.write_core(&src, 3, &Value::Bool(false));
        assert!(err.is_err());
    }

    #[test]
    fn online_write_core_rejects_non_bool() {
        let src = fake_source(true);
        let m = CpuOnlineMetric::new();
        assert!(m.write_core(&src, 0, &Value::Raw("1".into())).is_err());
    }

    #[test]
    fn core_has_online_file_reflects_presence() {
        let src = fake_source(true); // cpu0/cpu1 have online, cpu3 does not
        assert!(core_has_online_file(&src, 0));
        assert!(core_has_online_file(&src, 1));
        assert!(!core_has_online_file(&src, 3));
    }

    #[test]
    fn online_is_core_writable() {
        assert!(CpuOnlineMetric::new().is_core_writable());
        assert!(!CpuClusterMetric::new().is_core_writable());
    }
}
