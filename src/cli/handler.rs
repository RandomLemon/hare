use crate::cli::args::{CpuArgs, CpuCommand, Commands, FreqMetric, GovernorAction};
use crate::hardware::{Registry, SysfsSource, Value};
use anyhow::{Result, anyhow};

/// Dispatch a parsed top-level command.
pub fn dispatch(command: Commands) -> Result<()> {
    match command {
        Commands::Monitor => crate::tui::run(),
        Commands::Cpu(cpu) => dispatch_cpu(cpu),
    }
}

fn dispatch_cpu(cpu: CpuArgs) -> Result<()> {
    let registry = Registry::default_cpu();
    let source = SysfsSource::new();

    match cpu.command {
        CpuCommand::Freq(freq) => {
            let id = match freq.metric.unwrap_or(FreqMetric::Cur) {
                FreqMetric::Cur => "cpu.freq.cur",
                FreqMetric::Min => "cpu.freq.min",
                FreqMetric::Max => "cpu.freq.max",
            };
            print_metric(&registry, &source, id)
        }
        CpuCommand::Governor(gov) => match gov.action {
            GovernorAction::Get => print_metric(&registry, &source, "cpu.governor"),
            GovernorAction::List => print_metric(&registry, &source, "cpu.governor.available"),
            GovernorAction::Set { governor } => {
                write_metric(&registry, &source, "cpu.governor", Value::Enum(governor))
            }
        },
        CpuCommand::Topology => print_metric(&registry, &source, "cpu.topology.online"),
    }
}

fn print_metric(registry: &Registry, source: &SysfsSource, id: &str) -> Result<()> {
    let metric = registry
        .iter()
        .find(|m| m.id() == id)
        .ok_or_else(|| anyhow!("no metric registered with id `{}`", id))?;

    let value = metric.read(source)?;
    println!("{} ({})", metric.label(), metric.id());
    for line in value.lines() {
        println!("  {}", line);
    }
    Ok(())
}

fn write_metric(
    registry: &Registry,
    source: &SysfsSource,
    id: &str,
    value: Value,
) -> Result<()> {
    let metric = registry
        .iter()
        .find(|m| m.id() == id)
        .ok_or_else(|| anyhow!("no metric registered with id `{}`", id))?;

    if !metric.is_writable() {
        return Err(anyhow!("metric `{}` is read-only", id));
    }
    metric.write(source, &value)?;
    println!("set {} = {}", id, value.format());
    Ok(())
}
