use crate::hardware::metric::{Metric, Value};
use crate::hardware::source::Source;
use anyhow::Result;
use std::path::{Path, PathBuf};

const SYS_CPU: &str = "/sys/devices/system/cpu";

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
        let entries = source.list_dir(Path::new(SYS_CPU))?;
        let mut values: Vec<(usize, bool)> = Vec::new();

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
            values.push((index, online));
        }

        values.sort_by_key(|(index, _)| *index);
        Ok(Value::Series(
            values
                .into_iter()
                .map(|(_, b)| Value::Bool(b))
                .collect(),
        ))
    }
}
