use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hare")]
#[command(about = "A lightweight hardware information CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// CPU metrics and controls.
    Cpu(CpuArgs),
    /// Launch the interactive TUI monitor.
    Monitor,
}

#[derive(Args)]
pub struct CpuArgs {
    #[command(subcommand)]
    pub command: CpuCommand,
}

#[derive(Subcommand)]
pub enum CpuCommand {
    /// CPU frequency metrics.
    Freq(FreqArgs),
    /// CPU scaling governor.
    Governor(GovernorArgs),
    /// CPU topology / online status.
    Topology,
}

#[derive(Args)]
pub struct FreqArgs {
    #[command(subcommand)]
    pub metric: Option<FreqMetric>,
}

#[derive(Subcommand)]
pub enum FreqMetric {
    /// Current frequency per core.
    Cur,
    /// Minimum scaling frequency per core.
    Min,
    /// Maximum scaling frequency per core.
    Max,
}

#[derive(Args)]
pub struct GovernorArgs {
    #[command(subcommand)]
    pub action: GovernorAction,
}

#[derive(Subcommand)]
pub enum GovernorAction {
    /// Show the current governor per core.
    Get,
    /// Set the governor on every core.
    Set { governor: String },
    /// List available governors.
    List,
}
